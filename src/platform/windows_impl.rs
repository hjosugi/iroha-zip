#![allow(unsafe_code)]

use std::ffi::{OsStr, OsString, c_void};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::mem::{size_of, size_of_val};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::MetadataExt;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::ptr::null_mut;
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_HANDLE_EOF, ERROR_INVALID_PARAMETER,
    ERROR_NO_MORE_FILES, HANDLE, HANDLE_FLAG_INHERIT, HANDLE_FLAGS, HLOCAL, LocalFree,
    SetHandleInformation, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, GetAppContainerFolderPath,
};
use windows::Win32::Security::{FreeSid, PSID, SECURITY_CAPABILITIES};
use windows::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, FindClose, FindFirstStreamW,
    FindNextStreamW, FindStreamInfoStandard, GetFileInformationByHandle, WIN32_FIND_STREAM_DATA,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::JobObjects::{
    CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_JOB_MEMORY,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
};
use windows::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
    EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, InitializeProcThreadAttributeList,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_JOB_LIST, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
    PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOEXW, UpdateProcThreadAttribute,
    WaitForSingleObject,
};
use windows::core::{Error as WindowsError, HRESULT, PCWSTR, PWSTR};

use crate::error::{Result, SafeArcError};
use crate::monitor;
use crate::platform::{FileIdentity, ProcessResult, ProcessSpec};
use crate::util;

const CREATE_NO_WINDOW_RAW: u32 = 0x0800_0000;

pub struct Sandbox {
    root: PathBuf,
    memory_limit_bytes: usize,
    mode: Mode,
}

enum Mode {
    AppContainer { profile_name: String, sid: PSID },
    Unsandboxed,
}

impl Sandbox {
    pub fn new(memory_limit_mib: u64, allow_unsandboxed: bool) -> Result<Self> {
        let bytes = memory_limit_mib
            .checked_mul(1024 * 1024)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| SafeArcError::Config("sandbox memory limit is too large".to_owned()))?;
        if bytes < 64 * 1024 * 1024 {
            return Err(SafeArcError::Config(
                "sandbox memory limit must be at least 64 MiB".to_owned(),
            ));
        }

