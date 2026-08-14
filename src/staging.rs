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
    extracted_root: PathBuf,
    payload_root: PathBuf,
    workspace_root: PathBuf,
    summary: AuditSummary,
}

#[derive(Clone, Copy)]
pub(crate) enum ListingPolicy {
    External,
    CreatedByIrohaZip,
}

impl StagedArchive {
    pub(crate) fn extracted_root(&self) -> &Path {
        &self.extracted_root
    }

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
    listing_policy: ListingPolicy,
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

        let sandbox_backend = backend.copy_verified_to(&backend_dir)?;
        #[cfg(windows)]
        let (listing_program, listing_candidates) =
            prepare_windows_archive_lister(backend, &sandbox, &workspace_root)?;
        let _backend_sealed =
            sandbox.seal_sandbox_tree(&backend_dir, backend.copied_entry_count()?)?;
        let sandbox_archive = input_dir.join("archive.bin");
        archive_snapshot.copy_to_new(&sandbox_archive)?;
        let sandbox_archive_guard =
            AuditedFile::open(&sandbox_archive, config.limits.max_archive_bytes)?;
        if sandbox_archive_guard.fingerprint().length() != archive_snapshot.fingerprint().length()
            || sandbox_archive_guard.fingerprint().sha256()
                != archive_snapshot.fingerprint().sha256()
        {
            return Err(IrohaZipError::Policy(
                "sandbox archive copy does not match the retained input handle".to_owned(),
            ));
        }
        let _input_sealed = sandbox.seal_sandbox_tree(&input_dir, 1)?;

        let stdout_log = workspace_root.join("bsdtar.stdout.log");
        let stderr_log = workspace_root.join("bsdtar.stderr.log");

        #[cfg(windows)]
        let listing_args = internal_listing_arguments(
            &backend_dir,
            &listing_candidates,
            &sandbox_archive,
            encoding,
            &config.limits,
            allow_unsandboxed,
        )?;
        #[cfg(windows)]
        let listing_failure_name = "sandboxed libarchive UTF-8 listing";
        #[cfg(not(windows))]
        let listing_program = sandbox_backend.clone();
        #[cfg(not(windows))]
        let listing_args = bsdtar_listing_arguments(&sandbox_archive, encoding);
        #[cfg(not(windows))]
        let listing_failure_name = "bsdtar listing";
        let listing_baseline = policy::measure_tree(&workspace_root)?;
        let listing_limits = monitor::limits_with_baseline(
            &listing_baseline,
            2,
            0,
            MAX_ARCHIVE_LISTING_BYTES + MAX_ARCHIVE_LISTING_STDERR_BYTES,
            MAX_ARCHIVE_LISTING_BYTES,
        )?;
        let listing_result = sandbox.run(ProcessSpec {
            program: listing_program,
            args: listing_args,
            current_dir: workspace_root.clone(),
            temp_dir: None,
            stdin_file: None,
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
                "{listing_failure_name} exited with code {}. stderr={stderr:?}, stdout={stdout:?}",
                listing_result.exit_code
            )));
        }
        let listing = read_listing(&stdout_log)?;
        match listing_policy {
            ListingPolicy::External => {
                policy::validate_archive_listing(&listing, &config.limits)?;
            }
            ListingPolicy::CreatedByIrohaZip => {
                policy::validate_created_archive_listing(&listing, &config.limits)?;
            }
        }
        require_sandbox_archive_unchanged(&sandbox_archive, sandbox_archive_guard.fingerprint())?;
        remove_process_log(&stdout_log)?;
        remove_process_log(&stderr_log)?;
        fs::create_dir(&output_dir).map_err(|error| {
            IrohaZipError::io_path(
                "cannot create fresh sandbox output directory after preflight",
                &output_dir,
                error,
            )
        })?;

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
            temp_dir: None,
            stdin_file: None,
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
        require_sandbox_archive_unchanged(&sandbox_archive, sandbox_archive_guard.fingerprint())?;

        let summary = policy::audit_tree(&output_dir, &config.limits)?;
        let payload_root = choose_payload_root(&output_dir, &archive)?;
        Ok((output_dir, payload_root, workspace_root, summary))
    })();
    match operation {
        Ok((extracted_root, payload_root, workspace_root, summary)) => Ok(StagedArchive {
            sandbox,
            extracted_root,
            payload_root,
            workspace_root,
            summary,
        }),
        Err(error) => sandbox.fail_after_cleanup(error),
    }
}

fn require_sandbox_archive_unchanged(
    archive: &Path,
    expected: &crate::snapshot::FileFingerprint,
) -> Result<()> {
    let observed = AuditedFile::open(archive, expected.length())?;
    if observed.fingerprint() != expected {
        return Err(IrohaZipError::Policy(
            "sandbox archive changed between listing and extraction".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn prepare_windows_archive_lister(
    backend: &BackendBundle,
    sandbox: &Sandbox,
    workspace_root: &Path,
) -> Result<(PathBuf, PathBuf)> {
    const MAX_SELF_EXECUTABLE_BYTES: u64 = 512 * 1024 * 1024;

    let lister_dir = workspace_root.join("internal-lister");
    fs::create_dir(&lister_dir).map_err(|error| {
        IrohaZipError::io_path(
            "cannot create internal archive lister directory",
            &lister_dir,
            error,
        )
    })?;
    let candidates = lister_dir.join("backend-dll-candidates.txt");
    backend.write_library_candidates(&candidates)?;

    let current_executable = std::env::current_exe()
        .map_err(|error| IrohaZipError::io("cannot locate archive lister executable", error))?;
    let executable = lister_dir.join("iroha-zip-archive-lister.exe");
    let mut executable_snapshot =
        AuditedFile::open(&current_executable, MAX_SELF_EXECUTABLE_BYTES)?;
    executable_snapshot.copy_to_new(&executable)?;
    let _lister_sealed = sandbox.seal_sandbox_tree(&lister_dir, 2)?;
    Ok((executable, candidates))
}

#[cfg(windows)]
fn internal_listing_arguments(
    backend_root: &Path,
    candidates: &Path,
    archive: &Path,
    encoding: FilenameEncoding,
    limits: &policy::Limits,
    allow_unsandboxed: bool,
) -> Result<Vec<OsString>> {
    let max_entries = limits
        .max_files
        .checked_add(limits.max_directories)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| IrohaZipError::Config("archive entry limit overflow".to_owned()))?;
    let mut args = vec![
        OsString::from("internal-archive-listing"),
        backend_root.as_os_str().to_owned(),
        candidates.as_os_str().to_owned(),
        archive.as_os_str().to_owned(),
        OsString::from("--encoding"),
        OsString::from(encoding.cli_name()),
        OsString::from("--max-entries"),
        OsString::from(max_entries.to_string()),
        OsString::from("--max-path-bytes"),
        OsString::from(limits.max_path_bytes.to_string()),
    ];
    if allow_unsandboxed {
        args.push(OsString::from("--allow-unsandboxed"));
    }
    Ok(args)
}

#[cfg(not(windows))]
fn bsdtar_listing_arguments(archive: &Path, encoding: FilenameEncoding) -> Vec<OsString> {
    let mut args: Vec<OsString> = ["-t", "-f"].into_iter().map(OsString::from).collect();
    args.push(archive.as_os_str().to_owned());
    if let Some(option) = encoding.bsdtar_option() {
        args.push(OsString::from("--options"));
        args.push(OsString::from(option));
    }
    args
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
