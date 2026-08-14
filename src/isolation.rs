use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{IrohaZipError, Result};

#[cfg(windows)]
const TIMEOUT_MILLISECONDS: u64 = 250;
#[cfg(windows)]
const SLEEP_MILLISECONDS: u64 = 5_000;
#[cfg(windows)]
const MEMORY_LIMIT_MIB: u64 = 64;
#[cfg(windows)]
const REQUESTED_MEMORY_MIB: u64 = 256;
const STAGING_FIXTURES: [&str; 7] = [
    "read.txt",
    "overwrite.txt",
    "append.txt",
    "rename.txt",
    "delete.txt",
    "permissions.txt",
    "nested.txt",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkProbeResult {
    pub schema_version: u32,
    pub endpoint: String,
    pub connected: bool,
    pub error_kind: Option<String>,
}

pub fn network_probe(endpoint: SocketAddr) -> NetworkProbeResult {
    match TcpStream::connect_timeout(&endpoint, Duration::from_secs(2)) {
        Ok(stream) => {
            drop(stream);
            NetworkProbeResult {
                schema_version: 1,
                endpoint: endpoint.to_string(),
                connected: true,
                error_kind: None,
            }
        }
        Err(error) => NetworkProbeResult {
            schema_version: 1,
            endpoint: endpoint.to_string(),
            connected: false,
            error_kind: Some(format!("{:?}", error.kind())),
        },
    }
}

pub fn memory_probe(bytes: u64) -> Result<()> {
    const MAX_PROBE_BYTES: u64 = 1024 * 1024 * 1024;

    if bytes == 0 || bytes > MAX_PROBE_BYTES {
        return Err(IrohaZipError::Usage(format!(
            "internal memory probe must request 1..={MAX_PROBE_BYTES} bytes"
        )));
    }
    let bytes = usize::try_from(bytes)
        .map_err(|_| IrohaZipError::Usage("memory probe size does not fit usize".to_owned()))?;
    let mut allocation = Vec::new();
    allocation.try_reserve_exact(bytes).map_err(|error| {
        IrohaZipError::Sandbox(format!("memory probe allocation was rejected: {error}"))
    })?;
    allocation.resize(bytes, 0u8);
    for offset in (0..bytes).step_by(4096) {
        allocation[offset] = 0xA5;
    }
    std::hint::black_box(&allocation);
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProcessTempProbeResult {
    pub schema_version: u32,
    pub temp_environment: String,
    pub tmp_environment: String,
    pub resolved_path: String,
    pub rng_succeeded: bool,
    pub delete_on_close_succeeded: bool,
}

#[cfg(windows)]
pub fn process_temp_probe() -> Result<ProcessTempProbeResult> {
    let observed = crate::platform::probe_process_temp()?;
    Ok(ProcessTempProbeResult {
        schema_version: 1,
        temp_environment: observed.temp_environment.to_string_lossy().into_owned(),
        tmp_environment: observed.tmp_environment.to_string_lossy().into_owned(),
        resolved_path: observed.resolved_path.to_string_lossy().into_owned(),
        rng_succeeded: true,
        delete_on_close_succeeded: true,
    })
}

#[cfg(not(windows))]
pub fn process_temp_probe() -> Result<ProcessTempProbeResult> {
    Err(IrohaZipError::Unsupported(
        "the process temp probe is available only on Windows".to_owned(),
    ))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StagingWriteProbeResult {
    pub schema_version: u32,
    pub readable_paths: Vec<String>,
    pub denied_operations: Vec<String>,
}

pub fn staging_write_probe(root: &Path) -> Result<StagingWriteProbeResult> {
    use std::fs::{self, OpenOptions};

    let expected = b"sealed staging source\n";
    let mut readable_paths = Vec::new();
    if fs::read(root.join("read.txt"))
        .map_err(|error| IrohaZipError::io("cannot read staging-write probe fixture", error))?
        == expected
    {
        readable_paths.push("root-file".to_owned());
    }
    if fs::read(root.join("nested").join("nested.txt"))
        .map_err(|error| IrohaZipError::io("cannot read nested staging-write fixture", error))?
        == expected
    {
        readable_paths.push("nested-file".to_owned());
    }
    let parent = root.parent().ok_or_else(|| {
        IrohaZipError::Sandbox("sealed staging root unexpectedly has no parent".to_owned())
    })?;
    let parent_entries = fs::read_dir(parent)
        .map_err(|error| IrohaZipError::io("cannot enumerate sealed staging parent", error))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|error| IrohaZipError::io("cannot enumerate a sealed parent entry", error))?;
    if parent_entries.len() == 1 && parent_entries[0].file_name() == "source" {
        readable_paths.push("parent-directory".to_owned());
    }
    let root_entries = fs::read_dir(root)
        .map_err(|error| IrohaZipError::io("cannot enumerate sealed staging root", error))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|error| IrohaZipError::io("cannot enumerate a sealed staging entry", error))?;
    if root_entries.len() == STAGING_FIXTURES.len() {
        readable_paths.push("root-directory".to_owned());
    }
    let nested_entries = fs::read_dir(root.join("nested"))
        .map_err(|error| IrohaZipError::io("cannot enumerate sealed nested directory", error))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|error| IrohaZipError::io("cannot enumerate a sealed nested entry", error))?;
    if nested_entries.len() == 1 {
        readable_paths.push("nested-directory".to_owned());
    }
    std::env::set_current_dir(root).map_err(|error| {
        IrohaZipError::io("cannot enter sealed staging root as archive backend", error)
    })?;
    let current_entries = fs::read_dir(".")
        .map_err(|error| IrohaZipError::io("cannot enumerate sealed current directory", error))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|error| IrohaZipError::io("cannot enumerate a sealed current entry", error))?;
    if current_entries.len() == STAGING_FIXTURES.len() {
        readable_paths.push("current-directory".to_owned());
    }

    let mut denied_operations = Vec::new();
    record_denial(
        &mut denied_operations,
        "overwrite-existing-file",
        OpenOptions::new()
            .write(true)
            .open(root.join("overwrite.txt")),
    )?;
    record_denial(
        &mut denied_operations,
        "append-existing-file",
        OpenOptions::new()
            .append(true)
            .open(root.join("append.txt")),
    )?;
    record_denial(
        &mut denied_operations,
        "create-root-file",
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("created.txt")),
    )?;
    record_denial(
        &mut denied_operations,
        "create-parent-file",
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(parent.join("created-parent.txt")),
    )?;
    record_denial(
        &mut denied_operations,
        "create-root-directory",
        fs::create_dir(root.join("created-directory")),
    )?;
    record_denial(
        &mut denied_operations,
        "overwrite-nested-file",
        OpenOptions::new()
            .write(true)
            .open(root.join("nested").join("nested.txt")),
    )?;
    record_denial(
        &mut denied_operations,
        "create-nested-file",
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("nested").join("created.txt")),
    )?;
    record_denial(
        &mut denied_operations,
        "rename-file",
        fs::rename(root.join("rename.txt"), root.join("renamed.txt")),
    )?;
    record_denial(
        &mut denied_operations,
        "delete-file",
        fs::remove_file(root.join("delete.txt")),
    )?;
    let mut permissions = fs::metadata(root.join("permissions.txt"))
        .map_err(|error| IrohaZipError::io("cannot inspect staging-write probe fixture", error))?
        .permissions();
    permissions.set_readonly(true);
    record_denial(
        &mut denied_operations,
        "change-file-attributes",
        fs::set_permissions(root.join("permissions.txt"), permissions),
    )?;
    let (dacl_change_denied, owner_change_denied) =
        crate::platform::probe_staging_security_write_denials(&root.join("permissions.txt"))?;
    if dacl_change_denied {
        denied_operations.push("open-dacl-for-write".to_owned());
    }
    if owner_change_denied {
        denied_operations.push("open-owner-for-write".to_owned());
    }

    Ok(StagingWriteProbeResult {
        schema_version: 1,
        readable_paths,
        denied_operations,
    })
}

