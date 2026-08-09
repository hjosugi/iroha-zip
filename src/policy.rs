use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{IrohaZipError, Result};
use crate::platform;
use crate::snapshot::AuditedFile;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    /// Maximum size of the input archive itself.
    pub max_archive_bytes: u64,
    /// Maximum number of regular files in the extracted tree.
    pub max_files: u64,
    /// Maximum number of directories in the extracted tree.
    pub max_directories: u64,
    /// Maximum sum of extracted regular-file sizes.
    pub max_total_bytes: u64,
    /// Maximum size of a single extracted file.
    pub max_single_file_bytes: u64,
    /// Maximum relative path depth.
    pub max_depth: usize,
    /// Maximum UTF-8 byte length of a relative path.
    pub max_path_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_archive_bytes: 16 * 1024 * 1024 * 1024,
            max_files: 100_000,
            max_directories: 25_000,
            max_total_bytes: 32 * 1024 * 1024 * 1024,
            max_single_file_bytes: 8 * 1024 * 1024 * 1024,
            max_depth: 64,
            max_path_bytes: 4096,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuditSummary {
    pub files: u64,
    pub directories: u64,
    pub total_bytes: u64,
}

pub fn validate_input_archive(path: &Path, limits: &Limits) -> Result<PathBuf> {
    Ok(open_input_archive(path, limits)?.path().to_path_buf())
}

pub fn open_input_archive(path: &Path, limits: &Limits) -> Result<AuditedFile> {
    let snapshot = AuditedFile::open(path, limits.max_archive_bytes)?;
    if snapshot.fingerprint().length() == 0 {
        return Err(IrohaZipError::Policy(
            "empty input archive is rejected".to_owned(),
        ));
    }
    Ok(snapshot)
}

pub fn measure_tree(root: &Path) -> Result<AuditSummary> {
    let unbounded = Limits {
        max_archive_bytes: u64::MAX,
        max_files: u64::MAX,
        max_directories: u64::MAX,
        max_total_bytes: u64::MAX,
        max_single_file_bytes: u64::MAX,
        max_depth: usize::MAX,
        max_path_bytes: usize::MAX,
    };
    audit_tree(root, &unbounded)
}

pub fn audit_tree(root: &Path, limits: &Limits) -> Result<AuditSummary> {
    platform::validate_directory_security(root)?;
    let root = fs::canonicalize(root)
        .map_err(|error| IrohaZipError::io_path("cannot inspect extraction root", root, error))?;
    platform::validate_directory_security(&root)?;
    let mut summary = AuditSummary::default();
    let mut stack = vec![(root.clone(), 0usize)];
    let mut seen_file_ids = HashSet::new();

    while let Some((directory, depth)) = stack.pop() {
        if depth > limits.max_depth {
            return Err(IrohaZipError::Policy(format!(
                "directory depth exceeds {} at {}",
                limits.max_depth,
                directory.display()
            )));
        }

        let entries = fs::read_dir(&directory).map_err(|error| {
            IrohaZipError::io_path("cannot read extracted directory", &directory, error)
        })?;

        for entry in entries {
            let entry = entry.map_err(|error| {
                IrohaZipError::io_path("cannot read extracted directory entry", &directory, error)
            })?;
            let path = entry.path();
            let relative = path.strip_prefix(&root).map_err(|_| {
                IrohaZipError::Policy(format!(
                    "extracted path escaped the staging root: {}",
                    path.display()
                ))
            })?;

            validate_relative_path(relative, limits)?;

            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                IrohaZipError::io_path("cannot inspect extracted entry", &path, error)
            })?;
            let file_type = metadata.file_type();

            if file_type.is_symlink() {
                return Err(IrohaZipError::Policy(format!(
                    "symbolic links are not allowed: {}",
                    relative.display()
                )));
            }

            platform::validate_extracted_entry_security(&path, &metadata)?;

            if metadata.is_dir() {
                summary.directories = checked_add(summary.directories, 1, "directory count")?;
                if summary.directories > limits.max_directories {
                    return Err(IrohaZipError::Policy(format!(
                        "directory count exceeds {}",
                        limits.max_directories
                    )));
                }
                stack.push((path, depth + 1));
            } else if metadata.is_file() {
                summary.files = checked_add(summary.files, 1, "file count")?;
                if summary.files > limits.max_files {
                    return Err(IrohaZipError::Policy(format!(
                        "file count exceeds {}",
                        limits.max_files
                    )));
                }

                let size = metadata.len();
                if size > limits.max_single_file_bytes {
                    return Err(IrohaZipError::Policy(format!(
                        "single file exceeds {} bytes: {}",
                        limits.max_single_file_bytes,
                        relative.display()
                    )));
                }
                summary.total_bytes = checked_add(summary.total_bytes, size, "total size")?;
                if summary.total_bytes > limits.max_total_bytes {
                    return Err(IrohaZipError::Policy(format!(
                        "expanded data exceeds {} bytes",
                        limits.max_total_bytes
                    )));
                }

                if let Some(id) = platform::file_identity(&path)?
                    && !seen_file_ids.insert(id)
                {
                    return Err(IrohaZipError::Policy(format!(
                        "hard-linked or duplicate file identity detected: {}",
                        relative.display()
                    )));
                }
            } else {
                return Err(IrohaZipError::Policy(format!(
                    "special files are not allowed: {}",
                    relative.display()
                )));
            }
        }
    }

    Ok(summary)
}

