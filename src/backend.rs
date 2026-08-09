use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{Result, SafeArcError};
use crate::platform;

const MANIFEST_FILE: &str = "backend-manifest.tsv";
const MANIFEST_HEADER: &str = "SAFEARC-BACKEND-MANIFEST\t1";

#[derive(Clone, Debug)]
pub struct BackendBundle {
    root: PathBuf,
    executable: PathBuf,
    files: BTreeMap<PathBuf, String>,
}

impl BackendBundle {
    pub fn verify(root: &Path) -> Result<Self> {
        platform::validate_directory_security(root).map_err(|error| {
            SafeArcError::Backend(format!(
                "invalid backend directory {}: {error}",
                root.display()
            ))
        })?;
        let root = fs::canonicalize(root)
            .map_err(|error| SafeArcError::io_path("cannot open backend directory", root, error))?;
        platform::validate_directory_security(&root).map_err(|error| {
            SafeArcError::Backend(format!(
                "invalid resolved backend directory {}: {error}",
                root.display()
            ))
        })?;

        let manifest_path = root.join(MANIFEST_FILE);
        let manifest_metadata = fs::symlink_metadata(&manifest_path).map_err(|error| {
            SafeArcError::io_path("cannot inspect backend manifest", &manifest_path, error)
        })?;
        if !manifest_metadata.is_file() || manifest_metadata.file_type().is_symlink() {
            return Err(SafeArcError::Backend(format!(
                "backend manifest must be a regular file: {}",
                manifest_path.display()
            )));
        }
        platform::validate_extracted_entry_security(&manifest_path, &manifest_metadata)
            .map_err(|error| SafeArcError::Backend(format!("invalid backend manifest: {error}")))?;
        let (executable, files) = parse_manifest(&manifest_path)?;
        if !files.contains_key(&executable) {
            return Err(SafeArcError::Backend(format!(
                "manifest executable is not listed as a hashed file: {}",
                executable.display()
            )));
        }

        let actual = collect_files(&root)?;
        let expected: BTreeSet<PathBuf> = files.keys().cloned().collect();
        if actual != expected {
            let unexpected: Vec<String> = actual
                .difference(&expected)
                .map(|path| path.display().to_string())
                .collect();
            let missing: Vec<String> = expected
                .difference(&actual)
                .map(|path| path.display().to_string())
                .collect();
            return Err(SafeArcError::Backend(format!(
                "backend directory does not exactly match its manifest; unexpected={unexpected:?}, missing={missing:?}"
            )));
        }

        for (relative, expected_hash) in &files {
            let path = root.join(relative);
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                SafeArcError::io_path("cannot inspect backend file", &path, error)
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(SafeArcError::Backend(format!(
                    "backend entry must be a regular file: {}",
                    path.display()
                )));
            }
            platform::validate_extracted_entry_security(&path, &metadata).map_err(|error| {
                SafeArcError::Backend(format!("unsafe backend entry {}: {error}", path.display()))
            })?;
            let actual_hash = sha256_file(&path)?;
            if !actual_hash.eq_ignore_ascii_case(expected_hash) {
                return Err(SafeArcError::Backend(format!(
                    "SHA-256 mismatch for {}: expected {}, got {}",
                    relative.display(),
                    expected_hash,
                    actual_hash
                )));
            }
        }

        let executable_path = root.join(&executable);
        Ok(Self {
            root,
            executable: executable_path,
            files,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn executable_relative(&self) -> Result<&Path> {
        self.executable.strip_prefix(&self.root).map_err(|_| {
            SafeArcError::Backend("backend executable escaped backend root".to_owned())
        })
    }

    pub fn copy_verified_to(&self, destination: &Path) -> Result<PathBuf> {
        fs::create_dir_all(destination).map_err(|error| {
            SafeArcError::io_path(
                "cannot create sandbox backend directory",
                destination,
                error,
            )
        })?;

        for (relative, expected_hash) in &self.files {
            let source = self.root.join(relative);
            let target = destination.join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    SafeArcError::io_path(
                        "cannot create sandbox backend subdirectory",
                        parent,
                        error,
                    )
                })?;
            }
            let source_metadata = fs::symlink_metadata(&source).map_err(|error| {
                SafeArcError::io_path("cannot inspect backend source", &source, error)
            })?;
            copy_file_new_exact(&source, &target, source_metadata.len())?;
            let copied_hash = sha256_file(&target)?;
            if !copied_hash.eq_ignore_ascii_case(expected_hash) {
                return Err(SafeArcError::Backend(format!(
                    "backend changed while being copied: {}",
                    relative.display()
                )));
            }
        }

