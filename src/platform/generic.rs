use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File, Metadata, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::IsolationMode;
use crate::error::{IrohaZipError, Result};
use crate::monitor;
use crate::platform::{FileIdentity, ProcessIsolation, ProcessResult, ProcessSpec};
use crate::util;

pub struct Sandbox {
    root: PathBuf,
}

pub struct DirectorySnapshot {
    path: PathBuf,
    file: File,
    identity: Option<FileIdentity>,
}

impl DirectorySnapshot {
    pub fn open(path: &Path) -> Result<Self> {
        validate_directory_security(path)?;
        let path = fs::canonicalize(path).map_err(|error| {
            IrohaZipError::io_path("cannot resolve directory snapshot", path, error)
        })?;
        validate_directory_security(&path)?;
        let file = File::open(&path).map_err(|error| {
            IrohaZipError::io_path("cannot open directory snapshot", &path, error)
        })?;
        let metadata = file.metadata().map_err(|error| {
            IrohaZipError::io_path("cannot inspect directory snapshot handle", &path, error)
        })?;
        if !metadata.is_dir() {
            return Err(IrohaZipError::Policy(format!(
                "directory snapshot is not a directory: {}",
                path.display()
            )));
        }
        let identity = file_identity_from_handle(&path, &file)?;
        let snapshot = Self {
            path,
            file,
            identity,
        };
        snapshot.verify_unchanged()?;
        Ok(snapshot)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn identity(&self) -> Option<&FileIdentity> {
        self.identity.as_ref()
    }

    pub fn entries(&self, max_entries: u64) -> Result<Vec<OsString>> {
        self.verify_unchanged()?;
        let mut names = Vec::new();
        for entry in fs::read_dir(&self.path).map_err(|error| {
            IrohaZipError::io_path("cannot enumerate directory snapshot", &self.path, error)
        })? {
            let entry = entry.map_err(|error| {
                IrohaZipError::io_path("cannot read directory snapshot entry", &self.path, error)
            })?;
            if u64::try_from(names.len()).unwrap_or(u64::MAX) >= max_entries {
                return Err(IrohaZipError::Policy(format!(
                    "directory contains more than {max_entries} entries: {}",
                    self.path.display()
                )));
            }
            names.push(entry.file_name());
        }
        names.sort();
        self.verify_unchanged()?;
        Ok(names)
    }

    fn verify_unchanged(&self) -> Result<()> {
        validate_directory_security(&self.path)?;
        let metadata = self.file.metadata().map_err(|error| {
            IrohaZipError::io_path(
                "cannot inspect directory snapshot handle",
                &self.path,
                error,
            )
        })?;
        if !metadata.is_dir() {
            return Err(IrohaZipError::Policy(format!(
                "directory snapshot changed type: {}",
                self.path.display()
            )));
        }
        let handle_identity = file_identity_from_handle(&self.path, &self.file)?;
        let path_identity = file_identity(&self.path)?;
        if handle_identity != self.identity || path_identity != self.identity {
            return Err(IrohaZipError::Policy(format!(
                "directory identity changed during enumeration: {}",
                self.path.display()
            )));
        }
        Ok(())
    }
}

pub struct AttachmentHandoffSession;

static CONFIG_SAVE_LOCK: Mutex<()> = Mutex::new(());

pub struct ConfigSaveGuard {
    _guard: MutexGuard<'static, ()>,
}

pub fn lock_config_save() -> Result<ConfigSaveGuard> {
    let guard = CONFIG_SAVE_LOCK
        .lock()
        .map_err(|_| IrohaZipError::Config("configuration save lock is poisoned".to_owned()))?;
    Ok(ConfigSaveGuard { _guard: guard })
}

impl AttachmentHandoffSession {
    pub fn new() -> Result<Self> {
        Err(IrohaZipError::Unsupported(
            "Windows Attachment Services are only available on Windows".to_owned(),
        ))
    }

