use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::config::AttachmentHandoffPolicy;
use crate::error::{IrohaZipError, Result};
use crate::platform;
use crate::policy::{self, AuditSummary, Limits};
use crate::snapshot::{AuditedFile, FileFingerprint};
use crate::util;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeFingerprint {
    summary: AuditSummary,
    sha256: [u8; 32],
}

struct TreeAudit {
    fingerprint: TreeFingerprint,
    files: BTreeMap<PathBuf, FileFingerprint>,
    directories: BTreeMap<PathBuf, Option<platform::FileIdentity>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttachmentHandoffOutcome {
    Disabled,
    Completed {
        files: u64,
    },
    Incomplete {
        completed_files: u64,
        total_files: u64,
        reason: String,
    },
}

impl AttachmentHandoffOutcome {
    pub fn is_incomplete(&self) -> bool {
        matches!(self, Self::Incomplete { .. })
    }

    pub fn message(&self) -> String {
        match self {
            Self::Disabled => "Windows trust handoff: disabled".to_owned(),
            Self::Completed { files } => format!(
                "Windows trust handoff: completed for {files} files (this is not a clean verdict)"
            ),
            Self::Incomplete {
                completed_files,
                total_files,
                reason,
            } => format!(
                "Windows trust handoff: incomplete ({completed_files}/{total_files}); publication continued by explicit best-effort policy: {reason}"
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitResult {
    pub destination: PathBuf,
    pub attachment_handoff: AttachmentHandoffOutcome,
}

impl TreeFingerprint {
    pub fn summary(&self) -> &AuditSummary {
        &self.summary
    }

    pub fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

pub fn commit_tree(
    source_root: &Path,
    destination: &Path,
    motw: Option<&[u8]>,
    attachment_handoff: AttachmentHandoffPolicy,
    limits: &Limits,
) -> Result<CommitResult> {
    let destination = absolute_path(destination)?;
    let file_name = destination.file_name().ok_or_else(|| {
        IrohaZipError::Usage(format!(
            "destination has no final name: {}",
            destination.display()
        ))
    })?;
    policy::validate_component(file_name)?;

    let parent = destination.parent().ok_or_else(|| {
        IrohaZipError::Usage(format!(
            "destination has no parent: {}",
            destination.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        IrohaZipError::io_path("cannot create destination parent", parent, error)
    })?;
    platform::validate_directory_security(parent)?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        IrohaZipError::io_path("cannot resolve destination parent", parent, error)
    })?;
    platform::validate_directory_security(&parent)?;
    let destination = parent.join(file_name);

    if destination.exists() {
        return Err(IrohaZipError::Usage(format!(
            "refusing to overwrite existing destination: {}",
            destination.display()
        )));
    }

    let partial = parent.join(format!(".iroha-zip-partial-{}", util::unique_token()));
    let result = (|| {
        copy_audited_tree(source_root, &partial, limits)?;
        let expected = build_tree_audit(&partial, limits)?;
        if let Some(zone) = motw {
            apply_motw_tree(&partial, zone)?;
        }
        let handoff = perform_attachment_handoff(&partial, &expected.files, attachment_handoff)?;
        if attachment_handoff.is_enabled() {
            let observed = build_post_handoff_tree_audit(&partial, limits)?;
            let identities_match = expected.files.iter().all(|(path, before)| {
                observed
                    .files
                    .get(path)
                    .is_some_and(|after| after.identity() == before.identity())
            });
            if observed.fingerprint != expected.fingerprint || !identities_match {
                return Err(IrohaZipError::Policy(
                    "file identity, content, or tree structure changed during Windows trust handoff"
                        .to_owned(),
                ));
            }
            if let Some(zone) = motw {
                verify_motw_tree(&partial, expected.files.keys(), zone)?;
            }
        }
        if destination.exists() {
            return Err(IrohaZipError::Usage(format!(
                "destination appeared before publish: {}",
                destination.display()
            )));
        }
        fs::rename(&partial, &destination).map_err(|error| {
            IrohaZipError::io_path(
                "cannot atomically publish extracted directory",
                &destination,
                error,
            )
        })?;
        Ok(CommitResult {
            destination: destination.clone(),
            attachment_handoff: handoff,
        })
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&partial);
    }
    result
}

pub fn copy_audited_tree(
    source_root: &Path,
    target_root: &Path,
    limits: &Limits,
) -> Result<AuditSummary> {
    Ok(copy_audited_tree_fingerprint(source_root, target_root, limits)?.summary)
}

pub(crate) fn copy_audited_tree_fingerprint(
    source_root: &Path,
    target_root: &Path,
    limits: &Limits,
) -> Result<TreeFingerprint> {
    copy_audited_tree_inner(source_root, target_root, limits, || Ok(()))
}

fn copy_audited_tree_inner<F>(
    source_root: &Path,
    target_root: &Path,
    limits: &Limits,
    after_audit: F,
) -> Result<TreeFingerprint>
where
    F: FnOnce() -> Result<()>,
{
    let expected = build_tree_audit(source_root, limits)?;
    after_audit()?;
    fs::create_dir(target_root).map_err(|error| {
        IrohaZipError::io_path("cannot create staged output directory", target_root, error)
    })?;

    let copy_result = copy_tree(
        source_root,
        target_root,
        limits,
        &expected.files,
        &expected.directories,
    );
    if let Err(error) = copy_result {
        let _ = fs::remove_dir_all(target_root);
        return Err(error);
    }

    let copied = match fingerprint_tree(target_root, limits) {
        Ok(summary) => summary,
        Err(error) => {
            let _ = fs::remove_dir_all(target_root);
            return Err(error);
        }
    };
    if copied != expected.fingerprint {
        let _ = fs::remove_dir_all(target_root);
        return Err(IrohaZipError::Policy(
            "source tree changed while it was being copied".to_owned(),
        ));
    }
    Ok(copied)
}

pub fn fingerprint_tree(root: &Path, limits: &Limits) -> Result<TreeFingerprint> {
    Ok(build_tree_audit(root, limits)?.fingerprint)
}

fn build_tree_audit(root: &Path, limits: &Limits) -> Result<TreeAudit> {
    build_tree_audit_with(root, limits, platform::validate_extracted_entry_security)
}

fn build_post_handoff_tree_audit(root: &Path, limits: &Limits) -> Result<TreeAudit> {
    build_tree_audit_with(root, limits, platform::validate_post_handoff_entry_security)
}

fn build_tree_audit_with(
    root: &Path,
    limits: &Limits,
    validate_entry: fn(&Path, &fs::Metadata) -> Result<()>,
) -> Result<TreeAudit> {
    platform::validate_directory_security(root)?;
    let root = fs::canonicalize(root)
        .map_err(|error| IrohaZipError::io_path("cannot resolve audited tree", root, error))?;
    platform::validate_directory_security(&root)?;

    let max_entries = tree_entry_limit(limits);
    let root_snapshot = platform::DirectorySnapshot::open(&root)?;
    let mut directories = BTreeMap::new();
    directories.insert(PathBuf::new(), root_snapshot.identity().cloned());
    let mut relative_paths = BTreeSet::new();
    let mut stack = vec![root_snapshot];
    while let Some(directory_snapshot) = stack.pop() {
        let directory = directory_snapshot.path();
        if !directory.starts_with(&root) {
            return Err(IrohaZipError::Policy(format!(
                "audited directory escaped its root: {}",
                directory.display()
            )));
        }
        for name in directory_snapshot.entries(max_entries)? {
            policy::validate_component(&name)?;
            let path = directory.join(name);
            let relative = path.strip_prefix(&root).map_err(|_| {
                IrohaZipError::Policy(format!("audited path escaped its root: {}", path.display()))
            })?;
            policy::validate_relative_path(relative, limits)?;
            if !relative_paths.insert(relative.to_path_buf()) {
                return Err(IrohaZipError::Policy(format!(
                    "duplicate path in audited tree: {}",
                    relative.display()
                )));
            }

            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                IrohaZipError::io_path("cannot inspect audited tree entry", &path, error)
            })?;
            if metadata.file_type().is_symlink() {
                return Err(IrohaZipError::Policy(format!(
                    "symbolic links are not allowed: {}",
                    relative.display()
                )));
            }
            validate_entry(&path, &metadata)?;
            if metadata.is_dir() {
                let child = platform::DirectorySnapshot::open(&path)?;
                directories.insert(relative.to_path_buf(), child.identity().cloned());
                stack.push(child);
            } else if !metadata.is_file() {
                return Err(IrohaZipError::Policy(format!(
                    "special files are not allowed: {}",
                    relative.display()
                )));
            }
        }
    }

    let mut summary = AuditSummary::default();
    let mut hasher = Sha256::new();
    let mut files = BTreeMap::new();
    for relative in relative_paths {
        let path = root.join(&relative);
        let relative_text = relative.to_str().ok_or_else(|| {
            IrohaZipError::Policy(format!(
                "non-Unicode filenames are rejected: {}",
                relative.display()
            ))
        })?;
        let relative_bytes = relative_text.as_bytes();
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            IrohaZipError::io_path("cannot inspect fingerprinted entry", &path, error)
        })?;
        validate_entry(&path, &metadata)?;
        if metadata.is_dir() {
            let expected_identity = directories.get(&relative).ok_or_else(|| {
                IrohaZipError::Policy(format!(
                    "directory appeared after tree enumeration: {}",
                    relative.display()
                ))
            })?;
            let observed = platform::DirectorySnapshot::open(&path)?;
            if observed.identity() != expected_identity.as_ref() {
                return Err(IrohaZipError::Policy(format!(
                    "directory identity changed while fingerprinting: {}",
                    relative.display()
                )));
            }
            summary.directories = checked_increment(summary.directories, "directory count")?;
            if summary.directories > limits.max_directories {
                return Err(IrohaZipError::Policy(format!(
                    "directory count exceeds {}",
                    limits.max_directories
                )));
            }
            update_path_hash(&mut hasher, b'D', relative_bytes)?;
        } else if metadata.is_file() {
            let snapshot = AuditedFile::open(&path, limits.max_single_file_bytes)?;
            if !snapshot.path().starts_with(&root) {
                return Err(IrohaZipError::Policy(format!(
                    "fingerprinted file escaped its root: {}",
                    relative.display()
                )));
            }
            summary.files = checked_increment(summary.files, "file count")?;
            if summary.files > limits.max_files {
                return Err(IrohaZipError::Policy(format!(
                    "file count exceeds {}",
                    limits.max_files
                )));
            }
            summary.total_bytes = summary
                .total_bytes
                .checked_add(snapshot.fingerprint().length())
                .ok_or_else(|| IrohaZipError::Policy("total size overflow".to_owned()))?;
            if summary.total_bytes > limits.max_total_bytes {
                return Err(IrohaZipError::Policy(format!(
                    "expanded data exceeds {} bytes",
                    limits.max_total_bytes
                )));
            }
            update_path_hash(&mut hasher, b'F', relative_bytes)?;
            hasher.update(snapshot.fingerprint().length().to_le_bytes());
            hasher.update(snapshot.fingerprint().sha256());
            files.insert(relative, snapshot.fingerprint().clone());
        } else {
            return Err(IrohaZipError::Policy(format!(
                "entry changed while fingerprinting: {}",
                relative.display()
            )));
        }
    }

    Ok(TreeAudit {
        fingerprint: TreeFingerprint {
            summary,
            sha256: hasher.finalize().into(),
        },
        files,
        directories,
    })
}

fn perform_attachment_handoff(
    root: &Path,
    files: &BTreeMap<PathBuf, FileFingerprint>,
    policy: AttachmentHandoffPolicy,
) -> Result<AttachmentHandoffOutcome> {
    if !policy.is_enabled() {
        return Ok(AttachmentHandoffOutcome::Disabled);
    }
    let total_files = u64::try_from(files.len())
        .map_err(|_| IrohaZipError::Policy("attachment file count overflow".to_owned()))?;
    if files.is_empty() {
        return Ok(AttachmentHandoffOutcome::Completed { files: 0 });
    }
    let session = match platform::AttachmentHandoffSession::new() {
        Ok(session) => session,
        Err(error) if !policy.is_required() => {
            return Ok(AttachmentHandoffOutcome::Incomplete {
                completed_files: 0,
                total_files,
                reason: error.to_string(),
            });
        }
        Err(error) => return Err(error),
    };
    let relative_files = files.keys().cloned().collect::<Vec<_>>();
    perform_attachment_handoff_with(root, &relative_files, policy, |path| session.handoff(path))
}

fn perform_attachment_handoff_with(
    root: &Path,
    relative_files: &[PathBuf],
    policy: AttachmentHandoffPolicy,
    mut handoff: impl FnMut(&Path) -> Result<()>,
) -> Result<AttachmentHandoffOutcome> {
    let total_files = u64::try_from(relative_files.len())
        .map_err(|_| IrohaZipError::Policy("attachment file count overflow".to_owned()))?;
    let mut completed_files = 0_u64;
    for relative in relative_files {
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            IrohaZipError::io_path(
                "cannot inspect file before Windows trust handoff",
                &path,
                error,
            )
        })?;
        platform::validate_post_handoff_entry_security(&path, &metadata)?;
        match handoff(&path) {
            Ok(()) => completed_files = checked_increment(completed_files, "handoff file count")?,
            Err(error) if !policy.is_required() => {
                return Ok(AttachmentHandoffOutcome::Incomplete {
                    completed_files,
                    total_files,
                    reason: error.to_string(),
                });
            }
            Err(error) => return Err(error),
        }
    }
    Ok(AttachmentHandoffOutcome::Completed {
        files: completed_files,
    })
}

