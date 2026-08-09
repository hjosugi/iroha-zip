use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::backend::BackendBundle;
use crate::cli::FilenameEncoding;
use crate::config::Config;
use crate::error::{IrohaZipError, Result};
use crate::platform::{ProcessSpec, Sandbox};
use crate::{monitor, policy, transfer, util};

pub struct ExtractResult {
    pub destination: PathBuf,
    pub attachment_handoff: transfer::AttachmentHandoffOutcome,
}

pub struct ExtractRequest<'a> {
    pub backend: &'a BackendBundle,
    pub config: &'a Config,
    pub archive: &'a Path,
    pub output: Option<&'a Path>,
    pub encoding: FilenameEncoding,
    pub open: bool,
    pub allow_unsandboxed: bool,
}

pub fn extract(request: ExtractRequest<'_>) -> Result<ExtractResult> {
    let mut archive_snapshot = policy::open_input_archive(request.archive, &request.config.limits)?;
    let archive = archive_snapshot.path().to_path_buf();
    let destination = request
        .output
        .map(Path::to_path_buf)
        .map_or_else(|| util::smart_destination(&archive), Ok)?;
    if destination.exists() {
        return Err(IrohaZipError::Usage(format!(
            "refusing to overwrite existing destination: {}",
            destination.display()
        )));
    }

    let motw = if request.config.behavior.preserve_mark_of_the_web {
        crate::platform::read_mark_of_the_web(&archive)?
    } else {
        None
    };

    let sandbox = Sandbox::new(
        request.config.sandbox.memory_limit_mib,
        request.allow_unsandboxed,
        request.config.sandbox.isolation,
    )?;
    let backend_dir = sandbox.root().join("backend");
    let input_dir = sandbox.root().join("input");
    let output_dir = sandbox.root().join("output");
    fs::create_dir(&input_dir).map_err(|error| {
        IrohaZipError::io_path("cannot create sandbox input directory", &input_dir, error)
    })?;
    fs::create_dir(&output_dir).map_err(|error| {
        IrohaZipError::io_path("cannot create sandbox output directory", &output_dir, error)
    })?;

    let sandbox_backend = request.backend.copy_verified_to(&backend_dir)?;
    let sandbox_archive = input_dir.join("archive.bin");
    archive_snapshot.copy_to_new(&sandbox_archive)?;

    let stdout_log = sandbox.root().join("bsdtar.stdout.log");
    let stderr_log = sandbox.root().join("bsdtar.stderr.log");
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
    if let Some(option) = request.encoding.bsdtar_option() {
        args.push(OsString::from("--options"));
        args.push(OsString::from(option));
    }

    let baseline = policy::measure_tree(sandbox.root())?;
    let transient_bytes = request
        .config
        .limits
        .max_total_bytes
        .checked_add(request.config.limits.max_single_file_bytes)
        .and_then(|value| value.checked_add(2 * 1024 * 1024))
        .ok_or_else(|| {
            IrohaZipError::Config("extraction monitor byte budget overflow".to_owned())
        })?;
    let transient_files = request
        .config
        .limits
        .max_files
        .checked_add(18)
        .ok_or_else(|| {
            IrohaZipError::Config("extraction monitor file budget overflow".to_owned())
        })?;
    let transient_directories = request
        .config
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
        request
            .config
            .limits
            .max_single_file_bytes
            .max(request.config.limits.max_archive_bytes),
    )?;

    let result = sandbox.run(ProcessSpec {
        program: sandbox_backend,
        args,
        current_dir: sandbox.root().to_path_buf(),
        stdout_log: stdout_log.clone(),
        stderr_log: stderr_log.clone(),
        timeout: Duration::from_secs(request.config.sandbox.timeout_seconds),
        monitor_root: Some(sandbox.root().to_path_buf()),
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

    let summary = policy::audit_tree(&output_dir, &request.config.limits)?;
    let payload = choose_payload_root(&output_dir, &archive)?;
    let published = transfer::commit_tree(
        &payload,
        &destination,
        motw.as_deref(),
        request.config.behavior.attachment_handoff,
        &request.config.limits,
    )?;

    if request.open {
        crate::platform::open_folder(&published.destination)?;
    }

    eprintln!(
        "extracted {} files ({} bytes) to {}",
        summary.files,
        summary.total_bytes,
        published.destination.display()
    );
    Ok(ExtractResult {
        destination: published.destination,
        attachment_handoff: published.attachment_handoff,
    })
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
