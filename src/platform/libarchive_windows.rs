#![allow(unsafe_code)]

use std::ffi::{CStr, CString, OsStr, c_char, c_int, c_void};
use std::fs;
use std::io::{self, Write};
use std::mem::{size_of, transmute_copy};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;

use crate::backend::{
    MAX_BACKEND_MANIFEST_BYTES, MAX_BACKEND_MANIFEST_FILES, validate_manifest_path,
};
use crate::config::FilenameEncoding;
use crate::error::{IrohaZipError, Result};

use super::windows_impl::{
    require_current_process_appcontainer, validate_directory_security,
    validate_regular_file_security,
};

const ARCHIVE_OK: c_int = 0;
const ARCHIVE_EOF: c_int = 1;
const ARCHIVE_WARN: c_int = -20;
const LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR: u32 = 0x0000_0100;
const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;
const MAX_INTERNAL_LISTING_BYTES: u64 = 64 * 1024 * 1024;
const MAX_INTERNAL_PATH_BYTES: usize = 1024 * 1024;
const MAX_INTERNAL_ENTRIES: u64 = 1_000_000;

type ArchiveReadNew = unsafe extern "C" fn() -> *mut c_void;
type ArchiveReadSupport = unsafe extern "C" fn(*mut c_void) -> c_int;
type ArchiveReadSetOptions = unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int;
type ArchiveReadOpenFilenameW = unsafe extern "C" fn(*mut c_void, *const u16, usize) -> c_int;
type ArchiveReadNextHeader = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> c_int;
type ArchiveEntryPathnameUtf8 = unsafe extern "C" fn(*mut c_void) -> *const c_char;
type ArchiveReadDataSkip = unsafe extern "C" fn(*mut c_void) -> c_int;
type ArchiveReadClose = unsafe extern "C" fn(*mut c_void) -> c_int;
type ArchiveReadFree = unsafe extern "C" fn(*mut c_void) -> c_int;

