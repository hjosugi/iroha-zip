use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{IrohaZipError, Result};

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

    const TIMEOUT_MILLISECONDS: u64 = 250;
    const SLEEP_MILLISECONDS: u64 = 5_000;
    let timeout_probe = PreparedProbe::new(config, config.sandbox.memory_limit_mib, "timeout")?;
    let timeout_operation = match timeout_probe.sandbox.run(ProcessSpec {
        program: timeout_probe.executable.clone(),
        args: vec![
            OsString::from("internal-sleep-probe"),
            OsString::from(SLEEP_MILLISECONDS.to_string()),
        ],
        current_dir: timeout_probe.root.clone(),
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

    const MEMORY_LIMIT_MIB: u64 = 64;
    const REQUESTED_MEMORY_MIB: u64 = 256;
    let memory_probe = PreparedProbe::new(config, MEMORY_LIMIT_MIB, "memory")?;
    let memory_result = memory_probe.sandbox.run(ProcessSpec {
        program: memory_probe.executable.clone(),
        args: vec![
            OsString::from("internal-memory-probe"),
            OsString::from((REQUESTED_MEMORY_MIB * 1024 * 1024).to_string()),
        ],
        current_dir: memory_probe.root.clone(),
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

    Ok(IsolationReport {
        schema_version: 1,
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
        cleanup: vec![network_cleanup, timeout_cleanup, memory_cleanup],
    })
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