fn record_denial<T>(
    operations: &mut Vec<String>,
    name: &str,
    result: std::io::Result<T>,
) -> Result<()> {
    match result {
        Ok(value) => {
            drop(value);
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            operations.push(name.to_owned());
            Ok(())
        }
        Err(error) => Err(IrohaZipError::Sandbox(format!(
            "staging-write probe failed for an unexpected reason: {error}"
        ))),
    }
}

#[cfg(windows)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenReport {
    is_app_container: bool,
    is_less_privileged_app_container: bool,
    capability_count: u32,
}

#[cfg(windows)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct NetworkIsolationReport {
    endpoint: String,
    denied: bool,
    error_kind: String,
}

#[cfg(windows)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimeoutIsolationReport {
    limit_milliseconds: u64,
    requested_sleep_milliseconds: u64,
    rejected: bool,
    error: String,
}

#[cfg(windows)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryIsolationReport {
    limit_mib: u64,
    requested_mib: u64,
    rejected: bool,
    exit_code: i32,
}

#[cfg(windows)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CrashIsolationReport {
    terminated_without_success: bool,
    exit_code: i32,
}

#[cfg(windows)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoaderFailureIsolationReport {
    create_process_rejected: bool,
}

#[cfg(windows)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct StagingWriteIsolationReport {
    acl_applied: bool,
    readable_paths: Vec<String>,
    denied_operations: Vec<String>,
}