#[link(name = "kernel32")]
unsafe extern "system" {
    #[link_name = "LoadLibraryExW"]
    fn load_library_ex_w(path: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
    #[link_name = "GetProcAddress"]
    fn get_proc_address(module: *mut c_void, name: *const u8) -> *mut c_void;
    #[link_name = "FreeLibrary"]
    fn free_library(module: *mut c_void) -> i32;
}

struct ArchiveApi {
    module: *mut c_void,
    read_new: ArchiveReadNew,
    support_filter_all: ArchiveReadSupport,
    support_format_all: ArchiveReadSupport,
    set_options: ArchiveReadSetOptions,
    open_filename_w: ArchiveReadOpenFilenameW,
    next_header: ArchiveReadNextHeader,
    pathname_utf8: ArchiveEntryPathnameUtf8,
    data_skip: ArchiveReadDataSkip,
    close: ArchiveReadClose,
    free: ArchiveReadFree,
}

impl Drop for ArchiveApi {
    fn drop(&mut self) {
        if !self.module.is_null() {
            let _ = unsafe { free_library(self.module) };
        }
    }
}

impl ArchiveApi {
    fn load(candidates: &[PathBuf]) -> Result<Self> {
        for candidate in candidates {
            let wide = wide_null(candidate.as_os_str());
            let module = unsafe {
                load_library_ex_w(
                    wide.as_ptr(),
                    ptr::null_mut(),
                    LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
                )
            };
            if module.is_null() {
                continue;
            }
            if unsafe { get_proc_address(module, c"archive_read_new".as_ptr().cast()) }.is_null() {
                let _ = unsafe { free_library(module) };
                continue;
            }
            let loaded = unsafe { Self::from_module(module) };
            match loaded {
                Ok(api) => return Ok(api),
                Err(_) => {
                    let _ = unsafe { free_library(module) };
                }
            }
        }
        Err(IrohaZipError::Backend(
            "no verified backend DLL exports the required libarchive UTF-8 listing API".to_owned(),
        ))
    }

    unsafe fn from_module(module: *mut c_void) -> Result<Self> {
        Ok(Self {
            module,
            read_new: unsafe { required_symbol(module, c"archive_read_new")? },
            support_filter_all: unsafe {
                required_symbol(module, c"archive_read_support_filter_all")?
            },
            support_format_all: unsafe {
                required_symbol(module, c"archive_read_support_format_all")?
            },
            set_options: unsafe { required_symbol(module, c"archive_read_set_options")? },
            open_filename_w: unsafe { required_symbol(module, c"archive_read_open_filename_w")? },
            next_header: unsafe { required_symbol(module, c"archive_read_next_header")? },
            pathname_utf8: unsafe { required_symbol(module, c"archive_entry_pathname_utf8")? },
            data_skip: unsafe { required_symbol(module, c"archive_read_data_skip")? },
            close: unsafe { required_symbol(module, c"archive_read_close")? },
            free: unsafe { required_symbol(module, c"archive_read_free")? },
        })
    }
}

pub fn write_utf8_archive_listing(
    backend_root: &Path,
    candidate_file: &Path,
    archive_path: &Path,
    encoding: FilenameEncoding,
    max_entries: u64,
    max_path_bytes: usize,
    allow_unsandboxed: bool,
) -> Result<()> {
    require_current_process_appcontainer(allow_unsandboxed)?;
    let candidates = read_candidates(backend_root, candidate_file)?;
    validate_regular_file_security(archive_path)?;
    let api = ArchiveApi::load(&candidates)?;
    let archive = unsafe { (api.read_new)() };
    if archive.is_null() {
        return Err(IrohaZipError::Backend(
            "libarchive could not allocate an archive reader".to_owned(),
        ));
    }

    let operation = list_archive(
        &api,
        archive,
        archive_path,
        encoding,
        max_entries.min(MAX_INTERNAL_ENTRIES),
        max_path_bytes.min(MAX_INTERNAL_PATH_BYTES),
    );
    let close_status = unsafe { (api.close)(archive) };
    let free_status = unsafe { (api.free)(archive) };
    operation?;
    if close_status < ARCHIVE_WARN || free_status < ARCHIVE_WARN {
        return Err(IrohaZipError::Backend(format!(
            "libarchive listing cleanup failed: close={close_status}, free={free_status}"
        )));
    }
    Ok(())
}

fn list_archive(
    api: &ArchiveApi,
    archive: *mut c_void,
    archive_path: &Path,
    encoding: FilenameEncoding,
    max_entries: u64,
    max_path_bytes: usize,
) -> Result<()> {
    require_archive_status(
        unsafe { (api.support_filter_all)(archive) },
        "enable archive filters",
    )?;
    require_archive_status(
        unsafe { (api.support_format_all)(archive) },
        "enable archive formats",
    )?;
    if let Some(option) = encoding.bsdtar_option() {
        let option = CString::new(option).map_err(|_| {
            IrohaZipError::Backend("archive encoding option contains NUL".to_owned())
        })?;
        require_archive_status(
            unsafe { (api.set_options)(archive, option.as_ptr()) },
            "set archive filename encoding",
        )?;
    }

    let archive_wide = wide_null(archive_path.as_os_str());
    let open_status = unsafe { (api.open_filename_w)(archive, archive_wide.as_ptr(), 10240) };
    if open_status != ARCHIVE_OK {
        return Err(IrohaZipError::Backend(format!(
            "libarchive could not open the sandbox archive: status {open_status}"
        )));
    }

    let mut output = io::stdout().lock();
    let mut entry_count = 0u64;
    let mut output_bytes = 0u64;
    loop {
        let mut entry = ptr::null_mut();
        let status = unsafe { (api.next_header)(archive, &raw mut entry) };
        if status == ARCHIVE_EOF {
            break;
        }
        if status != ARCHIVE_OK || entry.is_null() {
            return Err(IrohaZipError::Backend(format!(
                "libarchive could not read an archive header: status {status}"
            )));
        }
        entry_count = entry_count
            .checked_add(1)
            .ok_or_else(|| IrohaZipError::Policy("archive member count overflow".to_owned()))?;
        if entry_count > max_entries {
            return Err(IrohaZipError::Policy(format!(
                "archive member count exceeds {max_entries}"
            )));
        }

        let name = unsafe { bounded_c_bytes((api.pathname_utf8)(entry), max_path_bytes) }?;
        if name.is_empty() {
            return Err(IrohaZipError::Policy(
                "archive member has an empty UTF-8 pathname".to_owned(),
            ));
        }
        if std::str::from_utf8(&name).is_err() {
            return Err(IrohaZipError::Policy(
                "libarchive returned a pathname that is not valid UTF-8".to_owned(),
            ));
        }
        if name.iter().any(|byte| matches!(*byte, b'\n' | b'\r')) {
            return Err(IrohaZipError::Policy(
                "archive member pathname contains a line break".to_owned(),
            ));
        }
        output_bytes = output_bytes
            .checked_add(u64::try_from(name.len()).unwrap_or(u64::MAX))
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| IrohaZipError::Policy("archive listing size overflow".to_owned()))?;
        if output_bytes > MAX_INTERNAL_LISTING_BYTES {
            return Err(IrohaZipError::Policy(format!(
                "archive member listing exceeds {MAX_INTERNAL_LISTING_BYTES} bytes"
            )));
        }
        output
            .write_all(&name)
            .and_then(|()| output.write_all(b"\n"))
            .map_err(|error| IrohaZipError::io("cannot write UTF-8 archive listing", error))?;

        let skip_status = unsafe { (api.data_skip)(archive) };
        if skip_status != ARCHIVE_OK {
            return Err(IrohaZipError::Backend(format!(
                "libarchive could not skip archive member data: status {skip_status}"
            )));
        }
    }
    output
        .flush()
        .map_err(|error| IrohaZipError::io("cannot flush UTF-8 archive listing", error))
}