    pub fn handoff(&self, _path: &Path) -> Result<()> {
        Err(IrohaZipError::Unsupported(
            "Windows Attachment Services are only available on Windows".to_owned(),
        ))
    }
}

impl Sandbox {
    pub fn new(
        _memory_limit_mib: u64,
        allow_unsandboxed: bool,
        _isolation: IsolationMode,
    ) -> Result<Self> {
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

    pub fn profile_name(&self) -> Option<&str> {
        None
    }

    pub fn run(&self, spec: ProcessSpec) -> Result<ProcessResult> {
        let stdout = File::create(&spec.stdout_log).map_err(|error| {
            IrohaZipError::io_path("cannot create process stdout log", &spec.stdout_log, error)
        })?;
        let stderr = File::create(&spec.stderr_log).map_err(|error| {
            IrohaZipError::io_path("cannot create process stderr log", &spec.stderr_log, error)
        })?;

        let mut environment = BTreeMap::<OsString, OsString>::new();
        environment.insert(OsString::from("LC_ALL"), OsString::from("C.UTF-8"));
        environment.insert(OsString::from("LANG"), OsString::from("C.UTF-8"));
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
                    isolation: ProcessIsolation::UNSANDBOXED,
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

    pub fn seal_staged_source(&self, path: &Path) -> Result<bool> {
        let resolved = fs::canonicalize(path).map_err(|error| {
            IrohaZipError::io_path("cannot resolve staged source before sealing", path, error)
        })?;
        validate_directory_security(&resolved)?;
        if resolved == self.root || !resolved.starts_with(&self.root) {
            return Err(IrohaZipError::Sandbox(format!(
                "refusing to change staging permissions outside a sandbox child: {}",
                resolved.display()
            )));
        }
        Ok(false)
    }

    pub fn cleanup(self) -> Result<()> {
        match fs::remove_dir_all(&self.root) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(IrohaZipError::io_path(
                "cannot remove unsandboxed temporary root",
                &self.root,
                error,
            )),
        }
    }

    pub fn fail_after_cleanup<T>(self, failure: IrohaZipError) -> Result<T> {
        match self.cleanup() {
            Ok(()) => Err(failure),
            Err(cleanup) => Err(IrohaZipError::Sandbox(format!(
                "{failure}; sandbox cleanup also failed: {cleanup}"
            ))),
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

pub fn probe_staging_security_write_denials(_path: &Path) -> Result<(bool, bool)> {
    Ok((false, false))
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

pub fn open_snapshot_source(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    }
    options
        .open(path)
        .map_err(|error| IrohaZipError::io_path("cannot open snapshot source", path, error))
}

pub fn create_snapshot_target(path: &Path) -> Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| IrohaZipError::io_path("cannot create snapshot target", path, error))
}

pub fn validate_open_snapshot_source(path: &Path, file: &File) -> Result<()> {
    let metadata = file.metadata().map_err(|error| {
        IrohaZipError::io_path("cannot inspect open snapshot file", path, error)
    })?;
    if !metadata.is_file() {
        return Err(IrohaZipError::Policy(format!(
            "open snapshot source is not a regular file: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(IrohaZipError::Policy(format!(
                "hard-linked snapshot source is rejected: {}",
                path.display()
            )));
        }
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

pub fn validate_post_handoff_entry_security(path: &Path, metadata: &Metadata) -> Result<()> {
    validate_extracted_entry_security(path, metadata)
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

pub fn file_identity_from_handle(path: &Path, file: &File) -> Result<Option<FileIdentity>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = file.metadata().map_err(|error| {
            IrohaZipError::io_path("cannot read open file identity", path, error)
        })?;
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

pub fn verify_mark_of_the_web(_path: &Path, _expected: &[u8]) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_failure_cleanup_preserves_the_error_and_removes_the_root() {
        let sandbox = Sandbox::new(64, true, IsolationMode::AppContainer).unwrap();
        let root = sandbox.root().to_path_buf();
        let result: Result<()> = sandbox.fail_after_cleanup(IrohaZipError::Usage("probe".into()));
        assert!(matches!(result, Err(IrohaZipError::Usage(message)) if message == "probe"));
        assert!(!root.exists());
    }

    #[test]
    fn unsandboxed_staging_seal_is_explicitly_detection_only_and_scoped() {
        let sandbox = Sandbox::new(64, true, IsolationMode::AppContainer).unwrap();
        let source = sandbox.root().join("source");
        fs::create_dir(&source).unwrap();
        assert!(!sandbox.seal_staged_source(&source).unwrap());
        assert!(sandbox.seal_staged_source(sandbox.root()).is_err());
    }

    #[test]
    fn directory_snapshot_is_bounded_and_detects_path_replacement() {
        let parent = std::env::temp_dir().join(format!(
            "iroha-zip-directory-snapshot-{}",
            util::unique_token()
        ));
        let source = parent.join("source");
        let moved = parent.join("moved");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("one.txt"), b"one").unwrap();
        fs::write(source.join("two.txt"), b"two").unwrap();

        let snapshot = DirectorySnapshot::open(&source).unwrap();
        assert!(snapshot.entries(1).is_err());
        assert_eq!(
            snapshot.entries(2).unwrap(),
            [OsString::from("one.txt"), OsString::from("two.txt")]
        );
        fs::rename(&source, &moved).unwrap();
        fs::create_dir(&source).unwrap();
        assert!(snapshot.entries(2).is_err());

        drop(snapshot);
        fs::remove_dir_all(&parent).unwrap();
    }
}
