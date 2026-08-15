#![allow(unsafe_code)]

use std::ffi::{OsStr, OsString, c_void};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::mem::{size_of, size_of_val};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::ptr::null_mut;
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_HANDLE_EOF, ERROR_INVALID_PARAMETER,
    ERROR_NO_MORE_FILES, ERROR_SUCCESS, FreeLibrary, GetHandleInformation, GetLastError, HANDLE,
    HANDLE_FLAG_INHERIT, HANDLE_FLAGS, HLOCAL, LocalFree, SetHandleInformation, WAIT_ABANDONED,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, EXPLICIT_ACCESS_W, GetNamedSecurityInfoW, NO_MULTIPLE_TRUSTEE,
    SE_FILE_OBJECT, SET_ACCESS, SetEntriesInAclW, SetNamedSecurityInfoW, TRUSTEE_IS_SID,
    TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
};
use windows::Win32::Security::Cryptography::{
    BCRYPT_ALG_HANDLE, BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS, BCRYPT_RNG_ALGORITHM,
    BCRYPTGENRANDOM_FLAGS, BCryptCloseAlgorithmProvider, BCryptGenRandom,
    BCryptOpenAlgorithmProvider,
};
use windows::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, GetAppContainerFolderPath,
};
use windows::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, FreeSid, GetTokenInformation, NO_INHERITANCE,
    PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SECURITY_CAPABILITIES,
    SUB_CONTAINERS_AND_OBJECTS_INHERIT, TOKEN_GROUPS, TOKEN_QUERY, TokenCapabilities,
    TokenIsAppContainer, TokenIsLessPrivilegedAppContainer,
};
use windows::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, CREATE_NEW, CreateFileW, DELETE, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TEMPORARY, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_DELETE_ON_CLOSE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_SEQUENTIAL_SCAN,
    FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_ID_BOTH_DIR_INFO,
    FILE_LIST_DIRECTORY, FILE_SHARE_MODE, FILE_SHARE_READ, FileIdBothDirectoryInfo,
    FileIdBothDirectoryRestartInfo, FindClose, FindFirstStreamW, FindNextStreamW,
    FindStreamInfoStandard, GetFileInformationByHandle, GetFileInformationByHandleEx, GetTempPathW,
    OPEN_EXISTING, WIN32_FIND_STREAM_DATA, WRITE_DAC, WRITE_OWNER,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize,
};
use windows::Win32::System::JobObjects::{
    CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION,
    JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use windows::Win32::System::LibraryLoader::{
    BeginUpdateResourceW, EndUpdateResourceW, FindResourceW, LOAD_LIBRARY_AS_DATAFILE_EXCLUSIVE,
    LOAD_LIBRARY_AS_IMAGE_RESOURCE, LoadLibraryExW, LoadResource, LockResource, SizeofResource,
    UpdateResourceW,
};
use windows::Win32::System::Pipes::CreatePipe;
use windows::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateMutexW, CreateProcessW,
    DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess,
    GetExitCodeProcess, InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_JOB_LIST, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
    PROCESS_INFORMATION, ReleaseMutex, ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW,
    UpdateProcThreadAttribute, WaitForSingleObject,
};
use windows::Win32::System::WindowsProgramming::PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT;
use windows::Win32::UI::Shell::{AttachmentServices, IAttachmentExecute};
use windows::Win32::UI::WindowsAndMessaging::RT_MANIFEST;
use windows::core::{Error as WindowsError, GUID, HRESULT, PCWSTR, PWSTR};

use crate::config::IsolationMode;
use crate::error::{IrohaZipError, Result};
use crate::monitor;
use crate::password::MAX_PASSWORD_UTF8_BYTES;
use crate::platform::{
    FileIdentity, ProcessIsolation, ProcessResult, ProcessSpec, ProcessTempObservation,
};
use crate::util;
use crate::windows_command_line;

const CREATE_NO_WINDOW_RAW: u32 = 0x0800_0000;
const PASSWORD_PIPE_BUFFER_BYTES: u32 = 4 * 1024;
const _: () = assert!(
    MAX_PASSWORD_UTF8_BYTES < PASSWORD_PIPE_BUFFER_BYTES as usize,
    "the complete delimited password must fit in the pipe before child resume"
);
const APP_CONTAINER_PROFILE_DELETE_ATTEMPTS: usize = 20;
const APP_CONTAINER_PROFILE_DELETE_RETRY_DELAY: Duration = Duration::from_millis(50);
const CONFIG_SAVE_MUTEX_NAME: &str = r"Local\iroha-zip.ConfigSave.v1";
const CONFIG_SAVE_MUTEX_TIMEOUT_MS: u32 = 30_000;
static IROHA_ZIP_ATTACHMENT_CLIENT: GUID = GUID::from_u128(0x8d3f90af_f983_4c6f_86ce_79c192a9352a);
#[cfg(test)]
static FORCE_ISOLATION_VERIFICATION_FAILURE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
const BACKEND_MANIFEST_RESOURCE_ID: u16 = 1;
const BACKEND_MANIFEST_LANGUAGE_EN_US: u16 = 0x0409;
const UTF8_BACKEND_MANIFEST: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings xmlns:ws2="http://schemas.microsoft.com/SMI/2016/WindowsSettings" xmlns:ws3="http://schemas.microsoft.com/SMI/2019/WindowsSettings">
      <ws2:longPathAware>true</ws2:longPathAware>
      <ws3:activeCodePage>UTF-8</ws3:activeCodePage>
    </windowsSettings>
  </application>
</assembly>
"#;

pub fn prepare_backend_executable(path: &Path) -> Result<()> {
    validate_regular_file_security(path)?;
    let wide_path = wide_null(path.as_os_str());
    let update = unsafe { BeginUpdateResourceW(PCWSTR(wide_path.as_ptr()), true) }
        .map_err(|error| windows_error_path("BeginUpdateResourceW", path, error))?;
    let resource_name = integer_resource(BACKEND_MANIFEST_RESOURCE_ID);
    let manifest_length = u32::try_from(UTF8_BACKEND_MANIFEST.len())
        .map_err(|_| IrohaZipError::Backend("UTF-8 backend manifest length overflow".to_owned()))?;
    let write_result = unsafe {
        UpdateResourceW(
            update,
            RT_MANIFEST,
            resource_name,
            BACKEND_MANIFEST_LANGUAGE_EN_US,
            Some(UTF8_BACKEND_MANIFEST.as_ptr().cast()),
            manifest_length,
        )
    };
    if let Err(error) = write_result {
        let _ = unsafe { EndUpdateResourceW(update, true) };
        return Err(windows_error_path("UpdateResourceW", path, error));
    }
    unsafe { EndUpdateResourceW(update, false) }
        .map_err(|error| windows_error_path("EndUpdateResourceW", path, error))?;

    validate_regular_file_security(path)?;
    verify_utf8_backend_manifest(path)
}

fn verify_utf8_backend_manifest(path: &Path) -> Result<()> {
    let wide_path = wide_null(path.as_os_str());
    let module = unsafe {
        LoadLibraryExW(
            PCWSTR(wide_path.as_ptr()),
            None,
            LOAD_LIBRARY_AS_DATAFILE_EXCLUSIVE | LOAD_LIBRARY_AS_IMAGE_RESOURCE,
        )
    }
    .map_err(|error| windows_error_path("LoadLibraryExW for manifest verification", path, error))?;
    let verification = (|| {
        let resource = unsafe {
            FindResourceW(
                Some(module),
                integer_resource(BACKEND_MANIFEST_RESOURCE_ID),
                RT_MANIFEST,
            )
        };
        if resource.0.is_null() {
            return Err(IrohaZipError::Backend(format!(
                "UTF-8 backend manifest resource is missing after preparation: {}",
                path.display()
            )));
        }
        let size = unsafe { SizeofResource(Some(module), resource) };
        let size = usize::try_from(size).map_err(|_| {
            IrohaZipError::Backend("UTF-8 backend manifest resource length overflow".to_owned())
        })?;
        let loaded = unsafe { LoadResource(Some(module), resource) }
            .map_err(|error| windows_error_path("LoadResource", path, error))?;
        let pointer = unsafe { LockResource(loaded) };
        if pointer.is_null() {
            return Err(IrohaZipError::Backend(format!(
                "UTF-8 backend manifest resource is unreadable after preparation: {}",
                path.display()
            )));
        }
        let observed = unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), size) };
        if observed != UTF8_BACKEND_MANIFEST {
            return Err(IrohaZipError::Backend(format!(
                "UTF-8 backend manifest resource changed after preparation: {}",
                path.display()
            )));
        }
        Ok(())
    })();
    let release = unsafe { FreeLibrary(module) }
        .map_err(|error| windows_error_path("FreeLibrary", path, error));
    verification.and(release)
}

fn integer_resource(value: u16) -> PCWSTR {
    PCWSTR(usize::from(value) as *const u16)
}

pub struct AttachmentHandoffSession {
    _apartment: ComApartment,
}

impl AttachmentHandoffSession {
    pub fn new() -> Result<Self> {
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        result.ok().map_err(|error| {
            IrohaZipError::TrustHandoff(format!("cannot initialize COM: {error}"))
        })?;
        Ok(Self {
            _apartment: ComApartment,
        })
    }

    pub fn handoff(&self, path: &Path) -> Result<()> {
        let local_path = wide_null(path.as_os_str());
        let attachment: IAttachmentExecute =
            unsafe { CoCreateInstance(&AttachmentServices, None, CLSCTX_INPROC_SERVER) }.map_err(
                |error| {
                    IrohaZipError::TrustHandoff(format!(
                        "cannot create Attachment Services for {}: {error}",
                        path.display()
                    ))
                },
            )?;
        unsafe {
            attachment
                .SetClientGuid(&raw const IROHA_ZIP_ATTACHMENT_CLIENT)
                .map_err(|error| {
                    IrohaZipError::TrustHandoff(format!(
                        "cannot identify the iroha-zip client for {}: {error}",
                        path.display()
                    ))
                })?;
            attachment
                .SetLocalPath(PCWSTR(local_path.as_ptr()))
                .map_err(|error| {
                    IrohaZipError::TrustHandoff(format!(
                        "cannot set the local attachment path for {}: {error}",
                        path.display()
                    ))
                })?;
            attachment.Save().map_err(|error| {
                IrohaZipError::TrustHandoff(format!(
                    "Attachment Services rejected or could not process {}: {error}",
                    path.display()
                ))
            })?;
        }
        Ok(())
    }
}

struct ComApartment;

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub struct ConfigSaveGuard(HANDLE);

pub fn lock_config_save() -> Result<ConfigSaveGuard> {
    lock_named_config_save(
        OsStr::new(CONFIG_SAVE_MUTEX_NAME),
        CONFIG_SAVE_MUTEX_TIMEOUT_MS,
    )
}