pub fn validate_relative_path(path: &Path, limits: &Limits) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(IrohaZipError::Policy(format!(
            "invalid relative path: {}",
            path.display()
        )));
    }

    let mut depth = 0usize;
    let mut utf8_bytes = 0usize;
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                depth = depth.saturating_add(1);
                validate_component(name)?;
                let text = name.to_str().ok_or_else(|| {
                    IrohaZipError::Policy(format!(
                        "non-Unicode filenames are rejected: {}",
                        path.display()
                    ))
                })?;
                utf8_bytes = utf8_bytes
                    .checked_add(text.len() + 1)
                    .ok_or_else(|| IrohaZipError::Policy("path length overflow".to_owned()))?;
            }
            _ => {
                return Err(IrohaZipError::Policy(format!(
                    "path contains an absolute, parent, or prefix component: {}",
                    path.display()
                )));
            }
        }
    }

    if depth > limits.max_depth {
        return Err(IrohaZipError::Policy(format!(
            "path depth exceeds {}: {}",
            limits.max_depth,
            path.display()
        )));
    }
    if utf8_bytes > limits.max_path_bytes {
        return Err(IrohaZipError::Policy(format!(
            "path exceeds {} UTF-8 bytes: {}",
            limits.max_path_bytes,
            path.display()
        )));
    }
    Ok(())
}

pub fn validate_component(name: &OsStr) -> Result<()> {
    let text = name.to_str().ok_or_else(|| {
        IrohaZipError::Policy("non-Unicode filename component is rejected".to_owned())
    })?;

    if text.is_empty() || text == "." || text == ".." {
        return Err(IrohaZipError::Policy(format!(
            "invalid filename component: {text:?}"
        )));
    }
    if text.ends_with(' ') || text.ends_with('.') {
        return Err(IrohaZipError::Policy(format!(
            "Windows-trimmed trailing dot or space is rejected: {text:?}"
        )));
    }
    if text.contains(':') {
        return Err(IrohaZipError::Policy(format!(
            "colon and NTFS alternate-stream syntax are rejected: {text:?}"
        )));
    }
    if text.chars().any(|character| {
        character == '\0'
            || character.is_control()
            || matches!(character, '<' | '>' | '"' | '|' | '?' | '*' | '\\' | '/')
    }) {
        return Err(IrohaZipError::Policy(format!(
            "Windows-invalid character is rejected: {text:?}"
        )));
    }

    let stem = text.split('.').next().unwrap_or(text).trim_end();
    let upper = stem.to_ascii_uppercase();
    let reserved = matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" | "CLOCK$"
    ) || is_numbered_reserved(&upper, "COM")
        || is_numbered_reserved(&upper, "LPT")
        || matches!(
            upper.as_str(),
            "COM¹" | "COM²" | "COM³" | "LPT¹" | "LPT²" | "LPT³"
        );
    if reserved {
        return Err(IrohaZipError::Policy(format!(
            "Windows device name is rejected: {text:?}"
        )));
    }

    Ok(())
}

fn is_numbered_reserved(value: &str, prefix: &str) -> bool {
    value
        .strip_prefix(prefix)
        .and_then(|suffix| suffix.parse::<u8>().ok())
        .is_some_and(|number| (1..=9).contains(&number))
}

fn checked_add(left: u64, right: u64, label: &str) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| IrohaZipError::Policy(format!("{label} overflow")))
}
