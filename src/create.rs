use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::backend::BackendBundle;
use crate::cli::CreateFormat;
use crate::config::{Config, FilenameEncoding};
use crate::error::{IrohaZipError, Result};
use crate::platform::{ProcessSpec, Sandbox};
use crate::snapshot::FileFingerprint;
use crate::{monitor, pax, policy, staging, transfer, util};

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
        .map_err(|error| IrohaZipError::io_path("cannot open archive source", source, error))?;
    crate::platform::validate_directory_security(&source)?;
    policy::audit_tree(&source, &config.limits)?;

    let output = normalized_output(output)?;
    if output.starts_with(&source) {
        return Err(IrohaZipError::Usage(format!(
            "output archive must not be inside the source directory: {}",
            output.display()
        )));
    }
    if output.exists() {
        return Err(IrohaZipError::Usage(format!(
            "refusing to overwrite existing archive: {}",
            output.display()
        )));
    }

    let sandbox = Sandbox::new(
        config.sandbox.memory_limit_mib,
        allow_unsandboxed,
        config.sandbox.isolation,
    )?;
    let operation = (|| {
        let backend_dir = sandbox.root().join("backend");
        let source_dir = sandbox.staged_source_path();
        let output_dir = sandbox.root().join("output");
        fs::create_dir(&output_dir).map_err(|error| {
            IrohaZipError::io_path(
                "cannot create sandbox archive directory",
                &output_dir,
                error,
            )
        })?;

        let sandbox_backend = backend.copy_verified_to(&backend_dir)?;
        let expected_source =
            transfer::copy_audited_tree_fingerprint(&source, &source_dir, &config.limits)?;
        let source_archive = sandbox.root().join("source.pax.tar");
        let source_archive_fingerprint =
            pax::write_tree_archive(&source_dir, &source_archive, &config.limits)?;
        let _source_write_sealed = sandbox.seal_staged_source(&source_dir)?;

        let sandbox_archive = output_dir.join("archive.bin");
        let stdout_log = sandbox.root().join("bsdtar.stdout.log");
        let stderr_log = sandbox.root().join("bsdtar.stderr.log");
        let mut args = create_arguments(format);
        args.push(OsString::from("-f"));
        args.push(sandbox_archive.as_os_str().to_owned());
        args.push(OsString::from("@-"));

        let baseline = policy::measure_tree(sandbox.root())?;
        let transient_bytes = config
            .limits
            .max_archive_bytes
            .checked_add(2 * 1024 * 1024)
            .ok_or_else(|| {
                IrohaZipError::Config("creation monitor byte budget overflow".to_owned())
            })?;
        let monitor_limits = monitor::limits_with_baseline(
            &baseline,
            4,
            1,
            transient_bytes,
            config
                .limits
                .max_single_file_bytes
                .max(config.limits.max_archive_bytes)
                .max(source_archive_fingerprint.length()),
        )?;

        let result = sandbox.run(ProcessSpec {
            program: sandbox_backend,
            args,
            current_dir: sandbox.root().to_path_buf(),
            stdin_file: Some(source_archive.clone()),
            stdout_log: stdout_log.clone(),
            stderr_log: stderr_log.clone(),
            timeout: Duration::from_secs(config.sandbox.timeout_seconds),
            monitor_root: Some(sandbox.root().to_path_buf()),
            limits: monitor_limits,
        })?;
        if result.exit_code != 0 {
            let stderr = util::read_limited(&stderr_log, 64 * 1024)?;
            let stdout = util::read_limited(&stdout_log, 16 * 1024)?;
            return Err(IrohaZipError::Backend(format!(
                "bsdtar exited with code {} while creating {}. stderr={stderr:?}, stdout={stdout:?}",
                result.exit_code,
                format.expected_extension()
            )));
        }

        require_tree_fingerprint(
            &source_dir,
            &config.limits,
            &expected_source,
            "backend modified the staged source tree while creating the archive",
        )?;
        require_file_fingerprint(
            &source_archive,
            &source_archive_fingerprint,
            "backend modified the bounded PAX source stream while creating the archive",
        )?;

        let verified_archive = verify_created_archive(
            backend,
            config,
            &sandbox_archive,
            &expected_source,
            allow_unsandboxed,
        )?;
        require_tree_fingerprint(
            &source_dir,
            &config.limits,
            &expected_source,
            "staged source tree changed before archive publication",
        )?;

        let mut publication_snapshot = open_verified_archive_for_publication(
            &sandbox_archive,
            &config.limits,
            &verified_archive,
        )?;
        publication_snapshot.copy_to_new(&output)?;
        Ok(())
    })();
    match operation {
        Ok(()) => {
            sandbox.cleanup()?;
            Ok(output)
        }
        Err(error) => sandbox.fail_after_cleanup(error),
    }
}