fn lock_named_config_save(name: &OsStr, timeout_milliseconds: u32) -> Result<ConfigSaveGuard> {
    let name = wide_null(name);
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }
        .map_err(|error| windows_error("CreateMutexW(config save)", error))?;
    let wait = unsafe { WaitForSingleObject(handle, timeout_milliseconds) };
    if wait != WAIT_OBJECT_0 && wait != WAIT_ABANDONED {
        let _ = unsafe { CloseHandle(handle) };
        return if wait == WAIT_TIMEOUT {
            Err(IrohaZipError::Config(
                "timed out waiting for another configuration save".to_owned(),
            ))
        } else {
            Err(IrohaZipError::Config(format!(
                "cannot acquire configuration save mutex: wait result {}",
                wait.0
            )))
        };
    }
    Ok(ConfigSaveGuard(handle))
}

impl Drop for ConfigSaveGuard {
    fn drop(&mut self) {
        let _ = unsafe { ReleaseMutex(self.0) };
        let _ = unsafe { CloseHandle(self.0) };
    }
}

pub struct Sandbox {
    root: PathBuf,
    sealed_source_parent: Option<PathBuf>,
    memory_limit_bytes: usize,
    mode: Option<Mode>,
}

pub struct DirectorySnapshot {
    path: PathBuf,
    handle: OwnedHandle,
    identity: FileIdentity,
}

impl DirectorySnapshot {
    pub fn open(path: &Path) -> Result<Self> {
        validate_directory_security(path)?;
        let path = fs::canonicalize(path).map_err(|error| {
            IrohaZipError::io_path("cannot resolve directory snapshot", path, error)
        })?;
        validate_directory_security(&path)?;
        let handle = open_directory_handle(&path)?;
        let info = file_information_from_raw_handle(&path, handle.handle())?;
        if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
            || info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 == 0
        {
            return Err(IrohaZipError::Policy(format!(
                "directory snapshot is not a real directory: {}",
                path.display()
            )));
        }
        let identity = identity_from_file_information(&info);
        let snapshot = Self {
            path,
            handle,
            identity,
        };
        snapshot.verify_unchanged()?;
        Ok(snapshot)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn identity(&self) -> Option<&FileIdentity> {
        Some(&self.identity)
    }

    pub fn entries(&self, max_entries: u64) -> Result<Vec<OsString>> {
        self.verify_unchanged()?;
        let mut storage = vec![0_u64; 8192];
        let buffer_bytes = storage
            .len()
            .checked_mul(size_of::<u64>())
            .ok_or_else(|| IrohaZipError::Policy("directory buffer size overflow".to_owned()))?;
        let buffer_size = u32::try_from(buffer_bytes).map_err(|_| {
            IrohaZipError::Policy("directory buffer does not fit the Windows API".to_owned())
        })?;
        let mut names = Vec::new();
        let mut class = FileIdBothDirectoryRestartInfo;
        loop {
            match unsafe {
                GetFileInformationByHandleEx(
                    self.handle.handle(),
                    class,
                    storage.as_mut_ptr().cast::<c_void>(),
                    buffer_size,
                )
            } {
                Ok(()) => parse_directory_buffer(&self.path, &storage, max_entries, &mut names)?,
                Err(error) if is_windows_error(&error, ERROR_NO_MORE_FILES.0) => break,
                Err(error) => {
                    return Err(windows_error_path(
                        "GetFileInformationByHandleEx(directory)",
                        &self.path,
                        error,
                    ));
                }
            }
            class = FileIdBothDirectoryInfo;
        }
        names.sort();
        self.verify_unchanged()?;
        Ok(names)
    }

    fn verify_unchanged(&self) -> Result<()> {
        validate_directory_security(&self.path)?;
        let handle_info = file_information_from_raw_handle(&self.path, self.handle.handle())?;
        if handle_info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
            || handle_info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 == 0
            || identity_from_file_information(&handle_info) != self.identity
        {
            return Err(IrohaZipError::Policy(format!(
                "directory handle changed during enumeration: {}",
                self.path.display()
            )));
        }
        let comparison = open_directory_handle(&self.path)?;
        let path_info = file_information_from_raw_handle(&self.path, comparison.handle())?;
        if identity_from_file_information(&path_info) != self.identity {
            return Err(IrohaZipError::Policy(format!(
                "directory identity changed during enumeration: {}",
                self.path.display()
            )));
        }
        Ok(())
    }
}

fn open_directory_handle(path: &Path) -> Result<OwnedHandle> {
    let wide = wide_null(path.as_os_str());
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|error| windows_error_path("CreateFileW(directory snapshot)", path, error))?;
    Ok(OwnedHandle::new(handle))
}

fn parse_directory_buffer(
    path: &Path,
    storage: &[u64],
    max_entries: u64,
    names: &mut Vec<OsString>,
) -> Result<()> {
    const NAME_OFFSET: usize = std::mem::offset_of!(FILE_ID_BOTH_DIR_INFO, FileName);

    let buffer_bytes = storage
        .len()
        .checked_mul(size_of::<u64>())
        .ok_or_else(|| IrohaZipError::Policy("directory buffer size overflow".to_owned()))?;
    let mut offset = 0usize;
    loop {
        let header_end = offset.checked_add(NAME_OFFSET).ok_or_else(|| {
            IrohaZipError::Policy(format!(
                "directory entry offset overflow while enumerating {}",
                path.display()
            ))
        })?;
        if header_end > buffer_bytes {
            return Err(invalid_directory_buffer(path));
        }
        let structure_end = offset
            .checked_add(size_of::<FILE_ID_BOTH_DIR_INFO>())
            .ok_or_else(|| invalid_directory_buffer(path))?;
        if structure_end > buffer_bytes || header_end % size_of::<u16>() != 0 {
            return Err(invalid_directory_buffer(path));
        }
        let info = unsafe {
            &*storage
                .as_ptr()
                .add(offset / size_of::<u64>())
                .cast::<FILE_ID_BOTH_DIR_INFO>()
        };
        let name_bytes =
            usize::try_from(info.FileNameLength).map_err(|_| invalid_directory_buffer(path))?;
        if name_bytes == 0 || name_bytes % size_of::<u16>() != 0 {
            return Err(invalid_directory_buffer(path));
        }
        let name_end = header_end
            .checked_add(name_bytes)
            .ok_or_else(|| invalid_directory_buffer(path))?;
        if name_end > buffer_bytes {
            return Err(invalid_directory_buffer(path));
        }
        let name_units = unsafe {
            std::slice::from_raw_parts(
                storage
                    .as_ptr()
                    .cast::<u16>()
                    .add(header_end / size_of::<u16>()),
                name_bytes / size_of::<u16>(),
            )
        };
        let name = OsString::from_wide(name_units);
        if name != OsStr::new(".") && name != OsStr::new("..") {
            if u64::try_from(names.len()).unwrap_or(u64::MAX) >= max_entries {
                return Err(IrohaZipError::Policy(format!(
                    "directory contains more than {max_entries} entries: {}",
                    path.display()
                )));
            }
            names.push(name);
        }

        let next =
            usize::try_from(info.NextEntryOffset).map_err(|_| invalid_directory_buffer(path))?;
        if next == 0 {
            break;
        }
        if next % size_of::<u64>() != 0
            || next < name_end.saturating_sub(offset)
            || next < NAME_OFFSET + size_of::<u16>()
        {
            return Err(invalid_directory_buffer(path));
        }
        offset = offset
            .checked_add(next)
            .ok_or_else(|| invalid_directory_buffer(path))?;
        if offset >= buffer_bytes {
            return Err(invalid_directory_buffer(path));
        }
    }
    Ok(())
}

fn invalid_directory_buffer(path: &Path) -> IrohaZipError {
    IrohaZipError::Policy(format!(
        "Windows returned a malformed directory enumeration buffer for {}",
        path.display()
    ))
}

enum Mode {
    AppContainer {
        profile_name: String,
        sid: PSID,
        isolation: IsolationMode,
    },
    Unsandboxed,
}

impl Sandbox {
    pub fn new(
        memory_limit_mib: u64,
        allow_unsandboxed: bool,
        isolation: IsolationMode,
    ) -> Result<Self> {
        let bytes = memory_limit_mib
            .checked_mul(1024 * 1024)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| IrohaZipError::Config("sandbox memory limit is too large".to_owned()))?;
        if bytes < 64 * 1024 * 1024 {
            return Err(IrohaZipError::Config(
                "sandbox memory limit must be at least 64 MiB".to_owned(),
            ));
        }

        match create_appcontainer() {
            Ok((profile_name, sid, root)) => {
                let mut sandbox = Self {
                    root,
                    sealed_source_parent: None,
                    memory_limit_bytes: bytes,
                    mode: Some(Mode::AppContainer {
                        profile_name,
                        sid,
                        isolation,
                    }),
                };
                let parent = std::env::temp_dir().join("iroha-zip-staged-sources");
                match util::create_unique_dir(&parent, "job-") {
                    Ok(path) => {
                        sandbox.sealed_source_parent = Some(path.clone());
                        let prepared = (|| {
                            validate_directory_security(&path)?;
                            let resolved = fs::canonicalize(&path).map_err(|error| {
                                IrohaZipError::io_path(
                                    "cannot resolve sealed staging source root",
                                    &path,
                                    error,
                                )
                            })?;
                            validate_directory_security(&resolved)?;
                            Ok(resolved)
                        })();
                        match prepared {
                            Ok(resolved) => {
                                if let Err(error) =
                                    grant_appcontainer_parent_readonly(&resolved, sid)
                                {
                                    return sandbox.fail_after_cleanup(error);
                                }
                                sandbox.sealed_source_parent = Some(resolved);
                                Ok(sandbox)
                            }
                            Err(error) => sandbox.fail_after_cleanup(error),
                        }
                    }
                    Err(error) => sandbox.fail_after_cleanup(error),
                }
            }
            Err(error) if allow_unsandboxed => {
                eprintln!(
                    "warning: AppContainer creation failed; explicit unsandboxed fallback is active: {error}"
                );
                let parent = std::env::temp_dir().join("iroha-zip-unsandboxed");
                let root = util::create_unique_dir(&parent, "job-")?;
                Ok(Self {
                    root,
                    sealed_source_parent: None,
                    memory_limit_bytes: bytes,
                    mode: Some(Mode::Unsandboxed),
                })
            }
            Err(error) => Err(error),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn staged_source_path(&self) -> PathBuf {
        self.sealed_source_parent
            .as_deref()
            .unwrap_or(&self.root)
            .join("source")
    }

    pub fn create_process_scratch(&self) -> Result<PathBuf> {
        // Windows reroutes TEMP and TMP for an AppContainer to this fixed
        // profile-relative name, regardless of the supplied environment block.
        let path = self.root.join("Temp");
        match fs::create_dir(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(IrohaZipError::io_path(
                    "cannot create process scratch directory",
                    &path,
                    error,
                ));
            }
        }
        validate_directory_security(&path)?;
        if fs::read_dir(&path)
            .map_err(|error| {
                IrohaZipError::io_path(
                    "cannot inspect fresh process scratch directory",
                    &path,
                    error,
                )
            })?
            .next()
            .transpose()
            .map_err(|error| {
                IrohaZipError::io_path("cannot read fresh process scratch directory", &path, error)
            })?
            .is_some()
        {
            return Err(IrohaZipError::Policy(
                "fresh process scratch directory is unexpectedly non-empty".to_owned(),
            ));
        }
        let mode = self.mode.as_ref().ok_or_else(|| {
            IrohaZipError::Sandbox(
                "cannot create a process scratch directory after sandbox cleanup".to_owned(),
            )
        })?;
        if let Mode::AppContainer { sid, .. } = mode {
            set_appcontainer_access(
                &path,
                *sid,
                FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0 | FILE_GENERIC_EXECUTE.0 | DELETE.0,
                SUB_CONTAINERS_AND_OBJECTS_INHERIT,
                true,
                "process scratch directory",
            )?;
        }
        Ok(path)
    }

