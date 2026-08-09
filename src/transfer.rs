use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

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
    limits: &Limits,
) -> Result<PathBuf> {
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
        if let Some(zone) = motw {
            apply_motw_tree(&partial, zone)?;
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
        Ok(destination.clone())
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
    copy_audited_tree_inner(source_root, target_root, limits, || Ok(()))
}

fn copy_audited_tree_inner<F>(
    source_root: &Path,
    target_root: &Path,
    limits: &Limits,
    after_audit: F,
) -> Result<AuditSummary>
where
    F: FnOnce() -> Result<()>,
{
    let expected = build_tree_audit(source_root, limits)?;
    after_audit()?;
    fs::create_dir(target_root).map_err(|error| {
        IrohaZipError::io_path("cannot create staged output directory", target_root, error)
    })?;

    let copy_result = copy_tree(source_root, target_root, limits, &expected.files);
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
    Ok(copied.summary)
}

pub fn fingerprint_tree(root: &Path, limits: &Limits) -> Result<TreeFingerprint> {
    Ok(build_tree_audit(root, limits)?.fingerprint)
}

fn build_tree_audit(root: &Path, limits: &Limits) -> Result<TreeAudit> {
    platform::validate_directory_security(root)?;
    let root = fs::canonicalize(root)
        .map_err(|error| IrohaZipError::io_path("cannot resolve audited tree", root, error))?;
    platform::validate_directory_security(&root)?;

    let mut relative_paths = BTreeSet::new();
    let mut stack = vec![root.clone()];
    while let Some(directory) = stack.pop() {
        platform::validate_directory_security(&directory)?;
        let directory = fs::canonicalize(&directory).map_err(|error| {
            IrohaZipError::io_path("cannot resolve audited tree directory", &directory, error)
        })?;
        if !directory.starts_with(&root) {
            return Err(IrohaZipError::Policy(format!(
                "audited directory escaped its root: {}",
                directory.display()
            )));
        }
        platform::validate_directory_security(&directory)?;
        for entry in fs::read_dir(&directory).map_err(|error| {
            IrohaZipError::io_path("cannot read audited tree", &directory, error)
        })? {
            let entry = entry.map_err(|error| {
                IrohaZipError::io_path("cannot read audited tree entry", &directory, error)
            })?;
            let path = entry.path();
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
            platform::validate_extracted_entry_security(&path, &metadata)?;
            if metadata.is_dir() {
                stack.push(path);
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
        if metadata.is_dir() {
            platform::validate_extracted_entry_security(&path, &metadata)?;
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
    })
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
) -> Result<()> {
    platform::validate_directory_security(source_root)?;
    platform::validate_directory_security(target_root)?;
    let source_root = fs::canonicalize(source_root).map_err(|error| {
        IrohaZipError::io_path("cannot resolve copy source root", source_root, error)
    })?;
    let target_root = fs::canonicalize(target_root).map_err(|error| {
        IrohaZipError::io_path("cannot resolve copy target root", target_root, error)
    })?;
    let mut stack = vec![(source_root.clone(), target_root.clone())];
    let mut copied_files = BTreeSet::new();
    while let Some((source_dir, target_dir)) = stack.pop() {
        platform::validate_directory_security(&source_dir)?;
        platform::validate_directory_security(&target_dir)?;
        let source_dir = fs::canonicalize(&source_dir).map_err(|error| {
            IrohaZipError::io_path(
                "cannot resolve audited source directory",
                &source_dir,
                error,
            )
        })?;
        let target_dir = fs::canonicalize(&target_dir).map_err(|error| {
            IrohaZipError::io_path("cannot resolve staged target directory", &target_dir, error)
        })?;
        if !source_dir.starts_with(&source_root) || !target_dir.starts_with(&target_root) {
            return Err(IrohaZipError::Policy(
                "source or target directory escaped its audited root".to_owned(),
            ));
        }
        platform::validate_directory_security(&source_dir)?;
        platform::validate_directory_security(&target_dir)?;
        for entry in fs::read_dir(&source_dir).map_err(|error| {
            IrohaZipError::io_path("cannot read audited source tree", &source_dir, error)
        })? {
            let entry = entry.map_err(|error| {
                IrohaZipError::io_path("cannot read audited source entry", &source_dir, error)
            })?;
            policy::validate_component(&entry.file_name())?;
            let source = entry.path();
            let target = target_dir.join(entry.file_name());
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
                fs::create_dir(&target).map_err(|error| {
                    IrohaZipError::io_path("cannot create staged directory", &target, error)
                })?;
                stack.push((source, target));
            } else if metadata.is_file() {
                let mut snapshot = AuditedFile::open(&source, limits.max_single_file_bytes)?;
                if !snapshot.path().starts_with(&source_root) {
                    return Err(IrohaZipError::Policy(format!(
                        "copy source escaped its audited root: {}",
                        source.display()
                    )));
                }
                let relative = source.strip_prefix(&source_root).map_err(|_| {
                    IrohaZipError::Policy(format!(
                        "copy source escaped its audited root: {}",
                        source.display()
                    ))
                })?;
                let expected = expected_files.get(relative).ok_or_else(|| {
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
                copied_files.insert(relative.to_path_buf());
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
    Ok(())
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
}
