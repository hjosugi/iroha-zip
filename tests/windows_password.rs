#![cfg(windows)]

use std::ffi::OsString;
use std::fs;
use std::time::Duration;

use iroha_zip::config::IsolationMode;
use iroha_zip::error::Result;
use iroha_zip::password::ArchivePassword;
use iroha_zip::platform::{ProcessResult, ProcessSpec, Sandbox, prepare_backend_executable};
use iroha_zip::policy::Limits;

const EXPECTED_PASSWORD: &str = "日本語-password-probe";

struct ProbeOutcome {
    result: Result<ProcessResult>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_probe(mode: &str, password: &str, timeout: Duration) -> ProbeOutcome {
    let sandbox = Sandbox::new(256, false, IsolationMode::AppContainer).unwrap();
    let root = sandbox.root().to_path_buf();
    let program_directory = root.join("password-probe-program");
    fs::create_dir(&program_directory).unwrap();
    let executable = program_directory.join("password-probe.exe");
    fs::copy(env!("CARGO_BIN_EXE_iroha-zip"), &executable).unwrap();
    prepare_backend_executable(&executable).unwrap();
    assert!(sandbox.seal_sandbox_tree(&program_directory, 1).unwrap());

    let stdout_path = root.join("password-probe.stdout.log");
    let stderr_path = root.join("password-probe.stderr.log");
    let secret = ArchivePassword::from_utf16(password.encode_utf16().collect()).unwrap();
    let result = sandbox.run(ProcessSpec {
        program: executable,
        args: vec![
            OsString::from("internal-password-probe"),
            OsString::from(mode),
        ],
        current_dir: root.clone(),
        temp_dir: None,
        stdin_file: None,
        interactive_password: Some(secret),
        stdout_log: stdout_path.clone(),
        stderr_log: stderr_path.clone(),
        timeout,
        monitor_root: None,
        limits: Limits::default(),
    });
    let stdout = fs::read(&stdout_path).unwrap_or_default();
    let stderr = fs::read(&stderr_path).unwrap_or_default();
    sandbox.cleanup().unwrap();
    assert!(!root.exists());
    ProbeOutcome {
        result,
        stdout,
        stderr,
    }
}

fn assert_secret_absent(outcome: &ProbeOutcome, secret: &str) {
    let secret = secret.as_bytes();
    assert!(
        !outcome
            .stdout
            .windows(secret.len())
            .any(|bytes| bytes == secret)
    );
    assert!(
        !outcome
            .stderr
            .windows(secret.len())
            .any(|bytes| bytes == secret)
    );
}

#[test]
fn conpty_delivers_one_non_ascii_password_without_logging_it() {
    let outcome = run_probe("accept", EXPECTED_PASSWORD, Duration::from_secs(10));
    let result = outcome.result.as_ref().unwrap();
    assert_eq!(result.exit_code, 0);
    assert!(result.isolation.is_app_container);
    assert_eq!(result.isolation.capability_count, 0);
    assert!(outcome.stdout.is_empty());
    assert_eq!(outcome.stderr, b"Enter passphrase:");
    assert_secret_absent(&outcome, EXPECTED_PASSWORD);
}

#[test]
fn conpty_rejects_wrong_password_retry_and_bounds_failure_paths() {
    let wrong = run_probe("repeat", "wrong-password", Duration::from_secs(10));
    let wrong_error = wrong.result.as_ref().unwrap_err().to_string();
    assert!(wrong_error.contains("automatic retries are forbidden"));
    assert_secret_absent(&wrong, "wrong-password");

    let timeout = run_probe("sleep", EXPECTED_PASSWORD, Duration::from_millis(500));
    assert!(
        timeout
            .result
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("exceeded")
    );
    assert_secret_absent(&timeout, EXPECTED_PASSWORD);

    let overflow = run_probe("overflow", EXPECTED_PASSWORD, Duration::from_secs(10));
    assert!(
        overflow
            .result
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("pseudoconsole output exceeded")
    );
    assert_secret_absent(&overflow, EXPECTED_PASSWORD);

    let crash = run_probe("crash", EXPECTED_PASSWORD, Duration::from_secs(10));
    assert_ne!(crash.result.as_ref().unwrap().exit_code, 0);
    assert_secret_absent(&crash, EXPECTED_PASSWORD);
}