    pub fn finish_process_scratch(&self, path: &Path) -> Result<()> {
        validate_directory_security(path)?;
        if path != self.root.join("Temp") {
            return Err(IrohaZipError::Sandbox(format!(
                "refusing to inspect an unexpected process scratch directory: {}",
                path.display()
            )));
        }
        let mut entries = fs::read_dir(path).map_err(|error| {
            IrohaZipError::io_path("cannot inspect process scratch directory", path, error)
        })?;
        if entries
            .next()
            .transpose()
            .map_err(|error| {
                IrohaZipError::io_path("cannot read process scratch directory", path, error)
            })?
            .is_some()
        {
            return Err(IrohaZipError::Policy(
                "archive backend left an unexpected process scratch entry".to_owned(),
            ));
        }
        fs::remove_dir(path).map_err(|error| {
            IrohaZipError::io_path("cannot remove empty process scratch directory", path, error)
        })
    }

    pub fn profile_name(&self) -> Option<&str> {
        match self.mode.as_ref()? {
            Mode::AppContainer { profile_name, .. } => Some(profile_name),
            Mode::Unsandboxed => None,
        }
    }

    pub fn run(&self, mut spec: ProcessSpec) -> Result<ProcessResult> {
        if let Some(temp_dir) = spec.temp_dir.as_deref() {
            validate_directory_security(temp_dir)?;
            let resolved = fs::canonicalize(temp_dir).map_err(|error| {
                IrohaZipError::io_path("cannot resolve process scratch directory", temp_dir, error)
            })?;
            if resolved != self.root.join("Temp") {
                return Err(IrohaZipError::Sandbox(format!(
                    "process scratch directory is not the AppContainer Temp directory: {}",
                    resolved.display()
                )));
            }
            spec.temp_dir = Some(resolved);
        }
        let mode = self.mode.as_ref().ok_or_else(|| {
            IrohaZipError::Sandbox("cannot run a process after sandbox cleanup".to_owned())
        })?;
        match mode {
            Mode::AppContainer { sid, isolation, .. } => {
                run_in_appcontainer(*sid, *isolation, self.memory_limit_bytes, &self.root, spec)
            }
            Mode::Unsandboxed => run_unsandboxed(spec),
        }
    }

    pub fn seal_staged_source_tree(&self, path: &Path, max_entries: u64) -> Result<bool> {
        let resolved = fs::canonicalize(path).map_err(|error| {
            IrohaZipError::io_path(
                "cannot resolve staged source tree before sealing",
                path,
                error,
            )
        })?;
        validate_directory_security(&resolved)?;
        let allowed_parent = self.sealed_source_parent.as_deref().unwrap_or(&self.root);
        if resolved == allowed_parent || !resolved.starts_with(allowed_parent) {
            return Err(IrohaZipError::Sandbox(format!(
                "refusing to seal a staged tree outside its sandbox child: {}",
                resolved.display()
            )));
        }

        let mode = self.mode.as_ref().ok_or_else(|| {
            IrohaZipError::Sandbox("cannot seal a staged tree after sandbox cleanup".to_owned())
        })?;
        match mode {
            Mode::AppContainer { sid, .. } => {
                restrict_appcontainer_tree_to_readonly_recursive(&resolved, *sid, max_entries)?;
                Ok(true)
            }
            Mode::Unsandboxed => Ok(false),
        }
    }

    pub fn seal_sandbox_tree(&self, path: &Path, max_entries: u64) -> Result<bool> {
        let resolved = fs::canonicalize(path).map_err(|error| {
            IrohaZipError::io_path("cannot resolve sandbox tree before sealing", path, error)
        })?;
        validate_directory_security(&resolved)?;
        if resolved == self.root || !resolved.starts_with(&self.root) {
            return Err(IrohaZipError::Sandbox(format!(
                "refusing to change an ACL outside a sandbox-root child: {}",
                resolved.display()
            )));
        }

        let mode = self.mode.as_ref().ok_or_else(|| {
            IrohaZipError::Sandbox("cannot seal a tree after sandbox cleanup".to_owned())
        })?;
        match mode {
            Mode::AppContainer { sid, .. } => {
                restrict_appcontainer_tree_to_readonly_recursive(&resolved, *sid, max_entries)?;
                Ok(true)
            }
            Mode::Unsandboxed => Ok(false),
        }
    }

    pub fn cleanup(mut self) -> Result<()> {
        self.cleanup_inner()
    }

    pub fn fail_after_cleanup<T>(self, failure: IrohaZipError) -> Result<T> {
        match self.cleanup() {
            Ok(()) => Err(failure),
            Err(cleanup) => Err(IrohaZipError::Sandbox(format!(
                "{failure}; sandbox cleanup also failed: {cleanup}"
            ))),
        }
    }

    fn cleanup_inner(&mut self) -> Result<()> {
        let Some(mode) = self.mode.take() else {
            return Ok(());
        };
        let mut failure = None;
        match mode {
            Mode::AppContainer {
                profile_name, sid, ..
            } => unsafe {
                if let Err(error) = delete_appcontainer_profile_with_retry(&profile_name) {
                    failure = Some(error);
                }
                if let Err(error) = fs::remove_dir_all(&self.root)
                    && error.kind() != std::io::ErrorKind::NotFound
                    && failure.is_none()
                {
                    failure = Some(IrohaZipError::io_path(
                        "cannot remove AppContainer temporary root",
                        &self.root,
                        error,
                    ));
                }
                if let Some(source_parent) = &self.sealed_source_parent
                    && let Err(error) = fs::remove_dir_all(source_parent)
                    && error.kind() != std::io::ErrorKind::NotFound
                    && failure.is_none()
                {
                    failure = Some(IrohaZipError::io_path(
                        "cannot remove sealed staging source root",
                        source_parent,
                        error,
                    ));
                }
                let _ = FreeSid(sid);
            },
            Mode::Unsandboxed => {
                if let Err(error) = fs::remove_dir_all(&self.root)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    failure = Some(IrohaZipError::io_path(
                        "cannot remove unsandboxed temporary root",
                        &self.root,
                        error,
                    ));
                }
            }
        }
        if self.root.exists() && failure.is_none() {
            failure = Some(IrohaZipError::Sandbox(format!(
                "sandbox temporary root still exists after cleanup: {}",
                self.root.display()
            )));
        }
        if let Some(source_parent) = &self.sealed_source_parent
            && source_parent.exists()
            && failure.is_none()
        {
            failure = Some(IrohaZipError::Sandbox(format!(
                "sealed staging source root still exists after cleanup: {}",
                source_parent.display()
            )));
        }
        failure.map_or(Ok(()), Err)
    }
}

fn restrict_appcontainer_tree_to_readonly_recursive(
    path: &Path,
    sid: PSID,
    max_entries: u64,
) -> Result<()> {
    let mut stack = vec![path.to_path_buf()];
    let mut entries = Vec::<(PathBuf, bool)>::new();
    let mut observed = 0u64;
    while let Some(directory) = stack.pop() {
        entries.push((directory.clone(), true));
        let children = fs::read_dir(&directory).map_err(|error| {
            IrohaZipError::io_path(
                "cannot enumerate sandbox tree while sealing",
                &directory,
                error,
            )
        })?;
        for child in children {
            let child = child.map_err(|error| {
                IrohaZipError::io_path("cannot read sandbox entry while sealing", &directory, error)
            })?;
            let child_path = child.path();
            let metadata = fs::symlink_metadata(&child_path).map_err(|error| {
                IrohaZipError::io_path(
                    "cannot inspect sandbox entry while sealing",
                    &child_path,
                    error,
                )
            })?;
            validate_extracted_entry_security(&child_path, &metadata)?;
            observed = observed.checked_add(1).ok_or_else(|| {
                IrohaZipError::Policy("sandbox sealing entry count overflow".to_owned())
            })?;
            if observed > max_entries {
                return Err(IrohaZipError::Policy(format!(
                    "sandbox tree changed before sealing; observed more than {max_entries} entries"
                )));
            }
            if metadata.is_dir() {
                stack.push(child_path);
            } else if metadata.is_file() {
                entries.push((child_path, false));
            } else {
                return Err(IrohaZipError::Policy(format!(
                    "special object appeared before sandbox sealing: {}",
                    child_path.display()
                )));
            }
        }
    }
    if observed != max_entries {
        return Err(IrohaZipError::Policy(format!(
            "sandbox tree entry count changed before sealing: expected {max_entries}, observed {observed}"
        )));
    }

    // Set every object's Package-SID access explicitly before sealing the root.
    // This removes any explicit write-capable ACE already present on a child;
    // root inheritance alone would preserve such an ACE.
    for (entry, is_directory) in entries.into_iter().rev() {
        set_appcontainer_access(
            &entry,
            sid,
            FILE_GENERIC_READ.0 | FILE_GENERIC_EXECUTE.0,
            if is_directory {
                SUB_CONTAINERS_AND_OBJECTS_INHERIT
            } else {
                NO_INHERITANCE
            },
            true,
            "sealed staged source",
        )?;
    }
    Ok(())
}

fn grant_appcontainer_parent_readonly(path: &Path, sid: PSID) -> Result<()> {
    // libarchive visits `.` before descending into it, which requires listing the
    // source entry from its unique per-job parent. Keep that parent read-only and
    // do not inherit access to future children. The source tree receives its own
    // protected read-only ACL immediately before the backend starts.
    set_appcontainer_access(
        path,
        sid,
        FILE_GENERIC_READ.0 | FILE_GENERIC_EXECUTE.0,
        NO_INHERITANCE,
        false,
        "sealed staging parent",
    )
}