#[cfg(windows)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupReport {
    profile_name: String,
    profile_delete_succeeded: bool,
    temporary_root_removed: bool,
}

#[cfg(windows)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IsolationReport {
    schema_version: u32,
    requested_mode: String,
    probe_sha256: String,
    token: TokenReport,
    network: NetworkIsolationReport,
    timeout: TimeoutIsolationReport,
    memory: MemoryIsolationReport,
    crash: CrashIsolationReport,
    loader_failure: LoaderFailureIsolationReport,
    process_temp: ProcessTempProbeResult,
    staging_write_seal: StagingWriteIsolationReport,
    cleanup: Vec<CleanupReport>,
}

#[cfg(windows)]
pub fn measure(config: &crate::config::Config) -> Result<IsolationReport> {
    use std::ffi::OsString;
    use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

    use crate::platform::ProcessSpec;
    use crate::util;

    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| IrohaZipError::io("cannot create isolation network probe", error))?;
    let endpoint = listener
        .local_addr()
        .map_err(|error| IrohaZipError::io("cannot identify isolation probe endpoint", error))?;
    let network_probe = PreparedProbe::new(config, config.sandbox.memory_limit_mib, "network")?;
    let stdout = network_probe.root.join("network-probe.stdout.json");
    let stderr = network_probe.root.join("network-probe.stderr.log");
    let operation = (|| {
        let result = network_probe.sandbox.run(ProcessSpec {
            program: network_probe.executable.clone(),
            args: vec![
                OsString::from("internal-network-probe"),
                OsString::from(endpoint.to_string()),
            ],
            current_dir: network_probe.root.clone(),
            temp_dir: None,
            stdin_file: None,
            stdout_log: stdout.clone(),
            stderr_log: stderr.clone(),
            timeout: Duration::from_secs(config.sandbox.timeout_seconds.clamp(1, 10)),
            monitor_root: None,
            limits: config.limits.clone(),
        })?;
        if result.exit_code != 0 {
            return Err(IrohaZipError::Sandbox(format!(
                "network probe exited with code {}: {}",
                result.exit_code,
                util::read_limited(&stderr, 16 * 1024)?
            )));
        }
        let raw_probe = util::read_limited(&stdout, 16 * 1024)?;
        let network: NetworkProbeResult = serde_json::from_str(&raw_probe).map_err(|error| {
            IrohaZipError::Sandbox(format!("cannot parse network probe result: {error}"))
        })?;
        if network.schema_version != 1 || network.endpoint != endpoint.to_string() {
            return Err(IrohaZipError::Sandbox(
                "network probe returned mismatched evidence".to_owned(),
            ));
        }
        if network.connected {
            return Err(IrohaZipError::Sandbox(format!(
                "zero-capability AppContainer unexpectedly connected to {endpoint}"
            )));
        }
        let error_kind = network.error_kind.ok_or_else(|| {
            IrohaZipError::Sandbox("denied network probe did not report an error kind".to_owned())
        })?;
        Ok((result, error_kind))
    })();
    drop(listener);
    let probe_hash = network_probe.sha256.clone();
    let ((network_result, network_error_kind), network_cleanup) =
        finish_probe(network_probe, operation)?;

    let timeout_probe = PreparedProbe::new(config, config.sandbox.memory_limit_mib, "timeout")?;
    let timeout_operation = match timeout_probe.sandbox.run(ProcessSpec {
        program: timeout_probe.executable.clone(),
        args: vec![
            OsString::from("internal-sleep-probe"),
            OsString::from(SLEEP_MILLISECONDS.to_string()),
        ],
        current_dir: timeout_probe.root.clone(),
        temp_dir: None,
        stdin_file: None,
        stdout_log: timeout_probe.root.join("timeout-probe.stdout.log"),
        stderr_log: timeout_probe.root.join("timeout-probe.stderr.log"),
        timeout: Duration::from_millis(TIMEOUT_MILLISECONDS),
        monitor_root: None,
        limits: config.limits.clone(),
    }) {
        Err(IrohaZipError::Sandbox(message)) if message.contains("exceeded") => {
            Ok(TimeoutIsolationReport {
                limit_milliseconds: TIMEOUT_MILLISECONDS,
                requested_sleep_milliseconds: SLEEP_MILLISECONDS,
                rejected: true,
                error: message,
            })
        }
        Err(error) => Err(IrohaZipError::Sandbox(format!(
            "timeout probe failed for an unexpected reason: {error}"
        ))),
        Ok(result) => Err(IrohaZipError::Sandbox(format!(
            "timeout probe unexpectedly exited with code {}",
            result.exit_code
        ))),
    };
    let (timeout, timeout_cleanup) = finish_probe(timeout_probe, timeout_operation)?;

    let memory_probe = PreparedProbe::new(config, MEMORY_LIMIT_MIB, "memory")?;
    let memory_result = memory_probe.sandbox.run(ProcessSpec {
        program: memory_probe.executable.clone(),
        args: vec![
            OsString::from("internal-memory-probe"),
            OsString::from((REQUESTED_MEMORY_MIB * 1024 * 1024).to_string()),
        ],
        current_dir: memory_probe.root.clone(),
        temp_dir: None,
        stdin_file: None,
        stdout_log: memory_probe.root.join("memory-probe.stdout.log"),
        stderr_log: memory_probe.root.join("memory-probe.stderr.log"),
        timeout: Duration::from_secs(10),
        monitor_root: None,
        limits: config.limits.clone(),
    });
    let memory_operation = memory_result.and_then(|result| {
        if result.exit_code == 0 {
            return Err(IrohaZipError::Sandbox(
                "memory probe unexpectedly exceeded its Job Object limit".to_owned(),
            ));
        }
        Ok((
            MemoryIsolationReport {
                limit_mib: MEMORY_LIMIT_MIB,
                requested_mib: REQUESTED_MEMORY_MIB,
                rejected: true,
                exit_code: result.exit_code,
            },
            result.isolation,
        ))
    });
    let ((memory, memory_isolation), memory_cleanup) =
        finish_probe(memory_probe, memory_operation)?;
    if memory_isolation != network_result.isolation {
        return Err(IrohaZipError::Sandbox(
            "isolation token evidence changed between probes".to_owned(),
        ));
    }

    let crash_probe = PreparedProbe::new(config, config.sandbox.memory_limit_mib, "crash")?;
    let crash_result = crash_probe.sandbox.run(ProcessSpec {
        program: crash_probe.executable.clone(),
        args: vec![OsString::from("internal-crash-probe")],
        current_dir: crash_probe.root.clone(),
        temp_dir: None,
        stdin_file: None,
        stdout_log: crash_probe.root.join("crash-probe.stdout.log"),
        stderr_log: crash_probe.root.join("crash-probe.stderr.log"),
        timeout: Duration::from_secs(config.sandbox.timeout_seconds.clamp(1, 10)),
        monitor_root: None,
        limits: config.limits.clone(),
    });
    let crash_operation = crash_result.and_then(|result| {
        if result.exit_code == 0 {
            return Err(IrohaZipError::Sandbox(
                "crash probe unexpectedly exited successfully".to_owned(),
            ));
        }
        Ok((
            CrashIsolationReport {
                terminated_without_success: true,
                exit_code: result.exit_code,
            },
            result.isolation,
        ))
    });
    let ((crash, crash_isolation), crash_cleanup) = finish_probe(crash_probe, crash_operation)?;
    if crash_isolation != network_result.isolation {
        return Err(IrohaZipError::Sandbox(
            "isolation token evidence changed during the crash probe".to_owned(),
        ));
    }

    let loader_probe =
        PreparedProbe::new(config, config.sandbox.memory_limit_mib, "loader-failure")?;
    let loader_operation = (|| {
        std::fs::write(&loader_probe.executable, b"not a Windows executable\n").map_err(
            |error| {
                IrohaZipError::io_path(
                    "cannot prepare invalid loader probe executable",
                    &loader_probe.executable,
                    error,
                )
            },
        )?;
        match loader_probe.sandbox.run(ProcessSpec {
            program: loader_probe.executable.clone(),
            args: Vec::new(),
            current_dir: loader_probe.root.clone(),
            temp_dir: None,
            stdin_file: None,
            stdout_log: loader_probe.root.join("loader-failure-probe.stdout.log"),
            stderr_log: loader_probe.root.join("loader-failure-probe.stderr.log"),
            timeout: Duration::from_secs(config.sandbox.timeout_seconds.clamp(1, 10)),
            monitor_root: None,
            limits: config.limits.clone(),
        }) {
            Err(IrohaZipError::Sandbox(message)) if message.contains("CreateProcessW") => {
                Ok(LoaderFailureIsolationReport {
                    create_process_rejected: true,
                })
            }
            Err(error) => Err(IrohaZipError::Sandbox(format!(
                "loader-failure probe failed for an unexpected reason: {error}"
            ))),
            Ok(result) => Err(IrohaZipError::Sandbox(format!(
                "invalid loader probe unexpectedly started and exited with code {}",
                result.exit_code
            ))),
        }
    })();
    let (loader_failure, loader_cleanup) = finish_probe(loader_probe, loader_operation)?;

    let temp_probe = PreparedProbe::new(config, config.sandbox.memory_limit_mib, "process-temp")?;
    let temp_operation = (|| {
        let scratch = temp_probe.sandbox.create_process_scratch()?;
        let stdout = temp_probe.root.join("process-temp-probe.stdout.json");
        let stderr = temp_probe.root.join("process-temp-probe.stderr.log");
        let result = temp_probe.sandbox.run(ProcessSpec {
            program: temp_probe.executable.clone(),
            args: vec![OsString::from("internal-process-temp-probe")],
            current_dir: temp_probe.root.clone(),
            temp_dir: Some(scratch.clone()),
            stdin_file: None,
            stdout_log: stdout.clone(),
            stderr_log: stderr.clone(),
            timeout: Duration::from_secs(config.sandbox.timeout_seconds.clamp(1, 10)),
            monitor_root: None,
            limits: config.limits.clone(),
        })?;
        if result.exit_code != 0 {
            return Err(IrohaZipError::Sandbox(format!(
                "process temp probe exited with code {}: {}",
                result.exit_code,
                util::read_limited(&stderr, 16 * 1024)?
            )));
        }
        let raw = util::read_limited(&stdout, 16 * 1024)?;
        let observed: ProcessTempProbeResult = serde_json::from_str(&raw).map_err(|error| {
            IrohaZipError::Sandbox(format!("cannot parse process temp probe result: {error}"))
        })?;
        if observed.schema_version != 1
            || observed.temp_environment.is_empty()
            || observed.tmp_environment.is_empty()
            || observed.resolved_path.is_empty()
            || !observed.rng_succeeded
            || !observed.delete_on_close_succeeded
        {
            return Err(IrohaZipError::Sandbox(format!(
                "process temp probe returned incomplete evidence: {observed:?}"
            )));
        }
        let observed_path = std::fs::canonicalize(&observed.resolved_path).map_err(|error| {
            IrohaZipError::io_path(
                "cannot resolve the child-reported process temp directory",
                Path::new(&observed.resolved_path),
                error,
            )
        })?;
        let expected_path = std::fs::canonicalize(&scratch).map_err(|error| {
            IrohaZipError::io_path(
                "cannot resolve the expected process temp directory",
                &scratch,
                error,
            )
        })?;
        if observed_path != expected_path {
            return Err(IrohaZipError::Sandbox(format!(
                "process temp resolved outside its dedicated scratch: observed={}; expected={}",
                observed_path.display(),
                expected_path.display()
            )));
        }
        temp_probe.sandbox.finish_process_scratch(&scratch)?;
        Ok((observed, result.isolation))
    })();
    let ((process_temp, temp_isolation), temp_cleanup) = finish_probe(temp_probe, temp_operation)?;
    if temp_isolation != network_result.isolation {
        return Err(IrohaZipError::Sandbox(
            "isolation token evidence changed during the process temp probe".to_owned(),
        ));
    }

    let staging_probe =
        PreparedProbe::new(config, config.sandbox.memory_limit_mib, "staging-write")?;
    let staging_operation = (|| {
        let staging_root = staging_probe.sandbox.staged_source_path();
        std::fs::create_dir(&staging_root).map_err(|error| {
            IrohaZipError::io_path(
                "cannot create staging-write probe directory",
                &staging_root,
                error,
            )
        })?;
        for name in STAGING_FIXTURES {
            let path = staging_root.join(name);
            std::fs::write(&path, b"sealed staging source\n").map_err(|error| {
                IrohaZipError::io_path("cannot create staging-write probe fixture", &path, error)
            })?;
        }
        let nested = staging_root.join("nested");
        std::fs::create_dir(&nested).map_err(|error| {
            IrohaZipError::io_path(
                "cannot create nested staging-write probe directory",
                &nested,
                error,
            )
        })?;
        std::fs::rename(staging_root.join("nested.txt"), nested.join("nested.txt")).map_err(
            |error| {
                IrohaZipError::io_path(
                    "cannot place nested staging-write probe fixture",
                    &nested,
                    error,
                )
            },
        )?;
        let seal_entry_budget = u64::try_from(STAGING_FIXTURES.len() + 1).map_err(|_| {
            IrohaZipError::Sandbox("staging-write probe entry budget overflow".to_owned())
        })?;
        let acl_applied = staging_probe
            .sandbox
            .seal_staged_source_tree(&staging_root, seal_entry_budget)?;
        if !acl_applied {
            return Err(IrohaZipError::Sandbox(
                "staging-write probe did not receive an AppContainer ACL".to_owned(),
            ));
        }
        let staging_stdout = staging_probe.root.join("staging-write-probe.stdout.json");
        let staging_stderr = staging_probe.root.join("staging-write-probe.stderr.log");
        let result = staging_probe.sandbox.run(ProcessSpec {
            program: staging_probe.executable.clone(),
            args: vec![
                OsString::from("internal-staging-write-probe"),
                staging_root.as_os_str().to_owned(),
            ],
            current_dir: staging_probe.root.clone(),
            temp_dir: None,
            stdin_file: None,
            stdout_log: staging_stdout.clone(),
            stderr_log: staging_stderr.clone(),
            timeout: Duration::from_secs(config.sandbox.timeout_seconds.clamp(1, 10)),
            monitor_root: None,
            limits: config.limits.clone(),
        })?;
        if result.exit_code != 0 {
            return Err(IrohaZipError::Sandbox(format!(
                "staging-write probe exited with code {}: {}",
                result.exit_code,
                util::read_limited(&staging_stderr, 16 * 1024)?
            )));
        }
        let raw = util::read_limited(&staging_stdout, 16 * 1024)?;
        let observed: StagingWriteProbeResult = serde_json::from_str(&raw).map_err(|error| {
            IrohaZipError::Sandbox(format!("cannot parse staging-write probe result: {error}"))
        })?;
        let expected = expected_staging_write_probe();
        if observed != expected {
            return Err(IrohaZipError::Sandbox(format!(
                "staging-write ACL did not satisfy the read-only contract: {observed:?}"
            )));
        }
        Ok((acl_applied, observed, result.isolation))
    })();
    let ((acl_applied, staging, staging_isolation), staging_cleanup) =
        finish_probe(staging_probe, staging_operation)?;
    if staging_isolation != network_result.isolation {
        return Err(IrohaZipError::Sandbox(
            "isolation token evidence changed during the staging-write probe".to_owned(),
        ));
    }

    Ok(IsolationReport {
        schema_version: 4,
        requested_mode: if config.sandbox.isolation.is_lpac() {
            "lpac"
        } else {
            "appcontainer"
        }
        .to_owned(),
        probe_sha256: probe_hash,
        token: TokenReport {
            is_app_container: network_result.isolation.is_app_container,
            is_less_privileged_app_container: network_result
                .isolation
                .is_less_privileged_app_container,
            capability_count: network_result.isolation.capability_count,
        },
        network: NetworkIsolationReport {
            endpoint: endpoint.to_string(),
            denied: true,
            error_kind: network_error_kind,
        },
        timeout,
        memory,
        crash,
        loader_failure,
        process_temp,
        staging_write_seal: StagingWriteIsolationReport {
            acl_applied,
            readable_paths: staging.readable_paths,
            denied_operations: staging.denied_operations,
        },
        cleanup: vec![
            network_cleanup,
            timeout_cleanup,
            memory_cleanup,
            crash_cleanup,
            loader_cleanup,
            temp_cleanup,
            staging_cleanup,
        ],
    })
}

