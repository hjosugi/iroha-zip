#![deny(unsafe_code)]

#[cfg(windows)]
use std::ffi::OsString;
use std::process::ExitCode;
#[cfg(windows)]
use std::process::Stdio;
#[cfg(windows)]
use std::time::Duration;

use clap::Parser;
use safearc::backend::BackendBundle;
use safearc::cli::{Cli, Command};
use safearc::config::{Config, default_config_path};
use safearc::create;
use safearc::error::Result;
#[cfg(windows)]
use safearc::error::SafeArcError;
use safearc::extract::{self, ExtractRequest};
#[cfg(windows)]
use safearc::platform::{ProcessSpec, Sandbox};
#[cfg(windows)]
use safearc::util;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("safearc: {error}");
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
                println!("AppContainer:  available; backend execution succeeded");
            }
            #[cfg(not(windows))]
            println!("AppContainer:  unavailable; backend execution was not attempted");
        }
        Command::Extract {
            archive,
            output,
            encoding,
            open,
            allow_unsandboxed,
        } => {
            let config = Config::load(&config_path)?;
            let backend = BackendBundle::verify(&config.backend_directory()?)?;
            let encoding = encoding.unwrap_or(config.behavior.default_filename_encoding);
            let published = extract::extract(ExtractRequest {
                backend: &backend,
                config: &config,
                archive: &archive,
                output: output.as_deref(),
                encoding,
                open,
                allow_unsandboxed,
            })?;
            println!("{}", published.display());
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
fn open_settings(config_path: &std::path::Path) -> Result<()> {
    let executable = std::env::current_exe()
        .map_err(|error| SafeArcError::io("cannot locate safearc executable", error))?;
    let directory = executable.parent().ok_or_else(|| {
        SafeArcError::Config("safearc executable has no parent directory".to_owned())
    })?;
    let settings = directory.join("safearc-settings.exe");
    std::process::Command::new(&settings)
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| SafeArcError::io_path("cannot open settings", &settings, error))?;
    Ok(())
}

#[cfg(not(windows))]
fn open_settings(_config_path: &std::path::Path) -> Result<()> {
    Err(safearc::error::SafeArcError::Unsupported(
        "the graphical settings screen is available on Windows".to_owned(),
    ))
}

#[cfg(windows)]
fn probe_backend_in_sandbox(backend: &BackendBundle, config: &Config) -> Result<String> {
    let sandbox = Sandbox::new(config.sandbox.memory_limit_mib, false)?;
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
        return Err(SafeArcError::Backend(format!(
            "sandboxed bsdtar --version failed with code {}. stderr={stderr:?}, stdout={stdout:?}",
            result.exit_code
        )));
    }
    let version = if stdout.is_empty() { stderr } else { stdout };
    if version.is_empty() {
        return Err(SafeArcError::Backend(
            "sandboxed bsdtar --version returned no text".to_owned(),
        ));
    }
    Ok(version)
}