fn set_appcontainer_access(
    path: &Path,
    sid: PSID,
    access_permissions: u32,
    inheritance: windows::Win32::Security::ACE_FLAGS,
    protect_dacl: bool,
    operation_name: &str,
) -> Result<()> {
    let wide_path = wide_null(path.as_os_str());
    let mut existing_acl: *mut ACL = null_mut();
    let mut security_descriptor = PSECURITY_DESCRIPTOR::default();
    let get_status = unsafe {
        GetNamedSecurityInfoW(
            PCWSTR(wide_path.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&raw mut existing_acl),
            None,
            &raw mut security_descriptor,
        )
    };
    if get_status != ERROR_SUCCESS {
        return Err(windows_status_error_path(
            &format!("GetNamedSecurityInfoW({operation_name})"),
            path,
            get_status.0,
        ));
    }

    let operation = (|| {
        if existing_acl.is_null() {
            return Err(IrohaZipError::Sandbox(format!(
                "{operation_name} unexpectedly has a null DACL: {}",
                path.display()
            )));
        }

        let entry = EXPLICIT_ACCESS_W {
            grfAccessPermissions: access_permissions,
            grfAccessMode: SET_ACCESS,
            grfInheritance: inheritance,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: null_mut(),
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_UNKNOWN,
                ptstrName: PWSTR(sid.0.cast::<u16>()),
            },
        };
        let mut sealed_acl: *mut ACL = null_mut();
        let merge_status = unsafe {
            SetEntriesInAclW(
                Some(std::slice::from_ref(&entry)),
                Some(existing_acl),
                &raw mut sealed_acl,
            )
        };
        if merge_status != ERROR_SUCCESS {
            return Err(windows_status_error_path(
                &format!("SetEntriesInAclW({operation_name})"),
                path,
                merge_status.0,
            ));
        }
        if sealed_acl.is_null() {
            return Err(IrohaZipError::Sandbox(format!(
                "SetEntriesInAclW returned a null DACL for {}",
                path.display()
            )));
        }

        let security_information = if protect_dacl {
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION
        } else {
            DACL_SECURITY_INFORMATION
        };
        let set_status = unsafe {
            SetNamedSecurityInfoW(
                PCWSTR(wide_path.as_ptr()),
                SE_FILE_OBJECT,
                security_information,
                None,
                None,
                Some(sealed_acl),
                None,
            )
        };
        unsafe {
            let _ = LocalFree(Some(HLOCAL(sealed_acl.cast::<c_void>())));
        }
        if set_status != ERROR_SUCCESS {
            return Err(windows_status_error_path(
                &format!("SetNamedSecurityInfoW({operation_name})"),
                path,
                set_status.0,
            ));
        }
        Ok(())
    })();

    unsafe {
        let _ = LocalFree(Some(HLOCAL(security_descriptor.0)));
    }
    operation
}

pub fn probe_staging_security_write_denials(path: &Path) -> Result<(bool, bool)> {
    Ok((
        access_mask_is_denied(path, WRITE_DAC.0)?,
        access_mask_is_denied(path, WRITE_OWNER.0)?,
    ))
}

pub fn probe_process_temp() -> Result<ProcessTempObservation> {
    let environment_primary = std::env::var_os("TEMP").unwrap_or_default();
    let environment_legacy = std::env::var_os("TMP").unwrap_or_default();
    let mut buffer = [0_u16; 512];
    let length = unsafe { GetTempPathW(Some(&mut buffer)) };
    if length == 0 {
        let code = unsafe { GetLastError() };
        return Err(process_temp_error(
            "GetTempPathW",
            &environment_primary,
            &environment_legacy,
            None,
            &format!("Win32 error {}", code.0),
        ));
    }
    let length = usize::try_from(length).map_err(|_| {
        process_temp_error(
            "GetTempPathW length conversion",
            &environment_primary,
            &environment_legacy,
            None,
            "returned length does not fit usize",
        )
    })?;
    if length >= buffer.len() {
        return Err(process_temp_error(
            "GetTempPathW buffer bound",
            &environment_primary,
            &environment_legacy,
            None,
            &format!("required {length} UTF-16 code units"),
        ));
    }
    let resolved_path = PathBuf::from(OsString::from_wide(&buffer[..length]));
    let metadata = fs::metadata(&resolved_path).map_err(|error| {
        process_temp_error(
            "temp directory metadata",
            &environment_primary,
            &environment_legacy,
            Some(&resolved_path),
            &format!("{error}; raw_os_error={:?}", error.raw_os_error()),
        )
    })?;
    if !metadata.is_dir() {
        return Err(process_temp_error(
            "temp directory type",
            &environment_primary,
            &environment_legacy,
            Some(&resolved_path),
            "resolved path is not a directory",
        ));
    }
    validate_directory_security(&resolved_path)?;

    let mut provider = BCRYPT_ALG_HANDLE::default();
    let open_status = unsafe {
        BCryptOpenAlgorithmProvider(
            &raw mut provider,
            BCRYPT_RNG_ALGORITHM,
            PCWSTR::null(),
            BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0),
        )
    };
    if open_status.is_err() {
        return Err(process_temp_error(
            "BCryptOpenAlgorithmProvider",
            &environment_primary,
            &environment_legacy,
            Some(&resolved_path),
            &format!("NTSTATUS 0x{:08X}", open_status.0.cast_unsigned()),
        ));
    }
    let mut random = [0_u8; 20];
    let random_status =
        unsafe { BCryptGenRandom(Some(provider), &mut random, BCRYPTGENRANDOM_FLAGS(0)) };
    let close_status = unsafe { BCryptCloseAlgorithmProvider(provider, 0) };
    if random_status.is_err() {
        return Err(process_temp_error(
            "BCryptGenRandom",
            &environment_primary,
            &environment_legacy,
            Some(&resolved_path),
            &format!("NTSTATUS 0x{:08X}", random_status.0.cast_unsigned()),
        ));
    }
    if close_status.is_err() {
        return Err(process_temp_error(
            "BCryptCloseAlgorithmProvider",
            &environment_primary,
            &environment_legacy,
            Some(&resolved_path),
            &format!("NTSTATUS 0x{:08X}", close_status.0.cast_unsigned()),
        ));
    }

    let filename = format!(
        "iroha-zip-temp-probe-{}-{:02x}{:02x}{:02x}{:02x}.tmp",
        std::process::id(),
        random[0],
        random[1],
        random[2],
        random[3]
    );
    let probe_path = resolved_path.join(filename);
    let wide_path = wide_null(probe_path.as_os_str());
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide_path.as_ptr()),
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0 | DELETE.0,
            FILE_SHARE_MODE(0),
            None,
            CREATE_NEW,
            FILE_ATTRIBUTE_TEMPORARY | FILE_FLAG_DELETE_ON_CLOSE,
            None,
        )
    }
    .map_err(|error| {
        process_temp_error(
            "CreateFileW temporary read/write/delete-on-close",
            &environment_primary,
            &environment_legacy,
            Some(&resolved_path),
            &format!("HRESULT 0x{:08X}: {error}", error.code().0.cast_unsigned()),
        )
    })?;
    drop(OwnedHandle::new(handle));
    if probe_path.exists() {
        return Err(process_temp_error(
            "FILE_FLAG_DELETE_ON_CLOSE",
            &environment_primary,
            &environment_legacy,
            Some(&resolved_path),
            "probe file remained after closing its only handle",
        ));
    }

    Ok(ProcessTempObservation {
        temp_environment: environment_primary,
        tmp_environment: environment_legacy,
        resolved_path,
    })
}

fn process_temp_error(
    step: &str,
    environment_primary: &OsStr,
    environment_legacy: &OsStr,
    resolved_path: Option<&Path>,
    detail: &str,
) -> IrohaZipError {
    IrohaZipError::Sandbox(format!(
        "process temp probe failed at {step}: {detail}; TEMP={}; TMP={}; GetTempPathW={:?}",
        environment_primary.display(),
        environment_legacy.display(),
        resolved_path.map(|path| path.display().to_string())
    ))
}

