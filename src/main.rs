#![deny(unsafe_code)]

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::fs;
use std::process::ExitCode;
#[cfg(windows)]
use std::process::Stdio;
#[cfg(windows)]
use std::time::Duration;

use clap::Parser;
use iroha_zip::backend::BackendBundle;
use iroha_zip::backend_evidence::BackendEvidence;
use iroha_zip::cli::{Cli, Command, PasswordProbeMode};
#[cfg(windows)]
use iroha_zip::config::AttachmentHandoffPolicy;
use iroha_zip::config::{Config, default_config_path};
use iroha_zip::create;
use iroha_zip::error::IrohaZipError;
use iroha_zip::error::Result;
use iroha_zip::extract::{self, ExtractRequest};
use iroha_zip::password::{PasswordPreparation, prepare_password};
#[cfg(windows)]
use iroha_zip::platform::{AttachmentHandoffSession, ProcessIsolation, ProcessSpec, Sandbox};
use iroha_zip::preview::{self, PreviewRequest};
#[cfg(windows)]
use iroha_zip::snapshot::AuditedFile;
#[cfg(windows)]
use iroha_zip::util;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("iroha-zip: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config_path = match cli.config {
        Some(path) => path,
        None => default_config_path()?,
    };

    match cli.command {
        Command::ConfigPath => {
            println!("{}", config_path.display());
        }
        Command::Settings => {
            open_settings(&config_path)?;
        }
        Command::InitConfig => {
            if Config::write_default(&config_path)? {
                println!("created {}", config_path.display());
            } else {
                println!("already exists: {}", config_path.display());
            }
        }
        Command::Doctor => {
            let config = Config::load(&config_path)?;
            let backend_dir = config.backend_directory()?;
            let backend = BackendBundle::verify(&backend_dir)?;
            println!("configuration: {}", config_path.display());
            println!("backend:       {}", backend.root().display());
            println!("executable:    {}", backend.executable().display());
            println!("backend files: verified by SHA-256 manifest");
            let evidence_root = BackendEvidence::directory_for(backend.root());
            if evidence_root.exists() {
                let evidence = BackendEvidence::verify(&backend)?;
                print_evidence(&evidence);
                if !evidence.is_supported() {
                    eprintln!(
                        "WARNING: backend source is unsupported and was accepted explicitly; provenance is not verified"
                    );
                }
            } else {
                eprintln!(
                    "WARNING: backend provenance/SBOM/license evidence is missing; re-import the backend"
                );
            }
            #[cfg(windows)]
            {
                let (version, isolation) = probe_backend_in_sandbox(&backend, &config)?;
                println!("version:       {version}");
                println!(
                    "isolation:     requested={}; AppContainer={}; LPAC={}; capabilities={}; backend execution succeeded",
                    config.sandbox.isolation.display_name(),
                    isolation.is_app_container,
                    isolation.is_less_privileged_app_container,
                    isolation.capability_count
                );
                report_attachment_handoff_diagnostic(config.behavior.attachment_handoff)?;
            }
            #[cfg(not(windows))]
            println!("AppContainer:  unavailable; backend execution was not attempted");
        }
        Command::VerifyBackendEvidence {
            backend,
            require_supported,
        } => {
            let backend = BackendBundle::verify(&backend)?;
            let evidence = BackendEvidence::verify(&backend)?;
            if require_supported && !evidence.is_supported() {
                return Err(iroha_zip::error::IrohaZipError::Backend(
                    "backend evidence identifies an unsupported source; omit --require-supported only after explicit review"
                        .to_owned(),
                ));
            }
            print_evidence(&evidence);
            if !evidence.is_supported() {
                eprintln!(
                    "WARNING: backend source is unsupported and was accepted explicitly; provenance is not verified"
                );
            }
        }
        Command::IsolationReport => {
            let config = Config::load(&config_path)?;
            let report = iroha_zip::isolation::measure(&config)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|error| {
                    iroha_zip::error::IrohaZipError::Sandbox(format!(
                        "cannot serialize isolation report: {error}"
                    ))
                })?
            );
        }
        Command::InternalNetworkProbe { endpoint } => {
            let report = iroha_zip::isolation::network_probe(endpoint);
            println!(
                "{}",
                serde_json::to_string(&report).map_err(|error| {
                    iroha_zip::error::IrohaZipError::Sandbox(format!(
                        "cannot serialize internal network probe: {error}"
                    ))
                })?
            );
        }
        Command::InternalSleepProbe { milliseconds } => {
            if milliseconds > 60_000 {
                return Err(iroha_zip::error::IrohaZipError::Usage(
                    "internal sleep probe is bounded to 60000 milliseconds".to_owned(),
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(milliseconds));
        }
        Command::InternalMemoryProbe { bytes } => {
            iroha_zip::isolation::memory_probe(bytes)?;
        }
        Command::InternalCrashProbe => {
            std::process::abort();
        }
        Command::InternalPasswordProbe { mode } => {
            run_password_probe(mode)?;
        }
        Command::InternalProcessTempProbe => {
            let report = iroha_zip::isolation::process_temp_probe()?;
            println!(
                "{}",
                serde_json::to_string(&report).map_err(|error| {
                    iroha_zip::error::IrohaZipError::Sandbox(format!(
                        "cannot serialize internal process temp probe: {error}"
                    ))
                })?
            );
        }
        Command::InternalStagingWriteProbe { root } => {
            let report = iroha_zip::isolation::staging_write_probe(&root)?;
            println!(
                "{}",
                serde_json::to_string(&report).map_err(|error| {
                    iroha_zip::error::IrohaZipError::Sandbox(format!(
                        "cannot serialize internal staging-write probe: {error}"
                    ))
                })?
            );
        }
        Command::InternalArchiveListing {
            backend_root,
            candidates,
            archive,
            encoding,
            max_entries,
            max_path_bytes,
            allow_unsandboxed,
        } => {
            #[cfg(windows)]
            iroha_zip::platform::write_utf8_archive_listing(
                &backend_root,
                &candidates,
                &archive,
                encoding,
                max_entries,
                max_path_bytes,
                allow_unsandboxed,
            )?;
            #[cfg(not(windows))]
            {
                let _ = (
                    backend_root,
                    candidates,
                    archive,
                    encoding,
                    max_entries,
                    max_path_bytes,
                    allow_unsandboxed,
                );
                return Err(iroha_zip::error::IrohaZipError::Unsupported(
                    "the internal UTF-8 archive lister is only available on Windows".to_owned(),
                ));
            }
        }
        Command::InternalPasswordArchiveExtraction {
            backend_root,
            candidates,
            archive,
            output,
            encoding,
            max_files,
            max_directories,
            max_total_bytes,
            max_single_file_bytes,
            max_depth,
            max_path_bytes,
        } => {
            #[cfg(windows)]
            iroha_zip::platform::extract_password_archive(
                &backend_root,
                &candidates,
                &archive,
                &output,
                encoding,
                &iroha_zip::policy::Limits {
                    max_archive_bytes: u64::MAX,
                    max_files,
                    max_directories,
                    max_total_bytes,
                    max_single_file_bytes,
                    max_depth,
                    max_path_bytes,
                },
            )?;
            #[cfg(not(windows))]
            {
                let _ = (
                    backend_root,
                    candidates,
                    archive,
                    output,
                    encoding,
                    max_files,
                    max_directories,
                    max_total_bytes,
                    max_single_file_bytes,
                    max_depth,
                    max_path_bytes,
                );
                return Err(iroha_zip::error::IrohaZipError::Unsupported(
                    "the internal password archive extractor is only available on Windows"
                        .to_owned(),
                ));
            }
        }
        Command::InternalRawArchive {
            backend_root,
            candidates,
            archive,
            filter,
            output_name,
            max_bytes,
            output,
            allow_unsandboxed,
        } => {
            #[cfg(windows)]
            iroha_zip::platform::process_raw_archive(
                &backend_root,
                &candidates,
                &archive,
                filter,
                &output_name,
                max_bytes,
                output.as_deref(),
                allow_unsandboxed,
            )?;
            #[cfg(not(windows))]
            {
                let _ = (
                    backend_root,
                    candidates,
                    archive,
                    filter,
                    output_name,
                    max_bytes,
                    output,
                    allow_unsandboxed,
                );
                return Err(iroha_zip::error::IrohaZipError::Unsupported(
                    "the internal raw-stream extractor is only available on Windows".to_owned(),
                ));
            }
        }
        Command::Preview {
            archive,
            encoding,
            prompt_password,
            allow_unsandboxed,
        } => {
            let config = Config::load(&config_path)?;
            let backend = BackendBundle::verify(&config.backend_directory()?)?;
            let encoding = encoding.unwrap_or(config.behavior.default_filename_encoding);
            let password = match prepare_password(prompt_password, || {
                iroha_zip::platform::prompt_archive_password(&archive)
            })? {
                PasswordPreparation::Ready(password) => password,
                PasswordPreparation::Cancelled => return Ok(()),
            };
            let result = preview::preview(PreviewRequest {
                backend: &backend,
                config: &config,
                archive: &archive,
                encoding,
                password,
                allow_unsandboxed,
            })?;
            for entry in &result.entries {
                println!(
                    "{}\t{}\t{}",
                    entry.kind.display_name(),
                    entry.size,
                    entry.path.display()
                );
            }
            eprintln!(
                "previewed {} files, {} directories ({} bytes); nothing was published",
                result.summary.files, result.summary.directories, result.summary.total_bytes
            );
        }
        Command::Extract {
            archive,
            output,
            encoding,
            select,
            prompt_password,
            open,
            allow_unsandboxed,
        } => {
            let config = Config::load(&config_path)?;
            let backend = BackendBundle::verify(&config.backend_directory()?)?;
            let encoding = encoding.unwrap_or(config.behavior.default_filename_encoding);
            let password = match prepare_password(prompt_password, || {
                iroha_zip::platform::prompt_archive_password(&archive)
            })? {
                PasswordPreparation::Ready(password) => password,
                PasswordPreparation::Cancelled => return Ok(()),
            };
            let result = extract::extract(ExtractRequest {
                backend: &backend,
                config: &config,
                archive: &archive,
                output: output.as_deref(),
                encoding,
                selections: &select,
                password,
                open,
                allow_unsandboxed,
            })?;
            eprintln!("{}", result.attachment_handoff.message());
            println!("{}", result.destination.display());
        }
        Command::Create {
            format,
            source,
            output,
            allow_unsandboxed,
        } => {
            let config = Config::load(&config_path)?;
            let backend = BackendBundle::verify(&config.backend_directory()?)?;
            let archive = create::create_archive(
                &backend,
                &config,
                format,
                &source,
                &output,
                allow_unsandboxed,
            )?;
            println!("{}", archive.display());
        }
    }
    Ok(())
}