fn verify_motw_tree<'a>(
    root: &Path,
    files: impl Iterator<Item = &'a PathBuf>,
    zone: &[u8],
) -> Result<()> {
    for relative in files {
        platform::verify_mark_of_the_web(&root.join(relative), zone)?;
    }
    Ok(())
}

fn update_path_hash(hasher: &mut Sha256, tag: u8, path: &[u8]) -> Result<()> {
    let length = u64::try_from(path.len())
        .map_err(|_| IrohaZipError::Policy("path length overflow".to_owned()))?;
    hasher.update([tag]);
    hasher.update(length.to_le_bytes());
    hasher.update(path);
    Ok(())
}

fn checked_increment(value: u64, label: &str) -> Result<u64> {
    value
        .checked_add(1)
        .ok_or_else(|| IrohaZipError::Policy(format!("{label} overflow")))
}

fn copy_tree(
    source_root: &Path,
    target_root: &Path,
    limits: &Limits,
    expected_files: &BTreeMap<PathBuf, FileFingerprint>,
    expected_directories: &BTreeMap<PathBuf, Option<platform::FileIdentity>>,
) -> Result<()> {
    platform::validate_directory_security(source_root)?;
    platform::validate_directory_security(target_root)?;
    let source_root = fs::canonicalize(source_root).map_err(|error| {
        IrohaZipError::io_path("cannot resolve copy source root", source_root, error)
    })?;
    let target_root = fs::canonicalize(target_root).map_err(|error| {
        IrohaZipError::io_path("cannot resolve copy target root", target_root, error)
    })?;
    let root_snapshot = platform::DirectorySnapshot::open(&source_root)?;
    require_expected_directory(&root_snapshot, Path::new(""), expected_directories)?;
    let max_entries = tree_entry_limit(limits);
    let mut stack = vec![(root_snapshot, target_root.clone(), PathBuf::new())];
    let mut copied_files = BTreeSet::new();
    let mut copied_directories = BTreeSet::from([PathBuf::new()]);
    while let Some((source_snapshot, target_dir, relative_dir)) = stack.pop() {
        let source_dir = source_snapshot.path();
        platform::validate_directory_security(&target_dir)?;
        let target_dir = fs::canonicalize(&target_dir).map_err(|error| {
            IrohaZipError::io_path("cannot resolve staged target directory", &target_dir, error)
        })?;
        if !source_dir.starts_with(&source_root) || !target_dir.starts_with(&target_root) {
            return Err(IrohaZipError::Policy(
                "source or target directory escaped its audited root".to_owned(),
            ));
        }
        platform::validate_directory_security(&target_dir)?;
        for name in source_snapshot.entries(max_entries)? {
            policy::validate_component(&name)?;
            let source = source_dir.join(&name);
            let target = target_dir.join(&name);
            let relative = relative_dir.join(&name);
            policy::validate_relative_path(&relative, limits)?;
            let metadata = fs::symlink_metadata(&source).map_err(|error| {
                IrohaZipError::io_path("cannot inspect audited source", &source, error)
            })?;
            if metadata.file_type().is_symlink() {
                return Err(IrohaZipError::Policy(format!(
                    "source tree changed to a symbolic link after audit: {}",
                    source.display()
                )));
            }
            platform::validate_extracted_entry_security(&source, &metadata)?;

            if metadata.is_dir() {
                let child = platform::DirectorySnapshot::open(&source)?;
                require_expected_directory(&child, &relative, expected_directories)?;
                fs::create_dir(&target).map_err(|error| {
                    IrohaZipError::io_path("cannot create staged directory", &target, error)
                })?;
                copied_directories.insert(relative.clone());
                stack.push((child, target, relative));
            } else if metadata.is_file() {
                let mut snapshot = AuditedFile::open(&source, limits.max_single_file_bytes)?;
                if !snapshot.path().starts_with(&source_root) {
                    return Err(IrohaZipError::Policy(format!(
                        "copy source escaped its audited root: {}",
                        source.display()
                    )));
                }
                let expected = expected_files.get(&relative).ok_or_else(|| {
                    IrohaZipError::Policy(format!(
                        "file appeared after source audit: {}",
                        relative.display()
                    ))
                })?;
                if snapshot.fingerprint() != expected {
                    return Err(IrohaZipError::Policy(format!(
                        "file identity, timestamps, length, or content changed after source audit: {}",
                        relative.display()
                    )));
                }
                snapshot.copy_to_new(&target)?;
                copied_files.insert(relative);
                let copied_metadata = fs::symlink_metadata(&target).map_err(|error| {
                    IrohaZipError::io_path("cannot inspect staged file", &target, error)
                })?;
                platform::validate_extracted_entry_security(&target, &copied_metadata)?;
            } else {
                return Err(IrohaZipError::Policy(format!(
                    "source tree changed after audit: {}",
                    source.display()
                )));
            }
        }
    }
    if copied_files.len() != expected_files.len() {
        return Err(IrohaZipError::Policy(
            "one or more files disappeared after source audit".to_owned(),
        ));
    }
    if copied_directories.len() != expected_directories.len()
        || copied_directories
            .iter()
            .any(|relative| !expected_directories.contains_key(relative))
    {
        return Err(IrohaZipError::Policy(
            "one or more directories disappeared after source audit".to_owned(),
        ));
    }
    Ok(())
}

