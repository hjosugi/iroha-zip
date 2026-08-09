use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Result, SafeArcError};
use crate::platform;
use crate::policy::{AuditSummary, Limits};

pub fn check_resource_limits(root: &Path, limits: &Limits) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    platform::validate_directory_security(root)?;

    let mut files = 0u64;
    let mut directories = 0u64;
    let mut total_bytes = 0u64;
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    while let Some(directory) = stack.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(SafeArcError::io_path(
                    "cannot monitor extraction directory",
                    &directory,
                    error,
                ));
            }
        };

        for entry in entries {
            let entry = entry.map_err(|error| {
                SafeArcError::io_path("cannot monitor extraction entry", &directory, error)
            })?;
            let path = entry.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(SafeArcError::io_path(
                        "cannot monitor extracted object",
                        &path,
                        error,
                    ));
                }
            };

            if metadata.file_type().is_symlink() {
                return Err(SafeArcError::Policy(format!(
                    "link created during extraction: {}",
                    path.display()
                )));
            }
            platform::validate_extracted_entry_security(&path, &metadata)?;
            if metadata.is_dir() {
                directories = directories
                    .checked_add(1)
                    .ok_or_else(|| SafeArcError::Policy("directory count overflow".to_owned()))?;
                if directories > limits.max_directories {
                    return Err(SafeArcError::Policy(format!(
                        "directory count exceeded {} while extracting",
                        limits.max_directories
                    )));
                }
                stack.push(path);
            } else if metadata.is_file() {
                files = files
                    .checked_add(1)
                    .ok_or_else(|| SafeArcError::Policy("file count overflow".to_owned()))?;
                if files > limits.max_files {
                    return Err(SafeArcError::Policy(format!(
                        "file count exceeded {} while extracting",
                        limits.max_files
                    )));
                }
                let size = metadata.len();
                if size > limits.max_single_file_bytes {
                    return Err(SafeArcError::Policy(format!(
                        "single file exceeded {} bytes while extracting: {}",
                        limits.max_single_file_bytes,
                        path.display()
                    )));
                }
                total_bytes = total_bytes
                    .checked_add(size)
                    .ok_or_else(|| SafeArcError::Policy("expanded size overflow".to_owned()))?;
                if total_bytes > limits.max_total_bytes {
                    return Err(SafeArcError::Policy(format!(
                        "expanded data exceeded {} bytes while extracting",
                        limits.max_total_bytes
                    )));
                }
            } else {
                return Err(SafeArcError::Policy(format!(
                    "special object created during extraction: {}",
                    path.display()
                )));
            }
        }
    }

    Ok(())
}

pub fn limits_with_baseline(
    baseline: &AuditSummary,
    extra_files: u64,
    extra_directories: u64,
    extra_bytes: u64,
    max_single_file_bytes: u64,
) -> Result<Limits> {
    Ok(Limits {
        max_archive_bytes: u64::MAX,
        max_files: checked_budget_add(baseline.files, extra_files, "file budget")?,
        max_directories: checked_budget_add(
            baseline.directories,
            extra_directories,
            "directory budget",
        )?,
        max_total_bytes: checked_budget_add(baseline.total_bytes, extra_bytes, "byte budget")?,
        max_single_file_bytes,
        max_depth: usize::MAX,
        max_path_bytes: usize::MAX,
    })
}

fn checked_budget_add(left: u64, right: u64, label: &str) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| SafeArcError::Config(format!("{label} overflow")))
}