fn run_password_probe(mode: PasswordProbeMode) -> Result<()> {
    use std::io::Write as _;

    const EXPECTED: &str = "日本語-password-probe";

    if std::env::args_os()
        .chain(std::env::vars_os().flat_map(|(key, value)| [key, value]))
        .any(|value| value.to_string_lossy().contains(EXPECTED))
    {
        return Err(IrohaZipError::Sandbox(
            "password probe sentinel reached command-line or environment state".to_owned(),
        ));
    }

    if mode == PasswordProbeMode::Overflow {
        let block = vec![b'X'; 64 * 1024];
        let mut output = std::io::stdout().lock();
        for _ in 0..18 {
            output.write_all(&block).map_err(|error| {
                iroha_zip::error::IrohaZipError::io("password probe output", error)
            })?;
        }
        output.flush().map_err(|error| {
            iroha_zip::error::IrohaZipError::io("flush password probe output", error)
        })?;
    }

    let input = read_password_probe_line()?;
    let matches = input.as_str() == EXPECTED;

    match mode {
        PasswordProbeMode::Accept if matches => Ok(()),
        PasswordProbeMode::Accept => Err(IrohaZipError::Backend(
            "password probe received the wrong one-use value".to_owned(),
        )),
        PasswordProbeMode::Repeat => {
            let _forbidden_retry = read_password_probe_line()?;
            Err(IrohaZipError::Backend(
                "password probe unexpectedly received a retry".to_owned(),
            ))
        }
        PasswordProbeMode::Sleep | PasswordProbeMode::Overflow => {
            std::thread::sleep(std::time::Duration::from_mins(1));
            Ok(())
        }
        PasswordProbeMode::Crash => std::process::abort(),
    }
}