        Ok(destination.join(self.executable_relative()?))
    }
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)
        .map_err(|error| SafeArcError::io_path("cannot open file for SHA-256", path, error))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| SafeArcError::io_path("cannot hash file", path, error))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn parse_manifest(path: &Path) -> Result<(PathBuf, BTreeMap<PathBuf, String>)> {
    let file = File::open(path)
        .map_err(|error| SafeArcError::io_path("cannot open backend manifest", path, error))?;
    let mut lines = BufReader::new(file).lines();
    let header = lines
        .next()
        .transpose()
        .map_err(|error| SafeArcError::io_path("cannot read backend manifest", path, error))?
        .ok_or_else(|| SafeArcError::Backend("backend manifest is empty".to_owned()))?;
    if header != MANIFEST_HEADER {
        return Err(SafeArcError::Backend(format!(
            "unsupported backend manifest header: {header:?}"
        )));
    }

    let mut executable = None;
    let mut files = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let line = line
            .map_err(|error| SafeArcError::io_path("cannot read backend manifest", path, error))?;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["executable", value] => {
                let relative = validate_manifest_path(value)?;
                if executable.replace(relative).is_some() {
                    return Err(SafeArcError::Backend(
                        "backend manifest contains multiple executable entries".to_owned(),
                    ));
                }
            }
            ["sha256", hash, value] => {
                if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(SafeArcError::Backend(format!(
                        "invalid SHA-256 on manifest line {}",
                        index + 2
                    )));
                }
                let relative = validate_manifest_path(value)?;
                if files.insert(relative.clone(), (*hash).to_owned()).is_some() {
                    return Err(SafeArcError::Backend(format!(
                        "duplicate manifest path: {}",
                        relative.display()
                    )));
                }
            }
            _ => {
                return Err(SafeArcError::Backend(format!(
                    "invalid backend manifest line {}: {line:?}",
                    index + 2
                )));
            }
        }
    }

    let executable = executable.ok_or_else(|| {
        SafeArcError::Backend("backend manifest has no executable entry".to_owned())
    })?;
    if files.is_empty() {
        return Err(SafeArcError::Backend(
            "backend manifest has no hashed files".to_owned(),
        ));
    }
    Ok((executable, files))
}

fn validate_manifest_path(value: &str) -> Result<PathBuf> {
    if value.is_empty()
        || value
            .chars()
            .any(|character| matches!(character, '\0' | '\t' | '\r' | '\n' | ':' | '\\'))
        || value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(SafeArcError::Backend(format!(
            "invalid or non-normalized manifest path: {value:?}"
        )));
    }
    let path = PathBuf::from(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(SafeArcError::Backend(format!(
            "manifest path must be relative and normalized: {value:?}"
        )));
    }
    Ok(path)
}

fn collect_files(root: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| SafeArcError::io_path("cannot enumerate backend", &directory, error))?
        {
            let entry = entry.map_err(|error| {
                SafeArcError::io_path("cannot enumerate backend entry", &directory, error)
            })?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                SafeArcError::io_path("cannot inspect backend entry", &path, error)
            })?;
            if metadata.file_type().is_symlink() {
                return Err(SafeArcError::Backend(format!(
                    "backend symlinks are forbidden: {}",
                    path.display()
                )));
            }
            platform::validate_extracted_entry_security(&path, &metadata).map_err(|error| {
                SafeArcError::Backend(format!("unsafe backend entry {}: {error}", path.display()))
            })?;
            if metadata.is_dir() {
                stack.push(path);
            } else if metadata.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| SafeArcError::Backend("backend file escaped root".to_owned()))?;
                if relative != Path::new(MANIFEST_FILE) {
                    files.insert(relative.to_path_buf());
                }
            } else {
                return Err(SafeArcError::Backend(format!(
                    "backend contains a special file: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(files)
}

fn copy_file_new_exact(source: &Path, target: &Path, expected_size: u64) -> Result<()> {
    let mut input = File::open(source)
        .map_err(|error| SafeArcError::io_path("cannot open backend source", source, error))?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|error| {
            SafeArcError::io_path("cannot create sandbox backend file", target, error)
        })?;

    let read_limit = expected_size.saturating_add(1);
    let copy_result = std::io::copy(&mut input.by_ref().take(read_limit), &mut output);
    let copied = match copy_result {
        Ok(copied) => copied,
        Err(error) => {
            drop(output);
            let _ = fs::remove_file(target);
            return Err(SafeArcError::io_path(
                "cannot copy backend file",
                target,
                error,
            ));
        }
    };
    if copied != expected_size {
        drop(output);
        let _ = fs::remove_file(target);
        return Err(SafeArcError::Backend(format!(
            "backend file changed while being copied: {}",
            source.display()
        )));
    }
    output
        .sync_all()
        .map_err(|error| SafeArcError::io_path("cannot flush backend file", target, error))?;
    Ok(())
}