#[cfg(windows)]
fn expected_staging_write_probe() -> StagingWriteProbeResult {
    StagingWriteProbeResult {
        schema_version: 1,
        readable_paths: [
            "root-file",
            "nested-file",
            "parent-directory",
            "root-directory",
            "nested-directory",
            "current-directory",
        ]
        .map(str::to_owned)
        .to_vec(),
        denied_operations: [
            "overwrite-existing-file",
            "append-existing-file",
            "create-root-file",
            "create-parent-file",
            "create-root-directory",
            "overwrite-nested-file",
            "create-nested-file",
            "rename-file",
            "delete-file",
            "change-file-attributes",
            "open-dacl-for-write",
            "open-owner-for-write",
        ]
        .map(str::to_owned)
        .to_vec(),
    }
}

#[cfg(windows)]
struct PreparedProbe {
    sandbox: crate::platform::Sandbox,
    root: std::path::PathBuf,
    profile_name: String,
    executable: std::path::PathBuf,
    sha256: String,
}

#[cfg(windows)]
impl PreparedProbe {
    fn new(config: &crate::config::Config, memory_limit_mib: u64, label: &str) -> Result<Self> {
        use std::fs;

        use crate::platform::Sandbox;
        use crate::{backend, util};

        let sandbox = Sandbox::new(memory_limit_mib, false, config.sandbox.isolation)?;
        let operation = (|| {
            let root = sandbox.root().to_path_buf();
            let profile_name = sandbox.profile_name().ok_or_else(|| {
                IrohaZipError::Sandbox(
                    "isolation report unexpectedly received an unsandboxed job".to_owned(),
                )
            })?;
            let profile_name = profile_name.to_owned();
            let current_executable = std::env::current_exe().map_err(|error| {
                IrohaZipError::io("cannot locate isolation probe executable", error)
            })?;
            let metadata = fs::symlink_metadata(&current_executable).map_err(|error| {
                IrohaZipError::io_path(
                    "cannot inspect isolation probe executable",
                    &current_executable,
                    error,
                )
            })?;
            crate::platform::validate_regular_file_security(&current_executable)?;
            let executable = root.join(format!("iroha-zip-{label}-probe.exe"));
            util::copy_file_new_exact(&current_executable, &executable, metadata.len())?;
            crate::platform::validate_regular_file_security(&executable)?;
            let source_hash = backend::sha256_file(&current_executable)?;
            let sha256 = backend::sha256_file(&executable)?;
            if source_hash != sha256 {
                return Err(IrohaZipError::Sandbox(
                    "copied isolation probe does not match the running executable".to_owned(),
                ));
            }
            Ok((root, profile_name, executable, sha256))
        })();
        match operation {
            Ok((root, profile_name, executable, sha256)) => Ok(Self {
                sandbox,
                root,
                profile_name,
                executable,
                sha256,
            }),
            Err(error) => sandbox.fail_after_cleanup(error),
        }
    }
}