fn require_expected_directory(
    snapshot: &platform::DirectorySnapshot,
    relative: &Path,
    expected_directories: &BTreeMap<PathBuf, Option<platform::FileIdentity>>,
) -> Result<()> {
    let expected = expected_directories.get(relative).ok_or_else(|| {
        IrohaZipError::Policy(format!(
            "directory appeared after source audit: {}",
            relative.display()
        ))
    })?;
    if snapshot.identity() != expected.as_ref() {
        return Err(IrohaZipError::Policy(format!(
            "directory identity changed after source audit: {}",
            relative.display()
        )));
    }
    Ok(())
}

fn tree_entry_limit(limits: &Limits) -> u64 {
    limits.max_files.saturating_add(limits.max_directories)
}

fn apply_motw_tree(root: &Path, zone: &[u8]) -> Result<()> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        platform::validate_directory_security(&directory)?;
        for entry in fs::read_dir(&directory).map_err(|error| {
            IrohaZipError::io_path("cannot enumerate output for MotW", &directory, error)
        })? {
            let entry = entry.map_err(|error| {
                IrohaZipError::io_path("cannot enumerate output entry for MotW", &directory, error)
            })?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                IrohaZipError::io_path("cannot inspect output for MotW", &path, error)
            })?;
            if metadata.is_dir() {
                platform::validate_extracted_entry_security(&path, &metadata)?;
                stack.push(path);
            } else if metadata.is_file() {
                platform::validate_extracted_entry_security(&path, &metadata)?;
                platform::write_mark_of_the_web(&path, zone)?;
            } else {
                return Err(IrohaZipError::Policy(format!(
                    "output changed before Mark-of-the-Web propagation: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let current = std::env::current_dir()
        .map_err(|error| IrohaZipError::io("cannot read current directory", error))?;
    Ok(current.join(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("iroha-zip-transfer-race-{}", util::unique_token()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn audited_tree_copy_rejects_same_size_mutation_after_audit() {
        let directory = TestDirectory::new();
        let source = directory.0.join("source");
        let target = directory.0.join("target");
        let file = source.join("item.txt");
        fs::create_dir(&source).unwrap();
        fs::write(&file, b"alpha").unwrap();

        let result = copy_audited_tree_inner(&source, &target, &Limits::default(), || {
            fs::write(&file, b"bravo").map_err(|error| {
                IrohaZipError::io_path("cannot mutate race-test source", &file, error)
            })
        });

        assert!(result.is_err());
        assert!(!target.exists());
    }

    #[test]
    fn audited_tree_copy_rejects_identity_replacement_with_identical_bytes() {
        let directory = TestDirectory::new();
        let source = directory.0.join("source");
        let target = directory.0.join("target");
        let file = source.join("item.txt");
        let moved = source.join("moved.txt");
        fs::create_dir(&source).unwrap();
        fs::write(&file, b"same bytes").unwrap();

        let result = copy_audited_tree_inner(&source, &target, &Limits::default(), || {
            fs::rename(&file, &moved)
                .and_then(|()| fs::write(&file, b"same bytes"))
                .map_err(|error| {
                    IrohaZipError::io_path("cannot replace race-test source", &file, error)
                })
        });

        assert!(result.is_err());
        assert!(!target.exists());
    }

    #[test]
    fn audited_tree_copy_rejects_empty_directory_identity_replacement() {
        let directory = TestDirectory::new();
        let source = directory.0.join("source");
        let target = directory.0.join("target");
        let empty = source.join("empty");
        let moved = source.join("moved");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&empty).unwrap();

        let result = copy_audited_tree_inner(&source, &target, &Limits::default(), || {
            fs::rename(&empty, &moved)
                .and_then(|()| fs::create_dir(&empty))
                .and_then(|()| fs::remove_dir(&moved))
                .map_err(|error| {
                    IrohaZipError::io_path("cannot replace race-test directory", &empty, error)
                })
        });

        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("directory identity changed")
        );
        assert!(!target.exists());
    }

    #[cfg(unix)]
    #[test]
    fn audited_tree_copy_rejects_symbolic_link_replacement_after_audit() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new();
        let source = directory.0.join("source");
        let target = directory.0.join("target");
        let outside = directory.0.join("outside.txt");
        let file = source.join("item.txt");
        fs::create_dir(&source).unwrap();
        fs::write(&file, b"audited").unwrap();
        fs::write(&outside, b"hostile").unwrap();

        let result = copy_audited_tree_inner(&source, &target, &Limits::default(), || {
            fs::remove_file(&file)
                .and_then(|()| symlink(&outside, &file))
                .map_err(|error| {
                    IrohaZipError::io_path("cannot link race-test source", &file, error)
                })
        });

        assert!(result.is_err());
        assert!(!target.exists());
    }

    #[cfg(windows)]
    #[test]
    fn audited_tree_copy_rejects_junction_replacement_after_audit() {
        let directory = TestDirectory::new();
        let source = directory.0.join("source");
        let target = directory.0.join("target");
        let nested = source.join("nested");
        let moved = source.join("moved");
        let outside = directory.0.join("outside");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&nested).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(nested.join("audited.txt"), b"audited").unwrap();
        fs::write(outside.join("hostile.txt"), b"hostile").unwrap();

        let result = copy_audited_tree_inner(&source, &target, &Limits::default(), || {
            fs::rename(&nested, &moved).map_err(|error| {
                IrohaZipError::io_path("cannot move race-test directory", &nested, error)
            })?;
            let status = std::process::Command::new("cmd.exe")
                .args(["/d", "/c", "mklink", "/J"])
                .arg(&nested)
                .arg(&outside)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map_err(|error| {
                    IrohaZipError::io_path("cannot start junction race probe", &nested, error)
                })?;
            if !status.success() {
                return Err(IrohaZipError::Policy(format!(
                    "cannot create junction race probe at {}: {status}",
                    nested.display()
                )));
            }
            Ok(())
        });

        assert!(result.is_err(), "a post-audit junction must be rejected");
        assert!(!target.exists(), "a rejected junction must publish nothing");
        assert_eq!(fs::read(outside.join("hostile.txt")).unwrap(), b"hostile");
    }

    #[test]
    fn best_effort_handoff_failure_is_explicit_but_required_failure_is_fatal() {
        let directory = TestDirectory::new();
        let root = directory.0.join("output");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("item.txt"), b"content").unwrap();
        let files = vec![PathBuf::from("item.txt")];

        let best_effort = perform_attachment_handoff_with(
            &root,
            &files,
            AttachmentHandoffPolicy::BestEffort,
            |_| {
                Err(IrohaZipError::TrustHandoff(
                    "simulated engine unavailability".to_owned(),
                ))
            },
        )
        .unwrap();
        assert!(matches!(
            best_effort,
            AttachmentHandoffOutcome::Incomplete {
                completed_files: 0,
                total_files: 1,
                ref reason,
            } if reason.contains("simulated engine unavailability")
        ));

        let required = perform_attachment_handoff_with(
            &root,
            &files,
            AttachmentHandoffPolicy::Required,
            |_| {
                Err(IrohaZipError::TrustHandoff(
                    "simulated engine unavailability".to_owned(),
                ))
            },
        );
        assert!(required.is_err());
    }

    #[test]
    fn successful_handoff_is_reported_without_claiming_a_clean_verdict() {
        let directory = TestDirectory::new();
        let root = directory.0.join("output");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("one.txt"), b"one").unwrap();
        fs::write(root.join("two.txt"), b"two").unwrap();
        let files = vec![PathBuf::from("one.txt"), PathBuf::from("two.txt")];
        let mut calls = 0_u64;

        let outcome = perform_attachment_handoff_with(
            &root,
            &files,
            AttachmentHandoffPolicy::Required,
            |_| {
                calls += 1;
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(calls, 2);
        assert_eq!(outcome, AttachmentHandoffOutcome::Completed { files: 2 });
        assert!(outcome.message().contains("not a clean verdict"));
    }

    #[test]
    fn post_handoff_fingerprint_detects_same_size_content_mutation() {
        let directory = TestDirectory::new();
        let root = directory.0.join("output");
        fs::create_dir(&root).unwrap();
        let file = root.join("item.txt");
        fs::write(&file, b"alpha").unwrap();
        let expected = build_tree_audit(&root, &Limits::default()).unwrap();

        fs::write(&file, b"bravo").unwrap();
        let observed = build_post_handoff_tree_audit(&root, &Limits::default()).unwrap();

        assert_ne!(observed.fingerprint, expected.fingerprint);
    }

    #[test]
    fn post_handoff_audit_records_identity_replacement_with_identical_bytes() {
        let directory = TestDirectory::new();
        let root = directory.0.join("output");
        fs::create_dir(&root).unwrap();
        let file = root.join("item.txt");
        let moved = root.join("moved.txt");
        fs::write(&file, b"same bytes").unwrap();
        let expected = build_tree_audit(&root, &Limits::default()).unwrap();

        fs::rename(&file, &moved).unwrap();
        fs::write(&file, b"same bytes").unwrap();
        fs::remove_file(&moved).unwrap();
        let observed = build_post_handoff_tree_audit(&root, &Limits::default()).unwrap();

        assert_eq!(observed.fingerprint, expected.fingerprint);
        assert_ne!(
            observed.files[Path::new("item.txt")].identity(),
            expected.files[Path::new("item.txt")].identity()
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn unavailable_handoff_obeys_publication_policy_and_cleans_partial_tree() {
        let directory = TestDirectory::new();
        let source = directory.0.join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("item.txt"), b"content").unwrap();

        let best_effort_destination = directory.0.join("best-effort");
        let best_effort = commit_tree(
            &source,
            &best_effort_destination,
            None,
            AttachmentHandoffPolicy::BestEffort,
            &Limits::default(),
        )
        .unwrap();
        assert_eq!(best_effort.destination, best_effort_destination);
        assert!(matches!(
            best_effort.attachment_handoff,
            AttachmentHandoffOutcome::Incomplete { .. }
        ));

        let required_destination = directory.0.join("required");
        let required = commit_tree(
            &source,
            &required_destination,
            None,
            AttachmentHandoffPolicy::Required,
            &Limits::default(),
        );
        assert!(required.is_err());
        assert!(!required_destination.exists());
        assert!(fs::read_dir(&directory.0).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".iroha-zip-partial-")
        }));
    }
}