fn access_mask_is_denied(path: &Path, access_mask: u32) -> Result<bool> {
    let mut options = OpenOptions::new();
    options.access_mode(access_mask);
    match options.open(path) {
        Ok(file) => {
            drop(file);
            Ok(false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Ok(true),
        Err(error) => Err(IrohaZipError::io_path(
            "staging security access probe failed unexpectedly",
            path,
            error,
        )),
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup_inner() {
            eprintln!("warning: sandbox cleanup failed: {error}");
        }
    }
}

fn create_appcontainer() -> Result<(String, PSID, PathBuf)> {
    let token = util::unique_token().replace('-', "");
    let suffix_len = token.len().min(28);
    let profile_name = format!("iroha-zip.Job.{}", &token[..suffix_len]);
    let name = wide_null(OsStr::new(&profile_name));
    let display = wide_null(OsStr::new("iroha-zip extraction job"));
    let description = wide_null(OsStr::new(
        "Ephemeral no-network archive extraction container",
    ));

    let sid = unsafe {
        CreateAppContainerProfile(
            PCWSTR(name.as_ptr()),
            PCWSTR(display.as_ptr()),
            PCWSTR(description.as_ptr()),
            None,
        )
    }
    .map_err(|error| windows_error("CreateAppContainerProfile", error))?;

    let mut cleanup_root = None;
    let setup_result = (|| {
        let root = appcontainer_folder(sid)?;
        cleanup_root = Some(root.clone());
        fs::create_dir_all(&root).map_err(|error| {
            IrohaZipError::io_path("cannot create AppContainer storage", &root, error)
        })?;
        validate_directory_security(&root)?;
        let resolved_root = fs::canonicalize(&root).map_err(|error| {
            IrohaZipError::io_path("cannot resolve AppContainer storage", &root, error)
        })?;
        validate_directory_security(&resolved_root)?;
        Ok(resolved_root)
    })();
    match setup_result {
        Ok(resolved_root) => Ok((profile_name, sid, resolved_root)),
        Err(error) => {
            let mut cleanup_errors = Vec::new();
            if let Err(cleanup) = delete_appcontainer_profile_with_retry(&profile_name) {
                cleanup_errors.push(cleanup.to_string());
            }
            if let Some(root) = cleanup_root
                && let Err(cleanup) = fs::remove_dir_all(&root)
                && cleanup.kind() != std::io::ErrorKind::NotFound
            {
                cleanup_errors.push(format!(
                    "cannot remove AppContainer initialization root {}: {cleanup}",
                    root.display()
                ));
            }
            unsafe {
                let _ = FreeSid(sid);
            }
            if cleanup_errors.is_empty() {
                Err(error)
            } else {
                Err(IrohaZipError::Sandbox(format!(
                    "{error}; AppContainer initialization cleanup also failed: {}",
                    cleanup_errors.join("; ")
                )))
            }
        }
    }
}

fn delete_appcontainer_profile_with_retry(profile_name: &str) -> Result<()> {
    let name = wide_null(OsStr::new(profile_name));
    retry_appcontainer_profile_delete(
        || unsafe { DeleteAppContainerProfile(PCWSTR(name.as_ptr())) },
        thread::sleep,
    )
    .map_err(|error| windows_error("DeleteAppContainerProfile", error))
}

fn retry_appcontainer_profile_delete(
    mut delete: impl FnMut() -> windows::core::Result<()>,
    mut delay: impl FnMut(Duration),
) -> windows::core::Result<()> {
    let mut last_error = None;
    for attempt in 0..APP_CONTAINER_PROFILE_DELETE_ATTEMPTS {
        match delete() {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
        if attempt + 1 < APP_CONTAINER_PROFILE_DELETE_ATTEMPTS {
            delay(APP_CONTAINER_PROFILE_DELETE_RETRY_DELAY);
        }
    }
    Err(last_error.expect("profile deletion is attempted at least once"))
}

fn appcontainer_folder(sid: PSID) -> Result<PathBuf> {
    let mut sid_text = PWSTR::null();
    unsafe { ConvertSidToStringSidW(sid, &raw mut sid_text) }
        .map_err(|error| windows_error("ConvertSidToStringSidW", error))?;

    let folder_result = unsafe { GetAppContainerFolderPath(PCWSTR(sid_text.0)) };
    unsafe {
        let _ = LocalFree(Some(HLOCAL(sid_text.0.cast::<c_void>())));
    }
    let folder =
        folder_result.map_err(|error| windows_error("GetAppContainerFolderPath", error))?;
    let os = unsafe { os_string_from_pwstr(folder) };
    unsafe {
        CoTaskMemFree(Some(folder.0.cast_const().cast::<c_void>()));
    }
    Ok(PathBuf::from(os))
}

fn run_in_appcontainer(
    sid: PSID,
    isolation: IsolationMode,
    memory_limit_bytes: usize,
    sandbox_root: &Path,
    mut spec: ProcessSpec,
) -> Result<ProcessResult> {
    let stdout = File::create(&spec.stdout_log).map_err(|error| {
        IrohaZipError::io_path("cannot create process stdout log", &spec.stdout_log, error)
    })?;
    let stderr = File::create(&spec.stderr_log).map_err(|error| {
        IrohaZipError::io_path("cannot create process stderr log", &spec.stderr_log, error)
    })?;
    let (stdin, mut password_channel) = if let Some(password) = spec.interactive_password.take() {
        if spec.stdin_file.is_some() {
            return Err(IrohaZipError::Sandbox(
                "one-use password input cannot be combined with a stdin file".to_owned(),
            ));
        }
        let (child_read, controller_write) = create_noninheritable_pipe()?;
        (
            child_read.into_file(),
            Some((controller_write.into_file(), password.into_transport()?)),
        )
    } else {
        (open_child_stdin(&spec)?, None)
    };

    let stdout_handle = raw_handle(&stdout);
    let stderr_handle = raw_handle(&stderr);
    let stdin_handle = raw_handle(&stdin);
    for handle in [stdout_handle, stderr_handle, stdin_handle] {
        unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT.0, HANDLE_FLAG_INHERIT) }
            .map_err(|error| windows_error("SetHandleInformation", error))?;
    }

    let job = OwnedHandle::new(
        unsafe { CreateJobObjectW(None, PCWSTR::null()) }
            .map_err(|error| windows_error("CreateJobObjectW", error))?,
    );
    let mut job_limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    job_limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        | JOB_OBJECT_LIMIT_JOB_MEMORY
        | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
        | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
    job_limits.BasicLimitInformation.ActiveProcessLimit = 1;
    job_limits.JobMemoryLimit = memory_limit_bytes;
    let job_limits_size = u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
        .map_err(|_| IrohaZipError::Sandbox("job limits structure is too large".to_owned()))?;
    unsafe {
        SetInformationJobObject(
            job.handle(),
            JobObjectExtendedLimitInformation,
            (&raw const job_limits).cast::<c_void>(),
            job_limits_size,
        )
    }
    .map_err(|error| windows_error("SetInformationJobObject", error))?;

    let attribute_count = if isolation.is_lpac() { 4 } else { 3 };
    let attributes = AttributeList::new(attribute_count)?;
    let capabilities = SECURITY_CAPABILITIES {
        AppContainerSid: sid,
        Capabilities: null_mut(),
        CapabilityCount: 0,
        Reserved: 0,
    };
    unsafe {
        UpdateProcThreadAttribute(
            attributes.list,
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            Some((&raw const capabilities).cast::<c_void>()),
            size_of::<SECURITY_CAPABILITIES>(),
            None,
            None,
        )
    }
    .map_err(|error| windows_error("set AppContainer process attribute", error))?;

    let all_application_packages_policy = PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT;
    if isolation.is_lpac() {
        unsafe {
            UpdateProcThreadAttribute(
                attributes.list,
                0,
                PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY as usize,
                Some(
                    (&raw const all_application_packages_policy)
                        .cast::<c_void>()
                        .cast_mut(),
                ),
                size_of_val(&all_application_packages_policy),
                None,
                None,
            )
        }
        .map_err(|error| windows_error("set LPAC process attribute", error))?;
    }

    let jobs = [job.handle()];
    unsafe {
        UpdateProcThreadAttribute(
            attributes.list,
            0,
            PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
            Some(jobs.as_ptr().cast::<c_void>()),
            size_of_val(&jobs),
            None,
            None,
        )
    }
    .map_err(|error| windows_error("set job-list process attribute", error))?;

    let inherited_handles = [stdin_handle, stdout_handle, stderr_handle];
    unsafe {
        UpdateProcThreadAttribute(
            attributes.list,
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            Some(inherited_handles.as_ptr().cast::<c_void>()),
            size_of_val(&inherited_handles),
            None,
            None,
        )
    }
    .map_err(|error| windows_error("set inherited-handle process attribute", error))?;

    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = u32::try_from(size_of::<STARTUPINFOEXW>())
        .map_err(|_| IrohaZipError::Sandbox("startup structure is too large".to_owned()))?;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = stdin_handle;
    startup.StartupInfo.hStdOutput = stdout_handle;
    startup.StartupInfo.hStdError = stderr_handle;
    startup.lpAttributeList = attributes.list;

    let application = wide_null(spec.program.as_os_str());
    let current_directory = wide_null(spec.current_dir.as_os_str());
    let program_units: Vec<u16> = spec.program.as_os_str().encode_wide().collect();
    let argument_units: Vec<Vec<u16>> = spec
        .args
        .iter()
        .map(|argument| argument.encode_wide().collect())
        .collect();
    let mut command_line = windows_command_line::encode(&program_units, &argument_units)
        .map_err(|error| IrohaZipError::Sandbox(format!("cannot encode command line: {error}")))?;
    // AppContainer path virtualization derives AC\Temp from the host-side
    // LOCALAPPDATA boundary. Supplying profile-local values for LOCALAPPDATA,
    // USERPROFILE, TEMP, or TMP applies that mapping twice and produces a
    // non-existent nested Packages\...\AC\Temp path.
    let environment = minimal_appcontainer_environment(&spec.program, sandbox_root)?;
    let mut process_info = PROCESS_INFORMATION::default();

    let create_result = unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            Some(PWSTR(command_line.as_mut_ptr())),
            None,
            None,
            true,
            EXTENDED_STARTUPINFO_PRESENT
                | CREATE_UNICODE_ENVIRONMENT
                | CREATE_NO_WINDOW
                | CREATE_SUSPENDED,
            Some(environment.as_ptr().cast::<c_void>()),
            PCWSTR(current_directory.as_ptr()),
            &raw const startup.StartupInfo,
            &raw mut process_info,
        )
    };

    for handle in [stdout_handle, stderr_handle, stdin_handle] {
        let _ = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0)) };
    }
    create_result.map_err(|error| windows_error("CreateProcessW", error))?;

    let process = OwnedHandle::new(process_info.hProcess);
    let thread_handle = OwnedHandle::new(process_info.hThread);

    let isolation_evidence = match verify_process_isolation(process.handle(), isolation) {
        Ok(evidence) => evidence,
        Err(error) => {
            let _ = unsafe { TerminateJobObject(job.handle(), 0xE000_0004) };
            let _ = unsafe { WaitForSingleObject(process.handle(), 5_000) };
            return Err(error);
        }
    };

    // The child is still suspended and its token has been positively checked.
    // A bounded write fits in the dedicated pipe buffer, so close-delimited EOF
    // is established without FlushFileBuffers waiting on child-side reads.
    if let Some((mut input, mut password)) = password_channel.take() {
        let write_result = input.write_all(password.line());
        drop(input);
        if let Err(error) = write_result {
            let _ = unsafe { TerminateJobObject(job.handle(), 0xE000_0007) };
            let _ = unsafe { WaitForSingleObject(process.handle(), 5_000) };
            return Err(IrohaZipError::io(
                "cannot write the one-use password channel",
                error,
            ));
        }
    }

    // The backend must not execute until the token and capability checks above
    // have positively established the requested isolation. CREATE_SUSPENDED
    // closes the interval between CreateProcessW and that fail-closed decision.
    let previous_suspend_count = unsafe { ResumeThread(thread_handle.handle()) };
    if previous_suspend_count == u32::MAX {
        let error = WindowsError::from_thread();
        let _ = unsafe { TerminateJobObject(job.handle(), 0xE000_0005) };
        let _ = unsafe { WaitForSingleObject(process.handle(), 5_000) };
        return Err(windows_error("ResumeThread", error));
    }
    if previous_suspend_count != 1 {
        let _ = unsafe { TerminateJobObject(job.handle(), 0xE000_0006) };
        let _ = unsafe { WaitForSingleObject(process.handle(), 5_000) };
        return Err(IrohaZipError::Sandbox(format!(
            "unexpected initial process suspend count: {previous_suspend_count}"
        )));
    }
    drop(thread_handle);

    wait_for_process(&process, &job, &spec, isolation_evidence)
}

fn create_noninheritable_pipe() -> Result<(OwnedHandle, OwnedHandle)> {
    let mut read = HANDLE::default();
    let mut write = HANDLE::default();
    unsafe {
        CreatePipe(
            &raw mut read,
            &raw mut write,
            None,
            PASSWORD_PIPE_BUFFER_BYTES,
        )
    }
    .map_err(|error| windows_error("CreatePipe(password channel)", error))?;
    let read = OwnedHandle::new(read);
    let write = OwnedHandle::new(write);
    for handle in [read.handle(), write.handle()] {
        unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0)) }
            .map_err(|error| windows_error("clear password pipe inheritance", error))?;
        let mut flags = 0u32;
        unsafe { GetHandleInformation(handle, &raw mut flags) }
            .map_err(|error| windows_error("verify password pipe inheritance", error))?;
        if flags & HANDLE_FLAG_INHERIT.0 != 0 {
            return Err(IrohaZipError::Sandbox(
                "password pipe handle remained inheritable".to_owned(),
            ));
        }
    }
    Ok((read, write))
}

fn verify_process_isolation(process: HANDLE, isolation: IsolationMode) -> Result<ProcessIsolation> {
    #[cfg(test)]
    if FORCE_ISOLATION_VERIFICATION_FAILURE.load(std::sync::atomic::Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(2));
        return Err(IrohaZipError::Sandbox(
            "forced isolation verification failure".to_owned(),
        ));
    }

    let mut token = HANDLE::default();
    unsafe {
        windows::Win32::System::Threading::OpenProcessToken(process, TOKEN_QUERY, &raw mut token)
    }
    .map_err(|error| windows_error("OpenProcessToken", error))?;
    let token = OwnedHandle::new(token);
    let is_app_container =
        query_token_flag(token.handle(), TokenIsAppContainer, "TokenIsAppContainer")? != 0;
    if !is_app_container {
        return Err(IrohaZipError::Sandbox(
            "created process does not have an AppContainer token".to_owned(),
        ));
    }
    // TokenIsLessPrivilegedAppContainer is not accepted for a regular AppContainer
    // token on every supported Windows build. Query it only when LPAC was requested;
    // that path remains fail-closed and must positively prove the stronger token mode.
    let is_lpac = if isolation.is_lpac() {
        query_token_flag(
            token.handle(),
            TokenIsLessPrivilegedAppContainer,
            "TokenIsLessPrivilegedAppContainer",
        )? != 0
    } else {
        false
    };
    if isolation.is_lpac() && !is_lpac {
        return Err(IrohaZipError::Sandbox(
            "LPAC was requested but the created process token is not less privileged".to_owned(),
        ));
    }
    let capability_count = query_token_capability_count(token.handle())?;
    if capability_count != 0 {
        return Err(IrohaZipError::Sandbox(format!(
            "created AppContainer token unexpectedly has {capability_count} capabilities"
        )));
    }
    Ok(ProcessIsolation {
        is_app_container,
        is_less_privileged_app_container: is_lpac,
        capability_count,
    })
}

