use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::backend::BackendBundle;
use crate::config::{Config, FilenameEncoding};
use crate::error::{IrohaZipError, Result};
use crate::platform::{ProcessSpec, Sandbox};
use crate::policy::AuditSummary;
use crate::snapshot::AuditedFile;
use crate::{monitor, policy, util};

const MAX_ARCHIVE_LISTING_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ARCHIVE_LISTING_STDERR_BYTES: u64 = 1024 * 1024;

pub(crate) struct StagedArchive {
    sandbox: Sandbox,
    payload_root: PathBuf,
    workspace_root: PathBuf,
    summary: AuditSummary,
}

impl StagedArchive {
    pub(crate) fn payload_root(&self) -> &Path {
        &self.payload_root
    }

    pub(crate) fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub(crate) fn summary(&self) -> &AuditSummary {
        &self.summary
    }

    pub(crate) fn finish(self) -> Result<()> {
        self.sandbox.cleanup()
    }

    pub(crate) fn fail<T>(self, failure: IrohaZipError) -> Result<T> {
        self.sandbox.fail_after_cleanup(failure)
    }
}

pub(crate) fn stage_archive(
    backend: &BackendBundle,
    config: &Config,
    mut archive_snapshot: AuditedFile,
    encoding: FilenameEncoding,
    allow_unsandboxed: bool,
) -> Result<StagedArchive> {
    let archive = archive_snapshot.path().to_path_buf();
    let sandbox = Sandbox::new(
        config.sandbox.memory_limit_mib,
        allow_unsandboxed,
        config.sandbox.isolation,
    )?;
    let operation = (|| {
        let workspace_root = sandbox.root().to_path_buf();
        let backend_dir = workspace_root.join("backend");
        let input_dir = workspace_root.join("input");
        let output_dir = workspace_root.join("output");
        fs::create_dir(&input_dir).map_err(|error| {
            IrohaZipError::io_path("cannot create sandbox input directory", &input_dir, error)
        })?;
        fs::create_dir(&output_dir).map_err(|error| {
            IrohaZipError::io_path("cannot create sandbox output directory", &output_dir, error)
        })?;

        let sandbox_backend = backend.copy_verified_to(&backend_dir)?;
        let sandbox_archive = input_dir.join("archive.bin");
        archive_snapshot.copy_to_new(&sandbox_archive)?;

        let stdout_log = workspace_root.join("bsdtar.stdout.log");
        let stderr_log = workspace_root.join("bsdtar.stderr.log");

        let mut listing_args: Vec<OsString> =
            ["-t", "-f"].into_iter().map(OsString::from).collect();
        listing_args.push(sandbox_archive.as_os_str().to_owned());
        if let Some(option) = encoding.bsdtar_option() {
            listing_args.push(OsString::from("--options"));
            listing_args.push(OsString::from(option));
        }
        let listing_baseline = policy::measure_tree(&workspace_root)?;
        let listing_limits = monitor::limits_with_baseline(
            &listing_baseline,
            2,
            0,
            MAX_ARCHIVE_LISTING_BYTES + MAX_ARCHIVE_LISTING_STDERR_BYTES,
            MAX_ARCHIVE_LISTING_BYTES,
        )?;
        let listing_result = sandbox.run(ProcessSpec {
            program: sandbox_backend.clone(),
            args: listing_args,
            current_dir: workspace_root.clone(),
            stdout_log: stdout_log.clone(),
            stderr_log: stderr_log.clone(),
            timeout: Duration::from_secs(config.sandbox.timeout_seconds),
            monitor_root: Some(workspace_root.clone()),
            limits: listing_limits,
        })?;
        if listing_result.exit_code != 0 {
            let stderr = util::read_limited(&stderr_log, 64 * 1024)?;
            let stdout = util::read_limited(&stdout_log, 16 * 1024)?;
            return Err(IrohaZipError::Backend(format!(
                "bsdtar listing exited with code {}. stderr={stderr:?}, stdout={stdout:?}",
                listing_result.exit_code
            )));
        }
        let listing = read_listing(&stdout_log)?;
        policy::validate_archive_listing(&listing, &config.limits)?;
        remove_process_log(&stdout_log)?;
        remove_process_log(&stderr_log)?;

        let mut args: Vec<OsString> = ["-x", "-f"].into_iter().map(OsString::from).collect();
        args.push(sandbox_archive.as_os_str().to_owned());
        args.push(OsString::from("-C"));
        args.push(output_dir.as_os_str().to_owned());
        args.extend(
            [
                "--safe-writes",
                "--no-same-owner",
                "--no-same-permissions",
                "--no-xattrs",
                "--no-acls",
                "--no-fflags",
                "--no-mac-metadata",
                "-k",
            ]
            .into_iter()
            .map(OsString::from),
        );
        if let Some(option) = encoding.bsdtar_option() {
            args.push(OsString::from("--options"));
            args.push(OsString::from(option));
        }

        let baseline = policy::measure_tree(&workspace_root)?;
        let transient_bytes = config
            .limits
            .max_total_bytes
            .checked_add(config.limits.max_single_file_bytes)
            .and_then(|value| value.checked_add(2 * 1024 * 1024))
            .ok_or_else(|| {
                IrohaZipError::Config("extraction monitor byte budget overflow".to_owned())
            })?;
        let transient_files = config.limits.max_files.checked_add(18).ok_or_else(|| {
            IrohaZipError::Config("extraction monitor file budget overflow".to_owned())
        })?;
        let transient_directories =
            config
                .limits
                .max_directories
                .checked_add(4)
                .ok_or_else(|| {
                    IrohaZipError::Config("extraction monitor directory budget overflow".to_owned())
                })?;
        let monitor_limits = monitor::limits_with_baseline(
            &baseline,
            transient_files,
            transient_directories,
            transient_bytes,
            config
                .limits
                .max_single_file_bytes
                .max(config.limits.max_archive_bytes),
        )?;

        let result = sandbox.run(ProcessSpec {
            program: sandbox_backend,
            args,
            current_dir: workspace_root.clone(),
            stdout_log: stdout_log.clone(),
            stderr_log: stderr_log.clone(),
            timeout: Duration::from_secs(config.sandbox.timeout_seconds),
            monitor_root: Some(workspace_root.clone()),
            limits: monitor_limits,
        })?;
        if result.exit_code != 0 {
            let stderr = util::read_limited(&stderr_log, 64 * 1024)?;
            let stdout = util::read_limited(&stdout_log, 16 * 1024)?;
            return Err(IrohaZipError::Backend(format!(
                "bsdtar exited with code {}. stderr={stderr:?}, stdout={stdout:?}",
                result.exit_code
            )));
        }

        let summary = policy::audit_tree(&output_dir, &config.limits)?;
        let payload_root = choose_payload_root(&output_dir, &archive)?;
        Ok((payload_root, workspace_root, summary))
    })();
    match operation {
        Ok((payload_root, workspace_root, summary)) => Ok(StagedArchive {
            sandbox,
            payload_root,
            workspace_root,
            summary,
        }),
        Err(error) => sandbox.fail_after_cleanup(error),
    }
}

