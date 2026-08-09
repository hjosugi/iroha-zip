use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{IrohaZipError, Result};
use crate::policy::{self, AuditSummary, Limits};
use crate::snapshot::AuditedFile;
use crate::{platform, transfer};

pub fn materialize_selection(
    source_root: &Path,
    target_root: &Path,
    selectors: &[PathBuf],
    limits: &Limits,
) -> Result<AuditSummary> {
    let selectors = validate_selectors(selectors, limits)?;
    platform::validate_directory_security(source_root)?;
    let source_root = fs::canonicalize(source_root).map_err(|error| {
        IrohaZipError::io_path("cannot resolve selection source", source_root, error)
    })?;
    platform::validate_directory_security(&source_root)?;
    let target_root = resolve_new_target(target_root)?;
    if target_root.starts_with(&source_root) || source_root.starts_with(&target_root) {
        return Err(IrohaZipError::Policy(
            "selection source and target must not contain each other".to_owned(),
        ));
    }
    if target_root.exists() {
        return Err(IrohaZipError::Usage(format!(
            "selection target already exists: {}",
            target_root.display()
        )));
    }

    let before = transfer::fingerprint_tree(&source_root, limits)?;
    let result = (|| {
        fs::create_dir(&target_root).map_err(|error| {
            IrohaZipError::io_path("cannot create selection target", &target_root, error)
        })?;
        for selector in &selectors {
            copy_selector(&source_root, &target_root, selector, limits)?;
        }
        let selected = transfer::fingerprint_tree(&target_root, limits)?;
        let after = transfer::fingerprint_tree(&source_root, limits)?;
        if after != before {
            return Err(IrohaZipError::Policy(
                "staged archive changed while selected entries were copied".to_owned(),
            ));
        }
        Ok(selected.summary().clone())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&target_root);
    }
    result
}

fn validate_selectors(selectors: &[PathBuf], limits: &Limits) -> Result<Vec<PathBuf>> {
    if selectors.is_empty() {
        return Err(IrohaZipError::Usage(
            "at least one selection path is required".to_owned(),
        ));
    }
    let maximum = limits
        .max_files
        .checked_add(limits.max_directories)
        .ok_or_else(|| IrohaZipError::Config("selection count limit overflow".to_owned()))?;
    if u64::try_from(selectors.len()).unwrap_or(u64::MAX) > maximum {
        return Err(IrohaZipError::Policy(format!(
            "selection count exceeds {maximum}"
        )));
    }

    let mut unique = BTreeSet::new();
    let mut keys = BTreeMap::new();
    for selector in selectors {
        policy::validate_relative_path(selector, limits)?;
        let text = selector.to_str().ok_or_else(|| {
            IrohaZipError::Policy(format!(
                "non-Unicode selection path is rejected: {}",
                selector.display()
            ))
        })?;
        if text
            .split(['/', '\\'])
            .any(|component| component.is_empty() || component == "." || component == "..")
        {
            return Err(IrohaZipError::Usage(format!(
                "selection path is not normalized: {}",
                selector.display()
            )));
        }
        if !unique.insert(selector.clone()) {
            return Err(IrohaZipError::Usage(format!(
                "duplicate selection path: {}",
                selector.display()
            )));
        }
        let key = selector
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
            .collect::<Vec<_>>();
        if let Some(previous) = keys.insert(key, selector.clone()) {
            return Err(IrohaZipError::Usage(format!(
                "case-insensitive duplicate selection paths are ambiguous: {} and {}",
                previous.display(),
                selector.display()
            )));
        }
    }
    for (key, selector) in &keys {
        for length in 1..key.len() {
            if let Some(ancestor) = keys.get(&key[..length]) {
                return Err(IrohaZipError::Usage(format!(
                    "overlapping selection paths are ambiguous: {} and {}",
                    ancestor.display(),
                    selector.display()
                )));
            }
        }
    }
    Ok(unique.into_iter().collect())
}

fn resolve_new_target(target: &Path) -> Result<PathBuf> {
    let absolute = if target.is_absolute() {
        target.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| IrohaZipError::io("cannot read current directory", error))?
            .join(target)
    };
    let name = absolute.file_name().ok_or_else(|| {
        IrohaZipError::Usage(format!(
            "selection target has no final name: {}",
            target.display()
        ))
    })?;
    policy::validate_component(name)?;
    let parent = absolute.parent().ok_or_else(|| {
        IrohaZipError::Usage(format!(
            "selection target has no parent: {}",
            target.display()
        ))
    })?;
    platform::validate_directory_security(parent)?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        IrohaZipError::io_path("cannot resolve selection target parent", parent, error)
    })?;
    platform::validate_directory_security(&parent)?;
    Ok(parent.join(name))
}

fn copy_selector(
    source_root: &Path,
    target_root: &Path,
    selector: &Path,
    limits: &Limits,
) -> Result<()> {
    let source = source_root.join(selector);
    let target = target_root.join(selector);
    let metadata = fs::symlink_metadata(&source).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            IrohaZipError::Usage(format!(
                "selected path does not exist in the policy-safe preview: {}",
                selector.display()
            ))
        } else {
            IrohaZipError::io_path("cannot inspect selected path", &source, error)
        }
    })?;
    platform::validate_extracted_entry_security(&source, &metadata)?;
    let parent = target.parent().ok_or_else(|| {
        IrohaZipError::Policy(format!(
            "selected path has no target parent: {}",
            selector.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        IrohaZipError::io_path("cannot create selected path parent", parent, error)
    })?;

    if metadata.is_dir() {
        transfer::copy_audited_tree(&source, &target, limits)?;
    } else if metadata.is_file() {
        let mut snapshot = AuditedFile::open(&source, limits.max_single_file_bytes)?;
        snapshot.copy_to_new(&target)?;
    } else {
        return Err(IrohaZipError::Policy(format!(
            "selected path is not a regular file or directory: {}",
            selector.display()
        )));
    }
    Ok(())
}