pub fn require_current_process_appcontainer(allow_unsandboxed: bool) -> Result<()> {
    if allow_unsandboxed {
        return Ok(());
    }
    let process = unsafe { GetCurrentProcess() };
    let mut token = HANDLE::default();
    unsafe {
        windows::Win32::System::Threading::OpenProcessToken(process, TOKEN_QUERY, &raw mut token)
    }
    .map_err(|error| windows_error("OpenProcessToken(current process)", error))?;
    let token = OwnedHandle::new(token);
    if query_token_flag(token.handle(), TokenIsAppContainer, "TokenIsAppContainer")? == 0 {
        return Err(IrohaZipError::Sandbox(
            "internal archive listing refuses to load libarchive outside an AppContainer"
                .to_owned(),
        ));
    }
    let capability_count = query_token_capability_count(token.handle())?;
    if capability_count != 0 {
        return Err(IrohaZipError::Sandbox(format!(
            "internal archive listing requires zero capabilities, observed {capability_count}"
        )));
    }
    Ok(())
}

fn query_token_capability_count(token: HANDLE) -> Result<u32> {
    const MAX_TOKEN_INFORMATION_BYTES: u32 = 1024 * 1024;

    let mut required = 0u32;
    let _ = unsafe { GetTokenInformation(token, TokenCapabilities, None, 0, &raw mut required) };
    if required < u32::try_from(size_of::<u32>()).unwrap_or(u32::MAX)
        || required > MAX_TOKEN_INFORMATION_BYTES
    {
        return Err(IrohaZipError::Sandbox(format!(
            "unexpected token capability buffer size: {required}"
        )));
    }
    let buffer_bytes = required.max(u32::try_from(size_of::<TOKEN_GROUPS>()).unwrap_or(u32::MAX));
    let bytes = usize::try_from(buffer_bytes)
        .map_err(|_| IrohaZipError::Sandbox("token capability size overflow".to_owned()))?;
    let mut storage = vec![0usize; bytes.div_ceil(size_of::<usize>())];
    let mut returned = buffer_bytes;
    unsafe {
        GetTokenInformation(
            token,
            TokenCapabilities,
            Some(storage.as_mut_ptr().cast::<c_void>()),
            buffer_bytes,
            &raw mut returned,
        )
    }
    .map_err(|error| windows_error("query token capabilities", error))?;
    if returned > buffer_bytes {
        return Err(IrohaZipError::Sandbox(format!(
            "token capability query exceeded its buffer: {returned} > {buffer_bytes}"
        )));
    }
    let groups = unsafe { &*storage.as_ptr().cast::<TOKEN_GROUPS>() };
    Ok(groups.GroupCount)
}

fn query_token_flag(
    token: HANDLE,
    information_class: windows::Win32::Security::TOKEN_INFORMATION_CLASS,
    information_name: &str,
) -> Result<u32> {
    let mut value = 0u32;
    let mut returned = 0u32;
    unsafe {
        GetTokenInformation(
            token,
            information_class,
            Some((&raw mut value).cast::<c_void>()),
            u32::try_from(size_of_val(&value))
                .map_err(|_| IrohaZipError::Sandbox("token flag size overflow".to_owned()))?,
            &raw mut returned,
        )
    }
    .map_err(|error| windows_error(&format!("GetTokenInformation({information_name})"), error))?;
    if returned != u32::try_from(size_of_val(&value)).unwrap_or(u32::MAX) {
        return Err(IrohaZipError::Sandbox(format!(
            "unexpected {information_name} size: {returned}"
        )));
    }
    Ok(value)
}

fn run_unsandboxed(spec: ProcessSpec) -> Result<ProcessResult> {
    if spec.interactive_password.is_some() {
        return Err(IrohaZipError::Unsupported(
            "secure archive-password input refuses the unsandboxed fallback".to_owned(),
        ));
    }
    let stdout = File::create(&spec.stdout_log).map_err(|error| {
        IrohaZipError::io_path("cannot create process stdout log", &spec.stdout_log, error)
    })?;
    let stderr = File::create(&spec.stderr_log).map_err(|error| {
        IrohaZipError::io_path("cannot create process stderr log", &spec.stderr_log, error)
    })?;
    let stdin = open_child_stdin(&spec)?;

    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.current_dir)
        .env_clear()
        .envs(minimal_environment_pairs(
            &spec.program,
            &spec.current_dir,
            spec.temp_dir.as_deref(),
        ))
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .creation_flags(CREATE_NO_WINDOW_RAW);
    let mut child = command.spawn().map_err(|error| {
        IrohaZipError::io_path(
            "cannot start unsandboxed archive backend",
            &spec.program,
            error,
        )
    })?;

    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| IrohaZipError::io("cannot query archive backend", error))?
        {
            if let Some(root) = &spec.monitor_root {
                monitor::check_resource_limits(root, &spec.limits)?;
            }
            return Ok(ProcessResult {
                exit_code: status.code().unwrap_or(-1),
                isolation: ProcessIsolation::UNSANDBOXED,
            });
        }
        if started.elapsed() >= spec.timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(IrohaZipError::Sandbox(format!(
                "unsandboxed archive backend exceeded {:?}",
                spec.timeout
            )));
        }
        if let Some(root) = &spec.monitor_root
            && let Err(error) = monitor::check_resource_limits(root, &spec.limits)
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        thread::sleep(Duration::from_millis(200));
    }
}

fn open_child_stdin(spec: &ProcessSpec) -> Result<File> {
    let Some(path) = spec.stdin_file.as_deref() else {
        return OpenOptions::new()
            .read(true)
            .open("NUL")
            .map_err(|error| IrohaZipError::io("cannot open NUL for sandbox stdin", error));
    };
    validate_regular_file_security(path)?;
    let file = open_snapshot_source(path)?;
    validate_open_snapshot_source(path, &file)?;
    Ok(file)
}

fn wait_for_process(
    process: &OwnedHandle,
    job: &OwnedHandle,
    spec: &ProcessSpec,
    isolation: ProcessIsolation,
) -> Result<ProcessResult> {
    let started = Instant::now();
    loop {
        let wait = unsafe { WaitForSingleObject(process.handle(), 200) };
        if wait == WAIT_OBJECT_0 {
            let mut exit_code = 0u32;
            unsafe { GetExitCodeProcess(process.handle(), &raw mut exit_code) }
                .map_err(|error| windows_error("GetExitCodeProcess", error))?;
            if let Some(root) = &spec.monitor_root {
                monitor::check_resource_limits(root, &spec.limits)?;
            }
            return Ok(ProcessResult {
                exit_code: exit_code.cast_signed(),
                isolation,
            });
        }
        if wait != WAIT_TIMEOUT {
            let _ = unsafe { TerminateJobObject(job.handle(), 0xE000_0001) };
            return Err(IrohaZipError::Sandbox(format!(
                "WaitForSingleObject returned unexpected status {wait:?}"
            )));
        }
        if started.elapsed() >= spec.timeout {
            let _ = unsafe { TerminateJobObject(job.handle(), 0xE000_0002) };
            let _ = unsafe { WaitForSingleObject(process.handle(), 5_000) };
            return Err(IrohaZipError::Sandbox(format!(
                "archive backend exceeded {:?}",
                spec.timeout
            )));
        }
        if let Some(root) = &spec.monitor_root
            && let Err(error) = monitor::check_resource_limits(root, &spec.limits)
        {
            let _ = unsafe { TerminateJobObject(job.handle(), 0xE000_0003) };
            let _ = unsafe { WaitForSingleObject(process.handle(), 5_000) };
            return Err(error);
        }
    }
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn new(handle: HANDLE) -> Self {
        Self(handle)
    }

    fn handle(&self) -> HANDLE {
        self.0
    }

    fn into_file(mut self) -> File {
        let raw = self.0.0 as RawHandle;
        self.0 = HANDLE::default();
        unsafe { File::from_raw_handle(raw) }
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

struct AttributeList {
    list: LPPROC_THREAD_ATTRIBUTE_LIST,
    _storage: Vec<usize>,
}

impl AttributeList {
    fn new(count: u32) -> Result<Self> {
        let mut bytes = 0usize;
        let _ = unsafe { InitializeProcThreadAttributeList(None, count, None, &raw mut bytes) };
        if bytes == 0 {
            return Err(IrohaZipError::Sandbox(
                "InitializeProcThreadAttributeList returned zero bytes".to_owned(),
            ));
        }
        let words = bytes.div_ceil(size_of::<usize>());
        let mut storage = vec![0usize; words];
        let list = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr().cast::<c_void>());
        unsafe { InitializeProcThreadAttributeList(Some(list), count, None, &raw mut bytes) }
            .map_err(|error| windows_error("InitializeProcThreadAttributeList", error))?;
        Ok(Self {
            list,
            _storage: storage,
        })
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.list) };
    }
}

pub fn validate_directory_security(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        IrohaZipError::io_path("cannot inspect directory security", path, error)
    })?;
    reject_reparse(path, &metadata)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(IrohaZipError::Policy(format!(
            "not a real directory: {}",
            path.display()
        )));
    }
    reject_named_streams(path)?;
    Ok(())
}

pub fn validate_regular_file_security(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| IrohaZipError::io_path("cannot inspect input file", path, error))?;
    reject_reparse(path, &metadata)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(IrohaZipError::Policy(format!(
            "input is not a regular file: {}",
            path.display()
        )));
    }
    let info = file_information(path)?;
    if info.nNumberOfLinks != 1 {
        return Err(IrohaZipError::Policy(format!(
            "hard-linked input is rejected to avoid replacement races: {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn open_snapshot_source(path: &Path) -> Result<File> {
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0 | FILE_FLAG_SEQUENTIAL_SCAN.0)
        .open(path)
        .map_err(|error| IrohaZipError::io_path("cannot open snapshot source", path, error))
}

pub fn create_snapshot_target(path: &Path) -> Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_READ.0)
        .open(path)
        .map_err(|error| IrohaZipError::io_path("cannot create snapshot target", path, error))
}