#[cfg(windows)]
fn finish_probe<T>(probe: PreparedProbe, operation: Result<T>) -> Result<(T, CleanupReport)> {
    let root = probe.root.clone();
    let profile_name = probe.profile_name;
    let cleanup = probe.sandbox.cleanup();
    let root_removed = !root.exists();
    match (operation, cleanup, root_removed) {
        (Ok(value), Ok(()), true) => Ok((
            value,
            CleanupReport {
                profile_name,
                profile_delete_succeeded: true,
                temporary_root_removed: true,
            },
        )),
        (Err(operation), Ok(()), true) => Err(operation),
        (operation, cleanup, removed) => Err(IrohaZipError::Sandbox(format!(
            "probe operation/cleanup failed: operation={:?}; cleanup={:?}; temporary_root_removed={removed}",
            operation.err(),
            cleanup.err()
        ))),
    }
}

#[cfg(not(windows))]
pub fn measure(_config: &crate::config::Config) -> Result<serde_json::Value> {
    Err(IrohaZipError::Unsupported(
        "AppContainer isolation reports are available only on Windows".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

    use super::*;

    #[test]
    fn network_probe_reports_observed_success_and_failure() {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).unwrap();
        let endpoint = listener.local_addr().unwrap();
        let connected = network_probe(endpoint);
        assert_eq!(connected.schema_version, 1);
        assert_eq!(connected.endpoint, endpoint.to_string());
        assert!(connected.connected);
        assert_eq!(connected.error_kind, None);

        let denied = network_probe(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0).into());
        assert!(!denied.connected);
        assert!(denied.error_kind.is_some());
    }
}