fn read_candidates(backend_root: &Path, candidate_file: &Path) -> Result<Vec<PathBuf>> {
    validate_directory_security(backend_root)?;
    validate_regular_file_security(candidate_file)?;
    let backend_root = fs::canonicalize(backend_root).map_err(|error| {
        IrohaZipError::io_path(
            "cannot resolve sandbox backend directory",
            backend_root,
            error,
        )
    })?;
    let bytes = fs::read(candidate_file).map_err(|error| {
        IrohaZipError::io_path(
            "cannot read libarchive candidate list",
            candidate_file,
            error,
        )
    })?;
    if bytes.len() > MAX_BACKEND_MANIFEST_BYTES {
        return Err(IrohaZipError::Backend(
            "libarchive candidate list exceeds the backend manifest byte limit".to_owned(),
        ));
    }
    let text = std::str::from_utf8(&bytes).map_err(|error| {
        IrohaZipError::Backend(format!("libarchive candidate list is not UTF-8: {error}"))
    })?;
    let mut candidates = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        if candidates.len() >= MAX_BACKEND_MANIFEST_FILES {
            return Err(IrohaZipError::Backend(
                "libarchive candidate count exceeds the backend manifest limit".to_owned(),
            ));
        }
        let relative = validate_manifest_path(line)?;
        let is_dll = relative
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"));
        if !is_dll {
            return Err(IrohaZipError::Backend(format!(
                "libarchive candidate is not a DLL: {}",
                relative.display()
            )));
        }
        let candidate = backend_root.join(&relative);
        validate_regular_file_security(&candidate)?;
        let resolved = fs::canonicalize(&candidate).map_err(|error| {
            IrohaZipError::io_path("cannot resolve libarchive candidate", &candidate, error)
        })?;
        if !resolved.starts_with(&backend_root) {
            return Err(IrohaZipError::Backend(format!(
                "libarchive candidate escaped the backend root: {}",
                relative.display()
            )));
        }
        candidates.push(resolved);
    }
    if candidates.is_empty() {
        return Err(IrohaZipError::Backend(
            "verified backend contains no DLL candidate for the UTF-8 listing API".to_owned(),
        ));
    }
    Ok(candidates)
}

fn require_archive_status(status: c_int, operation: &str) -> Result<()> {
    if status < ARCHIVE_WARN {
        Err(IrohaZipError::Backend(format!(
            "libarchive could not {operation}: status {status}"
        )))
    } else {
        Ok(())
    }
}

unsafe fn bounded_c_bytes(pointer: *const c_char, maximum: usize) -> Result<Vec<u8>> {
    if pointer.is_null() {
        return Err(IrohaZipError::Policy(
            "libarchive returned no UTF-8 pathname".to_owned(),
        ));
    }
    let mut bytes = Vec::new();
    for index in 0..=maximum {
        let byte = unsafe { *pointer.add(index).cast::<u8>() };
        if byte == 0 {
            return Ok(bytes);
        }
        if index == maximum {
            return Err(IrohaZipError::Policy(format!(
                "archive member pathname exceeds {maximum} UTF-8 bytes"
            )));
        }
        bytes.push(byte);
    }
    unreachable!()
}

unsafe fn required_symbol<T: Copy>(module: *mut c_void, name: &'static CStr) -> Result<T> {
    if size_of::<T>() != size_of::<*mut c_void>() {
        return Err(IrohaZipError::Backend(
            "invalid internal libarchive symbol declaration".to_owned(),
        ));
    }
    let symbol = unsafe { get_proc_address(module, name.as_ptr().cast()) };
    if symbol.is_null() {
        return Err(IrohaZipError::Backend(format!(
            "libarchive DLL is missing symbol {}",
            name.to_string_lossy()
        )));
    }
    Ok(unsafe { transmute_copy(&symbol) })
}

fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}