        match create_appcontainer() {
            Ok((profile_name, sid, root)) => Ok(Self {
                root,
                memory_limit_bytes: bytes,
                mode: Mode::AppContainer { profile_name, sid },
            }),
            Err(error) if allow_unsandboxed => {
                eprintln!(
                    "warning: AppContainer creation failed; explicit unsandboxed fallback is active: {error}"
                );
                let parent = std::env::temp_dir().join("safearc-unsandboxed");
                let root = util::create_unique_dir(&parent, "job-")?;
                Ok(Self {
                    root,
                    memory_limit_bytes: bytes,
                    mode: Mode::Unsandboxed,
                })
            }
            Err(error) => Err(error),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn run(&self, spec: ProcessSpec) -> Result<ProcessResult> {
        match &self.mode {
            Mode::AppContainer { sid, .. } => {
                run_in_appcontainer(*sid, self.memory_limit_bytes, spec)
            }
            Mode::Unsandboxed => run_unsandboxed(spec),
        }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        match &self.mode {
            Mode::AppContainer { profile_name, sid } => unsafe {
                let name = wide_null(OsStr::new(profile_name));
                let _ = DeleteAppContainerProfile(PCWSTR(name.as_ptr()));
                let _ = fs::remove_dir_all(&self.root);
                let _ = FreeSid(*sid);
            },
            Mode::Unsandboxed => {
                let _ = fs::remove_dir_all(&self.root);
            }
        }
    }
}

fn create_appcontainer() -> Result<(String, PSID, PathBuf)> {
    let token = util::unique_token().replace('-', "");
    let suffix_len = token.len().min(28);
    let profile_name = format!("SafeArc.Job.{}", &token[..suffix_len]);
    let name = wide_null(OsStr::new(&profile_name));
    let display = wide_null(OsStr::new("SafeArc extraction job"));
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

    let root_result = appcontainer_folder(sid);
    match root_result {
        Ok(root) => {
            fs::create_dir_all(&root).map_err(|error| {
                SafeArcError::io_path("cannot create AppContainer storage", &root, error)
            })?;
            validate_directory_security(&root)?;
            let resolved_root = fs::canonicalize(&root).map_err(|error| {
                SafeArcError::io_path("cannot resolve AppContainer storage", &root, error)
            })?;
            validate_directory_security(&resolved_root)?;
            Ok((profile_name, sid, resolved_root))
        }
        Err(error) => {
            unsafe {
                let _ = DeleteAppContainerProfile(PCWSTR(name.as_ptr()));
                let _ = FreeSid(sid);
            }
            Err(error)
        }
    }
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
    memory_limit_bytes: usize,
    spec: ProcessSpec,
) -> Result<ProcessResult> {
    let stdout = File::create(&spec.stdout_log).map_err(|error| {
        SafeArcError::io_path("cannot create process stdout log", &spec.stdout_log, error)
    })?;
    let stderr = File::create(&spec.stderr_log).map_err(|error| {
        SafeArcError::io_path("cannot create process stderr log", &spec.stderr_log, error)
    })?;
    let stdin = OpenOptions::new()
        .read(true)
        .open("NUL")
        .map_err(|error| SafeArcError::io("cannot open NUL for sandbox stdin", error))?;

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
        | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
    job_limits.BasicLimitInformation.ActiveProcessLimit = 1;
    job_limits.JobMemoryLimit = memory_limit_bytes;
    let job_limits_size = u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
        .map_err(|_| SafeArcError::Sandbox("job limits structure is too large".to_owned()))?;
    unsafe {
        SetInformationJobObject(
            job.handle(),
            JobObjectExtendedLimitInformation,
            (&raw const job_limits).cast::<c_void>(),
            job_limits_size,
        )
    }
    .map_err(|error| windows_error("SetInformationJobObject", error))?;

    let attributes = AttributeList::new(3)?;
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
        .map_err(|_| SafeArcError::Sandbox("startup structure is too large".to_owned()))?;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = stdin_handle;
    startup.StartupInfo.hStdOutput = stdout_handle;
    startup.StartupInfo.hStdError = stderr_handle;
    startup.lpAttributeList = attributes.list;

    let application = wide_null(spec.program.as_os_str());
    let current_directory = wide_null(spec.current_dir.as_os_str());
    let mut command_line = make_command_line(&spec.program, &spec.args);
    let environment = minimal_environment(&spec.program, &spec.current_dir);
    let mut process_info = PROCESS_INFORMATION::default();

    let create_result = unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            Some(PWSTR(command_line.as_mut_ptr())),
            None,
            None,
            true,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW,
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
    drop(thread_handle);

    wait_for_process(&process, &job, &spec)
}

fn run_unsandboxed(spec: ProcessSpec) -> Result<ProcessResult> {
    let stdout = File::create(&spec.stdout_log).map_err(|error| {
        SafeArcError::io_path("cannot create process stdout log", &spec.stdout_log, error)
    })?;
    let stderr = File::create(&spec.stderr_log).map_err(|error| {
        SafeArcError::io_path("cannot create process stderr log", &spec.stderr_log, error)
    })?;

    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.current_dir)
        .env_clear()
        .envs(minimal_environment_pairs(&spec.program, &spec.current_dir))
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .creation_flags(CREATE_NO_WINDOW_RAW);
    let mut child = command.spawn().map_err(|error| {
        SafeArcError::io_path(
            "cannot start unsandboxed archive backend",
            &spec.program,
            error,
        )
    })?;

    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| SafeArcError::io("cannot query archive backend", error))?
        {
            if let Some(root) = &spec.monitor_root {
                monitor::check_resource_limits(root, &spec.limits)?;
            }
            return Ok(ProcessResult {
                exit_code: status.code().unwrap_or(-1),
            });
        }
        if started.elapsed() >= spec.timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(SafeArcError::Sandbox(format!(
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

fn wait_for_process(
    process: &OwnedHandle,
    job: &OwnedHandle,
    spec: &ProcessSpec,
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
            });
        }
        if wait != WAIT_TIMEOUT {
            let _ = unsafe { TerminateJobObject(job.handle(), 0xE000_0001) };
            return Err(SafeArcError::Sandbox(format!(
                "WaitForSingleObject returned unexpected status {wait:?}"
            )));
        }
        if started.elapsed() >= spec.timeout {
            let _ = unsafe { TerminateJobObject(job.handle(), 0xE000_0002) };
            let _ = unsafe { WaitForSingleObject(process.handle(), 5_000) };
            return Err(SafeArcError::Sandbox(format!(
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
            return Err(SafeArcError::Sandbox(
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
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| SafeArcError::io_path("cannot inspect directory security", path, error))?;
    reject_reparse(path, &metadata)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(SafeArcError::Policy(format!(
            "not a real directory: {}",
            path.display()
        )));
    }
    reject_named_streams(path)?;
    Ok(())
}

pub fn validate_regular_file_security(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| SafeArcError::io_path("cannot inspect input file", path, error))?;
    reject_reparse(path, &metadata)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(SafeArcError::Policy(format!(
            "input is not a regular file: {}",
            path.display()
        )));
    }
    let info = file_information(path)?;
    if info.nNumberOfLinks != 1 {
        return Err(SafeArcError::Policy(format!(
            "hard-linked input is rejected to avoid replacement races: {}",
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
            return Err(SafeArcError::Policy(format!(
                "hard-linked output is rejected: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

pub fn file_identity(path: &Path) -> Result<Option<FileIdentity>> {
    let info = file_information(path)?;
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Ok(Some(FileIdentity {
        volume: u64::from(info.dwVolumeSerialNumber),
        index,
    }))
}

fn reject_reparse(path: &Path, metadata: &Metadata) -> Result<()> {
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(SafeArcError::Policy(format!(
            "NTFS reparse points are rejected: {}",
            path.display()
        )));
    }
    Ok(())
}

fn file_information(path: &Path) -> Result<BY_HANDLE_FILE_INFORMATION> {
    let file = File::open(path).map_err(|error| {
        SafeArcError::io_path("cannot open file for identity check", path, error)
    })?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(raw_handle(&file), &raw mut info) }
        .map_err(|error| windows_error_path("GetFileInformationByHandle", path, error))?;
    Ok(info)
}

fn reject_named_streams(path: &Path) -> Result<()> {
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
        if name != "::$DATA" {
            return Err(SafeArcError::Policy(format!(
                "NTFS alternate data stream is rejected on {}: {name:?}",
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
    let stream = zone_identifier_path(path);
    let mut file = match File::open(&stream) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(SafeArcError::io_path(
                "cannot read Mark-of-the-Web",
                path,
                error,
            ));
        }
    };
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(16 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|error| SafeArcError::io_path("cannot read Mark-of-the-Web", path, error))?;
    let text = String::from_utf8_lossy(&bytes);
    let zone = text
        .lines()
        .filter_map(|line| line.split_once('='))
        .find_map(|(key, value)| {
            if key.trim().eq_ignore_ascii_case("ZoneId") {
                value.trim().parse::<u8>().ok().filter(|zone| *zone <= 4)
            } else {
                None
            }
        })
        .unwrap_or(3);
    Ok(Some(
        format!("[ZoneTransfer]\r\nZoneId={zone}\r\n").into_bytes(),
    ))
}

pub fn write_mark_of_the_web(path: &Path, zone: &[u8]) -> Result<()> {
    if zone.len() > 16 * 1024 || zone.contains(&0) {
        return Err(SafeArcError::Policy(
            "invalid Mark-of-the-Web payload".to_owned(),
        ));
    }
    let stream = zone_identifier_path(path);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&stream)
        .map_err(|error| SafeArcError::io_path("cannot write Mark-of-the-Web", path, error))?;
    file.write_all(zone)
        .map_err(|error| SafeArcError::io_path("cannot write Mark-of-the-Web", path, error))?;
    file.sync_all()
        .map_err(|error| SafeArcError::io_path("cannot flush Mark-of-the-Web", path, error))?;
    Ok(())
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
        .map_err(|error| SafeArcError::io_path("cannot open output directory", path, error))?;
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

fn make_command_line(program: &Path, args: &[OsString]) -> Vec<u16> {
    let mut result = Vec::<u16>::new();
    append_quoted_argument(&mut result, program.as_os_str());
    for argument in args {
        result.push(u16::from(b' '));
        append_quoted_argument(&mut result, argument);
    }
    result.push(0);
    result
}

fn append_quoted_argument(output: &mut Vec<u16>, argument: &OsStr) {
    let units: Vec<u16> = argument.encode_wide().collect();
    let needs_quotes = units.is_empty()
        || units
            .iter()
            .any(|unit| matches!(*unit, 0x20 | 0x09 | 0x0a | 0x0b | 0x0c | 0x0d | 0x22));
    if !needs_quotes {
        output.extend(units);
        return;
    }

    output.push(u16::from(b'"'));
    let mut backslashes = 0usize;
    for unit in units {
        if unit == u16::from(b'\\') {
            backslashes += 1;
            continue;
        }
        if unit == u16::from(b'"') {
            output.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes * 2 + 1));
            output.push(unit);
            backslashes = 0;
            continue;
        }
        output.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes));
        backslashes = 0;
        output.push(unit);
    }
    output.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes * 2));
    output.push(u16::from(b'"'));
}

fn minimal_environment(program: &Path, root: &Path) -> Vec<u16> {
    let pairs = minimal_environment_pairs(program, root);
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

fn minimal_environment_pairs(program: &Path, root: &Path) -> Vec<(OsString, OsString)> {
    let system_root = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("WINDIR"))
        .unwrap_or_else(|| OsString::from(r"C:\Windows"));
    let backend_dir = program.parent().unwrap_or(root).as_os_str().to_owned();
    let root_os = root.as_os_str().to_owned();
    vec![
        (OsString::from("LOCALAPPDATA"), root_os.clone()),
        (OsString::from("PATH"), backend_dir),
        (OsString::from("SystemRoot"), system_root.clone()),
        (OsString::from("TEMP"), root_os.clone()),
        (OsString::from("TMP"), root_os.clone()),
        (OsString::from("USERPROFILE"), root_os),
        (OsString::from("WINDIR"), system_root),
    ]
}

fn windows_error(operation: &str, error: WindowsError) -> SafeArcError {
    SafeArcError::Sandbox(format!("{operation} failed: {error}"))
}

fn windows_error_path(operation: &str, path: &Path, error: WindowsError) -> SafeArcError {
    SafeArcError::Sandbox(format!(
        "{operation} failed for {}: {error}",
        path.display()
    ))
}

fn is_windows_error(error: &WindowsError, code: u32) -> bool {
    error.code() == HRESULT::from_win32(code)
}
