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
use iroha_zip::cli::{Cli, Command};
#[cfg(windows)]
use iroha_zip::config::AttachmentHandoffPolicy;
use iroha_zip::config::{Config, default_config_path};
use iroha_zip::create;
#[cfg(windows)]
use iroha_zip::error::IrohaZipError;
use iroha_zip::error::Result;
use iroha_zip::extract::{self, ExtractRequest};
#[cfg(windows)]
use iroha_zip::platform::{AttachmentHandoffSession, ProcessSpec, Sandbox};
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
            #[cfg(windows)]
            {
                let version = probe_backend_in_sandbox(&backend, &config)?;
                println!("version:       {version}");
                println!(
                    "isolation:     {}; backend execution succeeded",
                    config.sandbox.isolation.display_name()
                );
                report_attachment_handoff_diagnostic(config.behavior.attachment_handoff)?;
            }
            #[cfg(not(windows))]
            println!("AppContainer:  unavailable; backend execution was not attempted");
        }
        Command::Preview {
            archive,
            encoding,
            allow_unsandboxed,
        } => {
            let config = Config::load(&config_path)?;
            let backend = BackendBundle::verify(&config.backend_directory()?)?;
            let encoding = encoding.unwrap_or(config.behavior.default_filename_encoding);
            let result = preview::preview(PreviewRequest {
                backend: &backend,
                config: &config,
                archive: &archive,
                encoding,
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
            open,
            allow_unsandboxed,
        } => {
            let config = Config::load(&config_path)?;
            let backend = BackendBundle::verify(&config.backend_directory()?)?;
            let encoding = encoding.unwrap_or(config.behavior.default_filename_encoding);
            let result = extract::extract(ExtractRequest {
                backend: &backend,
                config: &config,
                archive: &archive,
                output: output.as_deref(),
                encoding,
                selections: &select,
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
fn probe_backend_in_sandbox(backend: &BackendBundle, config: &Config) -> Result<String> {
    let sandbox = Sandbox::new(
        config.sandbox.memory_limit_mib,
        false,
        config.sandbox.isolation,
    )?;
    let sandbox_backend = backend.copy_verified_to(&sandbox.root().join("backend"))?;
    let stdout_log = sandbox.root().join("doctor.stdout.log");
    let stderr_log = sandbox.root().join("doctor.stderr.log");
    let result = sandbox.run(ProcessSpec {
        program: sandbox_backend,
        args: vec![OsString::from("--version")],
        current_dir: sandbox.root().to_path_buf(),
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
    Ok(version)
}