pub fn validate_open_snapshot_source(path: &Path, file: &File) -> Result<()> {
    let metadata = file.metadata().map_err(|error| {
        IrohaZipError::io_path("cannot inspect open snapshot file", path, error)
    })?;
    reject_reparse(path, &metadata)?;
    if !metadata.is_file() {
        return Err(IrohaZipError::Policy(format!(
            "open snapshot source is not a regular file: {}",
            path.display()
        )));
    }
    let info = file_information_from_handle(path, file)?;
    if info.nNumberOfLinks != 1 {
        return Err(IrohaZipError::Policy(format!(
            "hard-linked snapshot source is rejected: {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn validate_extracted_entry_security(path: &Path, metadata: &Metadata) -> Result<()> {
    reject_reparse(path, metadata)?;
    reject_named_streams(path)?;
    if metadata.is_file() {
        let info = file_information(path)?;
        if info.nNumberOfLinks != 1 {
            return Err(IrohaZipError::Policy(format!(
                "hard-linked output is rejected: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

pub fn validate_post_handoff_entry_security(path: &Path, metadata: &Metadata) -> Result<()> {
    reject_reparse(path, metadata)?;
    reject_unexpected_post_handoff_streams(path, metadata.is_file())?;
    if metadata.is_file() {
        let info = file_information(path)?;
        if info.nNumberOfLinks != 1 {
            return Err(IrohaZipError::Policy(format!(
                "hard-linked output is rejected after Windows trust handoff: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

pub fn file_identity(path: &Path) -> Result<Option<FileIdentity>> {
    let info = file_information(path)?;
    Ok(Some(identity_from_file_information(&info)))
}

pub fn file_identity_from_handle(path: &Path, file: &File) -> Result<Option<FileIdentity>> {
    let info = file_information_from_handle(path, file)?;
    Ok(Some(identity_from_file_information(&info)))
}

fn identity_from_file_information(info: &BY_HANDLE_FILE_INFORMATION) -> FileIdentity {
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    FileIdentity {
        volume: u64::from(info.dwVolumeSerialNumber),
        index,
    }
}

fn reject_reparse(path: &Path, metadata: &Metadata) -> Result<()> {
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(IrohaZipError::Policy(format!(
            "NTFS reparse points are rejected: {}",
            path.display()
        )));
    }
    Ok(())
}

fn file_information(path: &Path) -> Result<BY_HANDLE_FILE_INFORMATION> {
    let file = File::open(path).map_err(|error| {
        IrohaZipError::io_path("cannot open file for identity check", path, error)
    })?;
    file_information_from_handle(path, &file)
}

fn file_information_from_handle(path: &Path, file: &File) -> Result<BY_HANDLE_FILE_INFORMATION> {
    file_information_from_raw_handle(path, raw_handle(file))
}

fn file_information_from_raw_handle(
    path: &Path,
    handle: HANDLE,
) -> Result<BY_HANDLE_FILE_INFORMATION> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(handle, &raw mut info) }
        .map_err(|error| windows_error_path("GetFileInformationByHandle", path, error))?;
    Ok(info)
}

fn reject_named_streams(path: &Path) -> Result<()> {
    reject_streams(path, false)
}

fn reject_unexpected_post_handoff_streams(path: &Path, allow_zone_identifier: bool) -> Result<()> {
    reject_streams(path, allow_zone_identifier)
}

fn reject_streams(path: &Path, allow_zone_identifier: bool) -> Result<()> {
    let mut data = WIN32_FIND_STREAM_DATA::default();
    let first = unsafe {
        FindFirstStreamW(
            PCWSTR(wide_null(path.as_os_str()).as_ptr()),
            FindStreamInfoStandard,
            (&raw mut data).cast::<c_void>(),
            None,
        )
    };
    let handle = match first {
        Ok(handle) => FindHandle(handle),
        Err(error)
            if is_windows_error(&error, ERROR_INVALID_PARAMETER.0)
                || is_windows_error(&error, ERROR_HANDLE_EOF.0)
                || is_windows_error(&error, ERROR_FILE_NOT_FOUND.0) =>
        {
            return Ok(());
        }
        Err(error) => {
            return Err(windows_error_path("FindFirstStreamW", path, error));
        }
    };

    loop {
        let name = wide_array_to_string(&data.cStreamName);
        let allowed = name == "::$DATA"
            || (allow_zone_identifier && name.eq_ignore_ascii_case(":Zone.Identifier:$DATA"));
        if !allowed {
            return Err(IrohaZipError::Policy(format!(
                "unexpected NTFS alternate data stream is rejected on {}: {name:?}",
                path.display()
            )));
        }
        match unsafe { FindNextStreamW(handle.0, (&raw mut data).cast::<c_void>()) } {
            Ok(()) => {}
            Err(error)
                if is_windows_error(&error, ERROR_HANDLE_EOF.0)
                    || is_windows_error(&error, ERROR_NO_MORE_FILES.0) =>
            {
                break;
            }
            Err(error) => return Err(windows_error_path("FindNextStreamW", path, error)),
        }
    }
    Ok(())
}

struct FindHandle(HANDLE);

impl Drop for FindHandle {
    fn drop(&mut self) {
        let _ = unsafe { FindClose(self.0) };
    }
}

pub fn read_mark_of_the_web(path: &Path) -> Result<Option<Vec<u8>>> {
    let Some(bytes) = read_zone_identifier_stream(path)? else {
        return Ok(None);
    };
    let zone = parse_zone_identifier(&bytes).unwrap_or(3);
    Ok(Some(
        format!("[ZoneTransfer]\r\nZoneId={zone}\r\n").into_bytes(),
    ))
}

fn read_zone_identifier_stream(path: &Path) -> Result<Option<Vec<u8>>> {
    let mut file = match File::open(zone_identifier_path(path)) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(IrohaZipError::io_path(
                "cannot read Mark-of-the-Web",
                path,
                error,
            ));
        }
    };
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(16 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| IrohaZipError::io_path("cannot read Mark-of-the-Web", path, error))?;
    if bytes.len() > 16 * 1024 || bytes.contains(&0) {
        return Err(IrohaZipError::Policy(format!(
            "invalid Mark-of-the-Web payload on {}",
            path.display()
        )));
    }
    Ok(Some(bytes))
}

fn parse_zone_identifier(bytes: &[u8]) -> Option<u8> {
    let text = std::str::from_utf8(bytes).ok()?;
    text.lines()
        .filter_map(|line| line.split_once('='))
        .find_map(|(key, value)| {
            if key.trim().eq_ignore_ascii_case("ZoneId") {
                value.trim().parse::<u8>().ok().filter(|zone| *zone <= 4)
            } else {
                None
            }
        })
}

pub fn write_mark_of_the_web(path: &Path, zone: &[u8]) -> Result<()> {
    if zone.len() > 16 * 1024 || zone.contains(&0) {
        return Err(IrohaZipError::Policy(
            "invalid Mark-of-the-Web payload".to_owned(),
        ));
    }
    let stream = zone_identifier_path(path);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&stream)
        .map_err(|error| IrohaZipError::io_path("cannot write Mark-of-the-Web", path, error))?;
    file.write_all(zone)
        .map_err(|error| IrohaZipError::io_path("cannot write Mark-of-the-Web", path, error))?;
    file.sync_all()
        .map_err(|error| IrohaZipError::io_path("cannot flush Mark-of-the-Web", path, error))?;
    Ok(())
}

pub fn verify_mark_of_the_web(path: &Path, expected: &[u8]) -> Result<()> {
    let expected_zone = parse_zone_identifier(expected).ok_or_else(|| {
        IrohaZipError::Policy("invalid expected Mark-of-the-Web payload".to_owned())
    })?;
    match read_zone_identifier_stream(path)? {
        Some(actual) if parse_zone_identifier(&actual) == Some(expected_zone) => Ok(()),
        Some(_) => Err(IrohaZipError::Policy(format!(
            "Mark-of-the-Web changed during Windows trust handoff: {}",
            path.display()
        ))),
        None => Err(IrohaZipError::Policy(format!(
            "Mark-of-the-Web disappeared during Windows trust handoff: {}",
            path.display()
        ))),
    }
}

pub fn open_folder(path: &Path) -> Result<()> {
    let windows_dir = std::env::var_os("WINDIR").unwrap_or_else(|| OsString::from(r"C:\Windows"));
    let explorer = PathBuf::from(windows_dir).join("explorer.exe");
    Command::new(&explorer)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| IrohaZipError::io_path("cannot open output directory", path, error))?;
    Ok(())
}

fn raw_handle(file: &File) -> HANDLE {
    HANDLE(file.as_raw_handle())
}

fn zone_identifier_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(":Zone.Identifier");
    PathBuf::from(value)
}

fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

unsafe fn os_string_from_pwstr(value: PWSTR) -> OsString {
    if value.is_null() {
        return OsString::new();
    }
    let mut length = 0usize;
    while unsafe { *value.0.add(length) } != 0 {
        length += 1;
    }
    OsString::from_wide(unsafe { std::slice::from_raw_parts(value.0, length) })
}

fn wide_array_to_string(value: &[u16]) -> String {
    let length = value
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length])
}

fn encode_environment(pairs: Vec<(OsString, OsString)>) -> Vec<u16> {
    let mut result = Vec::<u16>::new();
    for (key, value) in pairs {
        let mut entry = key;
        entry.push("=");
        entry.push(value);
        result.extend(entry.encode_wide());
        result.push(0);
    }
    result.push(0);
    result
}

struct AppContainerHostEnvironment {
    local_app_data: PathBuf,
    user_profile: PathBuf,
    temp: PathBuf,
}

fn minimal_appcontainer_environment(program: &Path, root: &Path) -> Result<Vec<u16>> {
    Ok(encode_environment(minimal_appcontainer_environment_pairs(
        program, root,
    )?))
}

fn minimal_appcontainer_environment_pairs(
    program: &Path,
    root: &Path,
) -> Result<Vec<(OsString, OsString)>> {
    let host = appcontainer_host_environment(root)?;
    let mut pairs = minimal_environment_pairs(program, root, Some(&host.temp));
    for (key, value) in &mut pairs {
        if key == "LOCALAPPDATA" {
            host.local_app_data.as_os_str().clone_into(value);
        } else if key == "USERPROFILE" {
            host.user_profile.as_os_str().clone_into(value);
        }
    }
    Ok(pairs)
}

fn appcontainer_host_environment(root: &Path) -> Result<AppContainerHostEnvironment> {
    let profile_root = root.parent().ok_or_else(|| {
        IrohaZipError::Sandbox(format!(
            "AppContainer root has no package profile parent: {}",
            root.display()
        ))
    })?;
    let packages = profile_root.parent().ok_or_else(|| {
        IrohaZipError::Sandbox(format!(
            "AppContainer root has no Packages parent: {}",
            root.display()
        ))
    })?;
    if !root
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("AC"))
        || !packages
            .file_name()
            .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("Packages"))
    {
        return Err(IrohaZipError::Sandbox(format!(
            "unexpected AppContainer storage layout: {}",
            root.display()
        )));
    }
    let local_app_data = packages.parent().ok_or_else(|| {
        IrohaZipError::Sandbox(format!(
            "AppContainer Packages directory has no LocalAppData parent: {}",
            packages.display()
        ))
    })?;
    let app_data = local_app_data.parent().ok_or_else(|| {
        IrohaZipError::Sandbox(format!(
            "LocalAppData directory has no AppData parent: {}",
            local_app_data.display()
        ))
    })?;
    let user_profile = app_data.parent().ok_or_else(|| {
        IrohaZipError::Sandbox(format!(
            "AppData directory has no user-profile parent: {}",
            app_data.display()
        ))
    })?;
    let temp = local_app_data.join("Temp");
    validate_directory_security(local_app_data)?;
    validate_directory_security(user_profile)?;
    validate_directory_security(&temp)?;
    Ok(AppContainerHostEnvironment {
        local_app_data: local_app_data.to_path_buf(),
        user_profile: user_profile.to_path_buf(),
        temp,
    })
}

fn minimal_environment_pairs(
    program: &Path,
    root: &Path,
    temp_dir: Option<&Path>,
) -> Vec<(OsString, OsString)> {
    let system_root = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("WINDIR"))
        .unwrap_or_else(|| OsString::from(r"C:\Windows"));
    let backend_dir = program.parent().unwrap_or(root).as_os_str().to_owned();
    let root_os = root.as_os_str().to_owned();
    let temp_os = temp_dir.unwrap_or(root).as_os_str().to_owned();
    vec![
        // Keep explicit UTF-8 locale hints for CRT/backend builds that honor
        // the environment. The prepared executable's activeCodePage manifest
        // is the process-wide Windows guarantee on supported OS versions.
        (OsString::from("LANG"), OsString::from(".utf8")),
        (OsString::from("LC_ALL"), OsString::from(".utf8")),
        (OsString::from("LOCALAPPDATA"), root_os.clone()),
        (OsString::from("PATH"), backend_dir),
        (OsString::from("SystemRoot"), system_root.clone()),
        (OsString::from("TEMP"), temp_os.clone()),
        (OsString::from("TMP"), temp_os),
        (OsString::from("USERPROFILE"), root_os),
        (OsString::from("WINDIR"), system_root),
    ]
}

fn windows_error(operation: &str, error: WindowsError) -> IrohaZipError {
    IrohaZipError::Sandbox(format!("{operation} failed: {error}"))
}

fn windows_error_path(operation: &str, path: &Path, error: WindowsError) -> IrohaZipError {
    IrohaZipError::Sandbox(format!(
        "{operation} failed for {}: {error}",
        path.display()
    ))
}

fn windows_status_error_path(operation: &str, path: &Path, status: u32) -> IrohaZipError {
    windows_error_path(
        operation,
        path,
        WindowsError::from_hresult(HRESULT::from_win32(status)),
    )
}

fn is_windows_error(error: &WindowsError, code: u32) -> bool {
    error.code() == HRESULT::from_win32(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "iroha-zip-windows-security-{}",
                util::unique_token()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct ForcedIsolationVerificationFailure;

    impl ForcedIsolationVerificationFailure {
        fn new() -> Self {
            assert!(
                !FORCE_ISOLATION_VERIFICATION_FAILURE
                    .swap(true, std::sync::atomic::Ordering::SeqCst),
                "forced isolation verification failure is already active"
            );
            Self
        }
    }

    impl Drop for ForcedIsolationVerificationFailure {
        fn drop(&mut self) {
            FORCE_ISOLATION_VERIFICATION_FAILURE.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[test]
    fn appcontainer_profile_deletion_retry_is_bounded_and_stops_on_success() {
        let mut attempts = 0usize;
        let mut delays = Vec::new();
        retry_appcontainer_profile_delete(
            || {
                attempts += 1;
                if attempts < 3 {
                    Err(WindowsError::from_hresult(HRESULT::from_win32(32)))
                } else {
                    Ok(())
                }
            },
            |delay| delays.push(delay),
        )
        .unwrap();
        assert_eq!(attempts, 3);
        assert_eq!(delays, vec![APP_CONTAINER_PROFILE_DELETE_RETRY_DELAY; 2]);

        let mut attempts = 0usize;
        let mut delay_count = 0usize;
        let error = retry_appcontainer_profile_delete(
            || {
                attempts += 1;
                Err(WindowsError::from_hresult(HRESULT::from_win32(32)))
            },
            |_| delay_count += 1,
        )
        .unwrap_err();
        assert_eq!(error.code(), HRESULT::from_win32(32));
        assert_eq!(attempts, APP_CONTAINER_PROFILE_DELETE_ATTEMPTS);
        assert_eq!(delay_count, APP_CONTAINER_PROFILE_DELETE_ATTEMPTS - 1);
    }

    #[test]
    fn appcontainer_process_stays_suspended_until_isolation_is_verified() {
        let sandbox = Sandbox::new(256, false, IsolationMode::AppContainer).unwrap();
        let root = sandbox.root().to_path_buf();
        let executable = root.join("suspended-verification-probe.exe");
        fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
        let stdout = root.join("suspended-verification-probe.stdout.log");
        let stderr = root.join("suspended-verification-probe.stderr.log");

        let forced_failure = ForcedIsolationVerificationFailure::new();
        let result = sandbox.run(ProcessSpec {
            program: executable,
            args: vec![OsString::from("--list")],
            current_dir: root.clone(),
            temp_dir: None,
            stdin_file: None,
            interactive_password: None,
            stdout_log: stdout.clone(),
            stderr_log: stderr,
            timeout: Duration::from_secs(5),
            monitor_root: None,
            limits: crate::policy::Limits::default(),
        });
        drop(forced_failure);

        let error = result.expect_err("forced verification must fail closed");
        assert!(
            error
                .to_string()
                .contains("forced isolation verification failure")
        );
        assert_eq!(
            fs::read(&stdout).unwrap(),
            b"",
            "a process rejected by isolation verification must never execute"
        );
        sandbox.cleanup().unwrap();
    }

    #[test]
    fn backend_environment_keeps_explicit_utf8_locale_hints() {
        let scratch = Path::new(r"C:\sandbox\Temp");
        let pairs = minimal_environment_pairs(
            Path::new(r"C:\backend\bsdtar.exe"),
            Path::new(r"C:\sandbox"),
            Some(scratch),
        );
        for key in ["LANG", "LC_ALL"] {
            let value = pairs
                .iter()
                .find_map(|(candidate, value)| (candidate == key).then_some(value))
                .expect("UTF-8 locale variable must be present");
            assert_eq!(value, ".utf8");
        }
        for key in ["TEMP", "TMP"] {
            let value = pairs
                .iter()
                .find_map(|(candidate, value)| (candidate == key).then_some(value))
                .expect("explicit process scratch variable must be present");
            assert_eq!(value, scratch.as_os_str());
        }
    }

    #[test]
    fn appcontainer_environment_uses_host_profile_boundaries() {
        let directory = TestDirectory::new();
        let user_profile = directory.0.join("User");
        let local_app_data = user_profile.join("AppData").join("Local");
        let host_temp = local_app_data.join("Temp");
        let root = local_app_data
            .join("Packages")
            .join("iroha-zip.test")
            .join("AC");
        fs::create_dir_all(&host_temp).unwrap();
        fs::create_dir_all(&root).unwrap();

        let pairs =
            minimal_appcontainer_environment_pairs(Path::new(r"C:\backend\bsdtar.exe"), &root)
                .unwrap();
        for (key, expected) in [
            ("LOCALAPPDATA", local_app_data.as_os_str()),
            ("USERPROFILE", user_profile.as_os_str()),
            ("TEMP", host_temp.as_os_str()),
            ("TMP", host_temp.as_os_str()),
        ] {
            let actual = pairs
                .iter()
                .find_map(|(candidate, value)| (candidate == key).then_some(value))
                .unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn backend_preparation_changes_only_the_disposable_copy_and_verifies_its_manifest() {
        let directory = TestDirectory::new();
        let original = std::env::current_exe().unwrap();
        let original_bytes = fs::read(&original).unwrap();
        let disposable = directory.0.join("prepared-backend.exe");
        fs::copy(&original, &disposable).unwrap();

        prepare_backend_executable(&disposable).unwrap();
        verify_utf8_backend_manifest(&disposable).unwrap();

        assert_eq!(fs::read(&original).unwrap(), original_bytes);
        assert_ne!(fs::read(&disposable).unwrap(), original_bytes);
    }

    #[test]
    fn zone_identifier_parser_requires_valid_utf8_and_a_bounded_zone() {
        assert_eq!(
            parse_zone_identifier(b"[ZoneTransfer]\r\nZoneId=3\r\n"),
            Some(3)
        );
        assert_eq!(parse_zone_identifier(b"ZoneId=5\r\n"), None);
        assert_eq!(parse_zone_identifier(b"ZoneId=not-a-number\r\n"), None);
        assert_eq!(parse_zone_identifier(b"\xffZoneId=3\r\n"), None);
    }

    #[test]
    fn post_handoff_validation_allows_only_zone_identifier_ads() {
        let directory = TestDirectory::new();
        let path = directory.0.join("item.txt");
        fs::write(&path, b"content").unwrap();
        let zone = b"[ZoneTransfer]\r\nZoneId=3\r\n";
        write_mark_of_the_web(&path, zone).unwrap();
        let metadata = fs::symlink_metadata(&path).unwrap();
        validate_post_handoff_entry_security(&path, &metadata).unwrap();
        verify_mark_of_the_web(&path, zone).unwrap();

        let mut unexpected = path.as_os_str().to_owned();
        unexpected.push(":unexpected");
        fs::write(PathBuf::from(unexpected), b"untrusted").unwrap();
        assert!(validate_post_handoff_entry_security(&path, &metadata).is_err());
    }

    #[test]
    fn directory_snapshot_enumerates_from_a_bounded_rename_blocking_handle() {
        let directory = TestDirectory::new();
        let source = directory.0.join("source");
        let moved = directory.0.join("moved");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("one.txt"), b"one").unwrap();
        fs::write(source.join("日本語.txt"), b"two").unwrap();

        let snapshot = DirectorySnapshot::open(&source).unwrap();
        assert!(snapshot.entries(1).is_err());
        assert_eq!(
            snapshot.entries(2).unwrap(),
            [OsString::from("one.txt"), OsString::from("日本語.txt")]
        );
        assert!(fs::rename(&source, &moved).is_err());
        drop(snapshot);
        fs::rename(&source, &moved).unwrap();
    }

    #[test]
    fn config_save_mutex_times_out_and_recovers() {
        let name = OsString::from(format!(
            r"Local\iroha-zip.ConfigSave.timeout.{}",
            util::unique_token()
        ));
        let (acquired_sender, acquired_receiver) = std::sync::mpsc::sync_channel(0);
        let (release_sender, release_receiver) = std::sync::mpsc::sync_channel(0);
        let owner_name = name.clone();
        let owner = std::thread::spawn(move || {
            let guard = lock_named_config_save(owner_name.as_os_str(), 5_000).unwrap();
            acquired_sender.send(()).unwrap();
            release_receiver.recv().unwrap();
            drop(guard);
        });
        acquired_receiver
            .recv_timeout(Duration::from_secs(5))
            .unwrap();

        let error = lock_named_config_save(name.as_os_str(), 10)
            .err()
            .expect("a held config mutex must time out");
        assert!(error.to_string().contains("timed out"));

        release_sender.send(()).unwrap();
        owner.join().unwrap();
        drop(lock_named_config_save(name.as_os_str(), 5_000).unwrap());
    }

    #[test]
    fn config_save_mutex_recovers_an_abandoned_owner() {
        let name = OsString::from(format!(
            r"Local\iroha-zip.ConfigSave.abandoned.{}",
            util::unique_token()
        ));
        let owner_name = name.clone();
        let abandoned_handle = std::thread::spawn(move || {
            let guard = lock_named_config_save(owner_name.as_os_str(), 5_000).unwrap();
            let raw_handle = guard.0.0 as usize;
            std::mem::forget(guard);
            raw_handle
        })
        .join()
        .unwrap();

        drop(lock_named_config_save(name.as_os_str(), 5_000).unwrap());
        let handle = HANDLE(abandoned_handle as *mut c_void);
        unsafe { CloseHandle(handle) }.unwrap();
    }
}