fn read_password_probe_line() -> Result<zeroize::Zeroizing<String>> {
    let mut input = zeroize::Zeroizing::new(String::new());
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|error| iroha_zip::error::IrohaZipError::io("read password probe input", error))?;
    while input.ends_with(['\r', '\n']) {
        input.pop();
    }
    Ok(input)
}

fn print_evidence(evidence: &BackendEvidence) {
    println!("evidence:      {}", evidence.root().display());
    println!("source:        {}", evidence.source_kind());
    println!("verification:  {}", evidence.verification_method());
    println!(
        "inventory:     {} packages, {} payload files",
        evidence.package_count(),
        evidence.file_count()
    );
}

#[cfg(windows)]
fn report_attachment_handoff_diagnostic(policy: AttachmentHandoffPolicy) -> Result<()> {
    if !policy.is_enabled() {
        println!("trust handoff: disabled by configuration");
        return Ok(());
    }
    match probe_attachment_handoff() {
        Ok(()) => {
            println!(
                "trust handoff: Windows Attachment Services accepted a benign diagnostic file; this is not a clean verdict"
            );
            Ok(())
        }
        Err(error) if !policy.is_required() => {
            println!(
                "trust handoff: unavailable; best-effort publication would continue with an explicit warning: {error}"
            );
            Ok(())
        }
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
fn probe_attachment_handoff() -> Result<()> {
    let parent = std::env::temp_dir().join("iroha-zip-attachment-diagnostic");
    let root = util::create_unique_dir(&parent, "probe-")?;
    let path = root.join("iroha-zip-benign-diagnostic.txt");
    let result = (|| {
        fs::write(&path, b"iroha-zip Windows trust handoff diagnostic\r\n").map_err(|error| {
            IrohaZipError::io_path("cannot create handoff diagnostic", &path, error)
        })?;
        let before = AuditedFile::open(&path, 1024)?.fingerprint().clone();
        let session = AttachmentHandoffSession::new()?;
        session.handoff(&path)?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            IrohaZipError::io_path("cannot inspect handoff diagnostic", &path, error)
        })?;
        iroha_zip::platform::validate_post_handoff_entry_security(&path, &metadata)?;
        let after = AuditedFile::open(&path, 1024)?;
        if before.length() != after.fingerprint().length()
            || before.sha256() != after.fingerprint().sha256()
        {
            return Err(IrohaZipError::TrustHandoff(
                "Attachment Services changed the diagnostic file's primary data".to_owned(),
            ));
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&root);
    result
}

#[cfg(windows)]
fn open_settings(config_path: &std::path::Path) -> Result<()> {
    let executable = std::env::current_exe()
        .map_err(|error| IrohaZipError::io("cannot locate iroha-zip executable", error))?;
    let directory = executable.parent().ok_or_else(|| {
        IrohaZipError::Config("iroha-zip executable has no parent directory".to_owned())
    })?;
    let settings = directory.join("iroha-zip-settings.exe");
    std::process::Command::new(&settings)
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| IrohaZipError::io_path("cannot open settings", &settings, error))?;
    Ok(())
}

#[cfg(not(windows))]
fn open_settings(_config_path: &std::path::Path) -> Result<()> {
    Err(iroha_zip::error::IrohaZipError::Unsupported(
        "the graphical settings screen is available on Windows".to_owned(),
    ))
}

#[cfg(windows)]
fn probe_backend_in_sandbox(
    backend: &BackendBundle,
    config: &Config,
) -> Result<(String, ProcessIsolation)> {
    let sandbox = Sandbox::new(
        config.sandbox.memory_limit_mib,
        false,
        config.sandbox.isolation,
    )?;
    let operation = (|| {
        let sandbox_backend = backend.copy_verified_to(&sandbox.root().join("backend"))?;
        let _backend_sealed = sandbox.seal_sandbox_tree(
            &sandbox.root().join("backend"),
            backend.copied_entry_count()?,
        )?;
        let stdout_log = sandbox.root().join("doctor.stdout.log");
        let stderr_log = sandbox.root().join("doctor.stderr.log");
        let result = sandbox.run(ProcessSpec {
            program: sandbox_backend,
            args: vec![OsString::from("--version")],
            current_dir: sandbox.root().to_path_buf(),
            temp_dir: None,
            stdin_file: None,
            interactive_password: None,
            stdout_log: stdout_log.clone(),
            stderr_log: stderr_log.clone(),
            timeout: Duration::from_secs(config.sandbox.timeout_seconds.clamp(1, 30)),
            monitor_root: None,
            limits: config.limits.clone(),
        })?;
        let stdout = util::read_limited(&stdout_log, 16 * 1024)?;
        let stderr = util::read_limited(&stderr_log, 16 * 1024)?;
        if result.exit_code != 0 {
            return Err(IrohaZipError::Backend(format!(
                "sandboxed bsdtar --version failed with code {}. stderr={stderr:?}, stdout={stdout:?}",
                result.exit_code
            )));
        }
        let version = if stdout.is_empty() { stderr } else { stdout };
        if version.is_empty() {
            return Err(IrohaZipError::Backend(
                "sandboxed bsdtar --version returned no text".to_owned(),
            ));
        }
        Ok((version, result.isolation))
    })();
    match operation {
        Ok(evidence) => {
            sandbox.cleanup()?;
            Ok(evidence)
        }
        Err(error) => sandbox.fail_after_cleanup(error),
    }
}
