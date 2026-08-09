use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::backend::BackendBundle;
use crate::cli::CreateFormat;
use crate::config::Config;
use crate::error::{Result, SafeArcError};
use crate::platform::{ProcessSpec, Sandbox};
use crate::{monitor, policy, transfer, util};

pub fn create_archive(
    backend: &BackendBundle,
    config: &Config,
    format: CreateFormat,
    source: &Path,
    output: &Path,
    allow_unsandboxed: bool,
) -> Result<PathBuf> {
    crate::platform::validate_directory_security(source)?;
    let source = fs::canonicalize(source)
        .map_err(|error| SafeArcError::io_path("cannot open archive source", source, error))?;
    crate::platform::validate_directory_security(&source)?;
    policy::audit_tree(&source, &config.limits)?;

    let output = normalized_output(output)?;
    if output.starts_with(&source) {
        return Err(SafeArcError::Usage(format!(
            "output archive must not be inside the source directory: {}",
            output.display()
        )));
    }
    if output.exists() {
        return Err(SafeArcError::Usage(format!(
            "refusing to overwrite existing archive: {}",
            output.display()
        )));
    }

    let sandbox = Sandbox::new(config.sandbox.memory_limit_mib, allow_unsandboxed)?;
    let backend_dir = sandbox.root().join("backend");
    let source_dir = sandbox.root().join("source");
    let output_dir = sandbox.root().join("output");
    fs::create_dir(&output_dir).map_err(|error| {
        SafeArcError::io_path(
            "cannot create sandbox archive directory",
            &output_dir,
            error,
        )
    })?;

    let sandbox_backend = backend.copy_verified_to(&backend_dir)?;
    transfer::copy_audited_tree(&source, &source_dir, &config.limits)?;

    let sandbox_archive = output_dir.join("archive.bin");
    let stdout_log = sandbox.root().join("bsdtar.stdout.log");
    let stderr_log = sandbox.root().join("bsdtar.stderr.log");
    let mut args = create_arguments(format);
    args.push(OsString::from("-f"));
    args.push(sandbox_archive.as_os_str().to_owned());
    args.push(OsString::from("-C"));
    args.push(source_dir.as_os_str().to_owned());
    args.push(OsString::from("."));

    let baseline = policy::measure_tree(sandbox.root())?;
    let transient_bytes = config
        .limits
        .max_archive_bytes
        .checked_add(2 * 1024 * 1024)
        .ok_or_else(|| SafeArcError::Config("creation monitor byte budget overflow".to_owned()))?;
    let monitor_limits = monitor::limits_with_baseline(
        &baseline,
        4,
        1,
        transient_bytes,
        config
            .limits
            .max_single_file_bytes
            .max(config.limits.max_archive_bytes),
    )?;

    let result = sandbox.run(ProcessSpec {
        program: sandbox_backend,
        args,
        current_dir: sandbox.root().to_path_buf(),
        stdout_log: stdout_log.clone(),
        stderr_log: stderr_log.clone(),
        timeout: Duration::from_secs(config.sandbox.timeout_seconds),
        monitor_root: Some(sandbox.root().to_path_buf()),
        limits: monitor_limits,
    })?;

    if result.exit_code != 0 {
        let stderr = util::read_limited(&stderr_log, 64 * 1024)?;
        let stdout = util::read_limited(&stdout_log, 16 * 1024)?;
        return Err(SafeArcError::Backend(format!(
            "bsdtar exited with code {} while creating {}. stderr={stderr:?}, stdout={stdout:?}",
            result.exit_code,
            format.expected_extension()
        )));
    }

    crate::platform::validate_regular_file_security(&sandbox_archive)?;
    let metadata = fs::symlink_metadata(&sandbox_archive).map_err(|error| {
        SafeArcError::io_path("cannot inspect staged archive", &sandbox_archive, error)
    })?;
    crate::platform::validate_extracted_entry_security(&sandbox_archive, &metadata)?;
    let size = metadata.len();
    if size == 0 {
        return Err(SafeArcError::Backend(
            "backend produced an empty archive".to_owned(),
        ));
    }
    if size > config.limits.max_archive_bytes {
        return Err(SafeArcError::Policy(format!(
            "created archive is {size} bytes; limit is {} bytes",
            config.limits.max_archive_bytes
        )));
    }

    util::copy_file_new_exact(&sandbox_archive, &output, size)?;
    Ok(output)
}

fn create_arguments(format: CreateFormat) -> Vec<OsString> {
    let values: &[&str] = match format {
        CreateFormat::Zip => &[
            "-c",
            "--format=zip",
            "--no-xattrs",
            "--no-acls",
            "--no-fflags",
        ],
        CreateFormat::SevenZip => &[
            "-c",
            "--format=7zip",
            "--no-xattrs",
            "--no-acls",
            "--no-fflags",
        ],
        CreateFormat::Tar => &[
            "-c",
            "--format=pax",
            "--no-xattrs",
            "--no-acls",
            "--no-fflags",
        ],
        CreateFormat::TarGz => &[
            "-c",
            "--format=pax",
            "-z",
            "--no-xattrs",
            "--no-acls",
            "--no-fflags",
        ],
    };
    values.iter().map(|value| OsString::from(*value)).collect()
}

fn normalized_output(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        let current = std::env::current_dir()
            .map_err(|error| SafeArcError::io("cannot read current directory", error))?;
        current.join(path)
    };
    let file_name = absolute.file_name().ok_or_else(|| {
        SafeArcError::Usage(format!("output has no filename: {}", absolute.display()))
    })?;
    policy::validate_component(file_name)?;
    let parent = absolute.parent().ok_or_else(|| {
        SafeArcError::Usage(format!("output has no parent: {}", absolute.display()))
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        SafeArcError::io_path("cannot create archive output directory", parent, error)
    })?;
    crate::platform::validate_directory_security(parent)?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        SafeArcError::io_path("cannot resolve archive output directory", parent, error)
    })?;
    crate::platform::validate_directory_security(&parent)?;
    Ok(parent.join(file_name))
}
