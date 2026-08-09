#![allow(unsafe_code)]

use std::ffi::{OsStr, OsString, c_void};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::mem::{size_of, size_of_val};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
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
    SetHandleInformation, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, GetAppContainerFolderPath,
};
use windows::Win32::Security::{
    FreeSid, GetTokenInformation, PSID, SECURITY_CAPABILITIES, TOKEN_QUERY, TokenIsAppContainer,
    TokenIsLessPrivilegedAppContainer,
};
use windows::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_FLAG_SEQUENTIAL_SCAN, FILE_SHARE_READ, FindClose, FindFirstStreamW, FindNextStreamW,
    FindStreamInfoStandard, GetFileInformationByHandle, WIN32_FIND_STREAM_DATA,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize,
};
use windows::Win32::System::JobObjects::{
    CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_JOB_MEMORY,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
};
use windows::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, CreateMutexW, CreateProcessW,
    DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess,
    InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_JOB_LIST, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
    PROCESS_INFORMATION, ReleaseMutex, STARTF_USESTDHANDLES, STARTUPINFOEXW,
    UpdateProcThreadAttribute, WaitForSingleObject,
};
use windows::Win32::System::WindowsProgramming::PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT;
use windows::Win32::UI::Shell::{AttachmentServices, IAttachmentExecute};
use windows::core::{Error as WindowsError, GUID, HRESULT, PCWSTR, PWSTR};

use crate::config::IsolationMode;
use crate::error::{IrohaZipError, Result};
use crate::monitor;
use crate::platform::{FileIdentity, ProcessResult, ProcessSpec};
use crate::util;
use crate::windows_command_line;

const CREATE_NO_WINDOW_RAW: u32 = 0x0800_0000;
static IROHA_ZIP_ATTACHMENT_CLIENT: GUID = GUID::from_u128(0x8d3f90af_f983_4c6f_86ce_79c192a9352a);

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
    let name = wide_null(OsStr::new(r"Local\iroha-zip.ConfigSave.v1"));
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }
        .map_err(|error| windows_error("CreateMutexW(config save)", error))?;
    let wait = unsafe { WaitForSingleObject(handle, 30_000) };
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
    memory_limit_bytes: usize,
    mode: Mode,
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
            Ok((profile_name, sid, root)) => Ok(Self {
                root,
                memory_limit_bytes: bytes,
                mode: Mode::AppContainer {
                    profile_name,
                    sid,
                    isolation,
                },
            }),
            Err(error) if allow_unsandboxed => {
                eprintln!(
                    "warning: AppContainer creation failed; explicit unsandboxed fallback is active: {error}"
                );
                let parent = std::env::temp_dir().join("iroha-zip-unsandboxed");
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
            Mode::AppContainer { sid, isolation, .. } => {
                run_in_appcontainer(*sid, *isolation, self.memory_limit_bytes, spec)
            }
            Mode::Unsandboxed => run_unsandboxed(spec),
        }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        match &self.mode {
            Mode::AppContainer {
                profile_name, sid, ..
            } => unsafe {
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

    let root_result = appcontainer_folder(sid);
    match root_result {
        Ok(root) => {
            fs::create_dir_all(&root).map_err(|error| {
                IrohaZipError::io_path("cannot create AppContainer storage", &root, error)
            })?;
            validate_directory_security(&root)?;
            let resolved_root = fs::canonicalize(&root).map_err(|error| {
                IrohaZipError::io_path("cannot resolve AppContainer storage", &root, error)
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
    isolation: IsolationMode,
    memory_limit_bytes: usize,
    spec: ProcessSpec,
) -> Result<ProcessResult> {
    let stdout = File::create(&spec.stdout_log).map_err(|error| {
        IrohaZipError::io_path("cannot create process stdout log", &spec.stdout_log, error)
    })?;
    let stderr = File::create(&spec.stderr_log).map_err(|error| {
        IrohaZipError::io_path("cannot create process stderr log", &spec.stderr_log, error)
    })?;
    let stdin = OpenOptions::new()
        .read(true)
        .open("NUL")
        .map_err(|error| IrohaZipError::io("cannot open NUL for sandbox stdin", error))?;

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

    if let Err(error) = verify_process_isolation(process.handle(), isolation) {
        let _ = unsafe { TerminateJobObject(job.handle(), 0xE000_0004) };
        let _ = unsafe { WaitForSingleObject(process.handle(), 5_000) };
        return Err(error);
    }

    wait_for_process(&process, &job, &spec)
}

fn verify_process_isolation(process: HANDLE, isolation: IsolationMode) -> Result<()> {
    let mut token = HANDLE::default();
    unsafe {
        windows::Win32::System::Threading::OpenProcessToken(process, TOKEN_QUERY, &raw mut token)
    }
    .map_err(|error| windows_error("OpenProcessToken", error))?;
    let token = OwnedHandle::new(token);
    if query_token_flag(token.handle(), TokenIsAppContainer)? == 0 {
        return Err(IrohaZipError::Sandbox(
            "created process does not have an AppContainer token".to_owned(),
        ));
    }
    if isolation.is_lpac()
        && query_token_flag(token.handle(), TokenIsLessPrivilegedAppContainer)? == 0
    {
        return Err(IrohaZipError::Sandbox(
            "LPAC was requested but the created process token is not less privileged".to_owned(),
        ));
    }
    Ok(())
}

fn query_token_flag(
    token: HANDLE,
    information_class: windows::Win32::Security::TOKEN_INFORMATION_CLASS,
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
    .map_err(|error| windows_error("GetTokenInformation", error))?;
    if returned != u32::try_from(size_of_val(&value)).unwrap_or(u32::MAX) {
        return Err(IrohaZipError::Sandbox(format!(
            "unexpected token flag size: {returned}"
        )));
    }
    Ok(value)
}

fn run_unsandboxed(spec: ProcessSpec) -> Result<ProcessResult> {
    let stdout = File::create(&spec.stdout_log).map_err(|error| {
        IrohaZipError::io_path("cannot create process stdout log", &spec.stdout_log, error)
    })?;
    let stderr = File::create(&spec.stderr_log).map_err(|error| {
        IrohaZipError::io_path("cannot create process stderr log", &spec.stderr_log, error)
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
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Ok(Some(FileIdentity {
        volume: u64::from(info.dwVolumeSerialNumber),
        index,
    }))
}

pub fn file_identity_from_handle(path: &Path, file: &File) -> Result<Option<FileIdentity>> {
    let info = file_information_from_handle(path, file)?;
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Ok(Some(FileIdentity {
        volume: u64::from(info.dwVolumeSerialNumber),
        index,
    }))
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
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(raw_handle(file), &raw mut info) }
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

fn windows_error(operation: &str, error: WindowsError) -> IrohaZipError {
    IrohaZipError::Sandbox(format!("{operation} failed: {error}"))
}

fn windows_error_path(operation: &str, path: &Path, error: WindowsError) -> IrohaZipError {
    IrohaZipError::Sandbox(format!(
        "{operation} failed for {}: {error}",
        path.display()
    ))
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
}
