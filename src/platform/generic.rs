use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File, Metadata};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{IrohaZipError, Result};
use crate::monitor;
use crate::platform::{FileIdentity, ProcessResult, ProcessSpec};
use crate::util;

pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    pub fn new(_memory_limit_mib: u64, allow_unsandboxed: bool) -> Result<Self> {
        if !allow_unsandboxed {
            return Err(IrohaZipError::Unsupported(
                "AppContainer isolation is only implemented on Windows. Pass --allow-unsandboxed only for controlled testing."
                    .to_owned(),
            ));
        }
        let parent = std::env::temp_dir().join("iroha-zip-unsandboxed");
        let root = util::create_unique_dir(&parent, "job-")?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn run(&self, spec: ProcessSpec) -> Result<ProcessResult> {
        let stdout = File::create(&spec.stdout_log).map_err(|error| {
            IrohaZipError::io_path("cannot create process stdout log", &spec.stdout_log, error)
        })?;
        let stderr = File::create(&spec.stderr_log).map_err(|error| {
            IrohaZipError::io_path("cannot create process stderr log", &spec.stderr_log, error)
        })?;

        let mut environment = BTreeMap::<OsString, OsString>::new();
        environment.insert(OsString::from("LC_ALL"), OsString::from("C"));
        environment.insert(OsString::from("LANG"), OsString::from("C"));
        environment.insert(OsString::from("HOME"), self.root.as_os_str().to_owned());
        environment.insert(OsString::from("TMPDIR"), self.root.as_os_str().to_owned());
        if let Some(parent) = spec.program.parent() {
            environment.insert(OsString::from("PATH"), parent.as_os_str().to_owned());
        }

        let mut child = Command::new(&spec.program)
            .args(&spec.args)
            .current_dir(&spec.current_dir)
            .env_clear()
            .envs(environment)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| {
                IrohaZipError::io_path("cannot start archive backend", &spec.program, error)
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
                    "archive backend exceeded {:?}",
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
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn validate_directory_security(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        IrohaZipError::io_path("cannot inspect directory security", path, error)
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(IrohaZipError::Policy(format!(
            "not a real directory: {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn validate_regular_file_security(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| IrohaZipError::io_path("cannot inspect file security", path, error))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(IrohaZipError::Policy(format!(
            "not a regular file: {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn validate_extracted_entry_security(path: &Path, metadata: &Metadata) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.is_file() && metadata.nlink() != 1 {
            return Err(IrohaZipError::Policy(format!(
                "hard-linked output is rejected: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

pub fn file_identity(path: &Path) -> Result<Option<FileIdentity>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::metadata(path)
            .map_err(|error| IrohaZipError::io_path("cannot read file identity", path, error))?;
        return Ok(Some(FileIdentity {
            volume: metadata.dev(),
            index: metadata.ino(),
        }));
    }
    #[allow(unreachable_code)]
    Ok(None)
}

pub fn read_mark_of_the_web(_path: &Path) -> Result<Option<Vec<u8>>> {
    Ok(None)
}

pub fn write_mark_of_the_web(_path: &Path, _zone: &[u8]) -> Result<()> {
    Ok(())
}

pub fn open_folder(path: &Path) -> Result<()> {
    let status = Command::new("xdg-open")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| IrohaZipError::io_path("cannot open output directory", path, error))?;
    if !status.success() {
        return Err(IrohaZipError::Sandbox(format!(
            "xdg-open failed with {status}"
        )));
    }
    Ok(())
}