fn read_listing(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read as _;

    let mut file = fs::File::open(path).map_err(|error| {
        IrohaZipError::io_path("cannot open archive member listing", path, error)
    })?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_ARCHIVE_LISTING_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            IrohaZipError::io_path("cannot read archive member listing", path, error)
        })?;
    if bytes.len() as u64 > MAX_ARCHIVE_LISTING_BYTES {
        return Err(IrohaZipError::Policy(format!(
            "archive member listing exceeds {MAX_ARCHIVE_LISTING_BYTES} bytes"
        )));
    }
    Ok(bytes)
}

fn remove_process_log(path: &Path) -> Result<()> {
    fs::remove_file(path)
        .map_err(|error| IrohaZipError::io_path("cannot remove archive preflight log", path, error))
}

fn choose_payload_root(output_root: &Path, archive: &Path) -> Result<PathBuf> {
    let base = archive
        .file_name()
        .and_then(|name| name.to_str())
        .map(util::archive_base_name)
        .unwrap_or_default();
    let mut entries = fs::read_dir(output_root).map_err(|error| {
        IrohaZipError::io_path("cannot inspect extracted root", output_root, error)
    })?;
    let first = match entries.next() {
        Some(entry) => entry.map_err(|error| {
            IrohaZipError::io_path("cannot inspect extracted root entry", output_root, error)
        })?,
        None => return Ok(output_root.to_path_buf()),
    };
    if entries.next().is_some() {
        return Ok(output_root.to_path_buf());
    }

    let metadata = fs::symlink_metadata(first.path()).map_err(|error| {
        IrohaZipError::io_path(
            "cannot inspect extracted top-level entry",
            &first.path(),
            error,
        )
    })?;
    let name_matches = first
        .file_name()
        .to_str()
        .is_some_and(|name| name.eq_ignore_ascii_case(&base));
    if metadata.is_dir() && name_matches {
        Ok(first.path())
    } else {
        Ok(output_root.to_path_buf())
    }
}