fn require_file_fingerprint(path: &Path, expected: &FileFingerprint, message: &str) -> Result<()> {
    let observed = crate::snapshot::AuditedFile::open(path, expected.length())?;
    if observed.fingerprint() != expected {
        return Err(IrohaZipError::Policy(message.to_owned()));
    }
    Ok(())
}

fn open_verified_archive_for_publication(
    archive: &Path,
    limits: &policy::Limits,
    verified: &FileFingerprint,
) -> Result<crate::snapshot::AuditedFile> {
    let snapshot = policy::open_input_archive(archive, limits)?;
    if snapshot.fingerprint() != verified {
        return Err(IrohaZipError::Policy(
            "created archive identity, timestamps, length, or content changed after verification"
                .to_owned(),
        ));
    }
    Ok(snapshot)
}

fn verify_created_archive(
    backend: &BackendBundle,
    config: &Config,
    archive: &Path,
    expected_source: &transfer::TreeFingerprint,
    allow_unsandboxed: bool,
) -> Result<FileFingerprint> {
    let archive_snapshot = policy::open_input_archive(archive, &config.limits)?;
    let archive_fingerprint = archive_snapshot.fingerprint().clone();
    let staged = staging::stage_archive(
        backend,
        config,
        archive_snapshot,
        FilenameEncoding::Auto,
        staging::ListingPolicy::CreatedByIrohaZip,
        allow_unsandboxed,
    )?;
    let observed = match transfer::fingerprint_tree(staged.extracted_root(), &config.limits) {
        Ok(observed) => observed,
        Err(error) => return staged.fail(error),
    };
    if &observed != expected_source {
        return staged.fail(IrohaZipError::Policy(
            "created archive does not reproduce the audited source tree".to_owned(),
        ));
    }
    staged.finish()?;
    Ok(archive_fingerprint)
}

fn require_tree_fingerprint(
    root: &Path,
    limits: &policy::Limits,
    expected: &transfer::TreeFingerprint,
    message: &str,
) -> Result<()> {
    if &transfer::fingerprint_tree(root, limits)? != expected {
        return Err(IrohaZipError::Policy(message.to_owned()));
    }
    Ok(())
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
            .map_err(|error| IrohaZipError::io("cannot read current directory", error))?;
        current.join(path)
    };
    let file_name = absolute.file_name().ok_or_else(|| {
        IrohaZipError::Usage(format!("output has no filename: {}", absolute.display()))
    })?;
    policy::validate_component(file_name)?;
    let parent = absolute.parent().ok_or_else(|| {
        IrohaZipError::Usage(format!("output has no parent: {}", absolute.display()))
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        IrohaZipError::io_path("cannot create archive output directory", parent, error)
    })?;
    crate::platform::validate_directory_security(parent)?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        IrohaZipError::io_path("cannot resolve archive output directory", parent, error)
    })?;
    crate::platform::validate_directory_security(&parent)?;
    Ok(parent.join(file_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "iroha-zip-created-publication-{}",
                util::unique_token()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn bounded_pax_input_needs_no_backend_path_rewrite() {
        for format in [
            CreateFormat::Zip,
            CreateFormat::SevenZip,
            CreateFormat::Tar,
            CreateFormat::TarGz,
        ] {
            let arguments = create_arguments(format);
            assert!(!arguments.iter().any(|argument| argument == "-s"));
        }
    }

    #[test]
    fn verified_archive_identity_replacement_is_rejected_before_publication() {
        let directory = TestDirectory::new();
        let archive = directory.0.join("archive.bin");
        let moved = directory.0.join("moved.bin");
        fs::write(&archive, b"same bytes").unwrap();
        let verified = policy::open_input_archive(&archive, &policy::Limits::default())
            .unwrap()
            .fingerprint()
            .clone();

        fs::rename(&archive, &moved).unwrap();
        fs::write(&archive, b"same bytes").unwrap();

        assert!(
            open_verified_archive_for_publication(&archive, &policy::Limits::default(), &verified)
                .is_err()
        );
    }

    #[test]
    fn verified_archive_same_size_mutation_is_rejected_before_publication() {
        let directory = TestDirectory::new();
        let archive = directory.0.join("archive.bin");
        fs::write(&archive, b"alpha").unwrap();
        let verified = policy::open_input_archive(&archive, &policy::Limits::default())
            .unwrap()
            .fingerprint()
            .clone();

        fs::write(&archive, b"bravo").unwrap();

        assert!(
            open_verified_archive_for_publication(&archive, &policy::Limits::default(), &verified)
                .is_err()
        );
    }
}
