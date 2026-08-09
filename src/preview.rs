use std::fs;
use std::path::{Path, PathBuf};

use crate::backend::BackendBundle;
use crate::config::{Config, FilenameEncoding};
use crate::error::{IrohaZipError, Result};
use crate::policy::{self, AuditSummary, Limits};
use crate::{platform, staging, transfer};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveEntryKind {
    File,
    Directory,
}

impl ArchiveEntryKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveEntry {
    pub path: PathBuf,
    pub kind: ArchiveEntryKind,
    pub size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewResult {
    pub entries: Vec<ArchiveEntry>,
    pub summary: AuditSummary,
}

pub struct PreviewRequest<'a> {
    pub backend: &'a BackendBundle,
    pub config: &'a Config,
    pub archive: &'a Path,
    pub encoding: FilenameEncoding,
    pub allow_unsandboxed: bool,
}

pub fn preview(request: PreviewRequest<'_>) -> Result<PreviewResult> {
    let archive = policy::open_input_archive(request.archive, &request.config.limits)?;
    let staged = staging::stage_archive(
        request.backend,
        request.config,
        archive,
        request.encoding,
        request.allow_unsandboxed,
    )?;
    inventory_tree(staged.payload_root(), &request.config.limits)
}

pub fn inventory_tree(root: &Path, limits: &Limits) -> Result<PreviewResult> {
    let before = transfer::fingerprint_tree(root, limits)?;
    let root = fs::canonicalize(root)
        .map_err(|error| IrohaZipError::io_path("cannot resolve preview root", root, error))?;
    platform::validate_directory_security(&root)?;
    let mut entries = Vec::new();
    let mut stack = vec![root.clone()];

    while let Some(directory) = stack.pop() {
        platform::validate_directory_security(&directory)?;
        for entry in fs::read_dir(&directory).map_err(|error| {
            IrohaZipError::io_path("cannot read preview directory", &directory, error)
        })? {
            let entry = entry.map_err(|error| {
                IrohaZipError::io_path("cannot read preview entry", &directory, error)
            })?;
            let path = entry.path();
            let relative = path
                .strip_prefix(&root)
                .map_err(|_| {
                    IrohaZipError::Policy(format!(
                        "preview entry escaped its root: {}",
                        path.display()
                    ))
                })?
                .to_path_buf();
            policy::validate_relative_path(&relative, limits)?;
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                IrohaZipError::io_path("cannot inspect preview entry", &path, error)
            })?;
            platform::validate_extracted_entry_security(&path, &metadata)?;
            let (kind, size) = if metadata.is_dir() {
                stack.push(path);
                (ArchiveEntryKind::Directory, 0)
            } else if metadata.is_file() {
                (ArchiveEntryKind::File, metadata.len())
            } else {
                return Err(IrohaZipError::Policy(format!(
                    "special preview entry is not allowed: {}",
                    relative.display()
                )));
            };
            entries.push(ArchiveEntry {
                path: relative,
                kind,
                size,
            });
        }
    }

    entries.sort_by(|left, right| left.path.cmp(&right.path));
    let after = transfer::fingerprint_tree(&root, limits)?;
    if after != before {
        return Err(IrohaZipError::Policy(
            "staged archive changed while its preview was built".to_owned(),
        ));
    }
    Ok(PreviewResult {
        entries,
        summary: before.summary().clone(),
    })
}
