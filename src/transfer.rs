use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{IrohaZipError, Result};
use crate::platform;
use crate::policy::{self, AuditSummary, Limits};
use crate::util;

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
    let expected = policy::audit_tree(source_root, limits)?;
    fs::create_dir(target_root).map_err(|error| {
        IrohaZipError::io_path("cannot create staged output directory", target_root, error)
    })?;

    let copy_result = copy_tree(source_root, target_root);
    if let Err(error) = copy_result {
        let _ = fs::remove_dir_all(target_root);
        return Err(error);
    }

    let copied = match policy::audit_tree(target_root, limits) {
        Ok(summary) => summary,
        Err(error) => {
            let _ = fs::remove_dir_all(target_root);
            return Err(error);
        }
    };
    if copied != expected {
        let _ = fs::remove_dir_all(target_root);
        return Err(IrohaZipError::Policy(
            "source tree changed while it was being copied".to_owned(),
        ));
    }
    Ok(copied)
}

fn copy_tree(source_root: &Path, target_root: &Path) -> Result<()> {
    let mut stack = vec![(source_root.to_path_buf(), target_root.to_path_buf())];
    while let Some((source_dir, target_dir)) = stack.pop() {
        platform::validate_directory_security(&source_dir)?;
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
                util::copy_file_new_exact(&source, &target, metadata.len())?;
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
