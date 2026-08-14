use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{IrohaZipError, Result};
use crate::platform;
use crate::policy;
use crate::util::hex_lower;

pub(crate) const MANIFEST_FILE: &str = "backend-manifest.tsv";
const MANIFEST_HEADER: &str = "IROHA-ZIP-BACKEND-MANIFEST\t1";
pub const MAX_BACKEND_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_BACKEND_MANIFEST_FILES: usize = 4096;
const MAX_BACKEND_MANIFEST_PATH_BYTES: usize = 4096;
const MAX_BACKEND_MANIFEST_PATH_DEPTH: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendManifest {
    executable: PathBuf,
    files: BTreeMap<PathBuf, String>,
}

impl BackendManifest {
    pub fn parse(input: &[u8]) -> Result<Self> {
        if input.len() > MAX_BACKEND_MANIFEST_BYTES {
            return Err(IrohaZipError::Backend(format!(
                "backend manifest exceeds the {MAX_BACKEND_MANIFEST_BYTES} byte limit"
            )));
        }
        let text = std::str::from_utf8(input).map_err(|error| {
            IrohaZipError::Backend(format!("backend manifest is not valid UTF-8: {error}"))
        })?;
        let mut lines = text.lines();
        let header = lines
            .next()
            .ok_or_else(|| IrohaZipError::Backend("backend manifest is empty".to_owned()))?;
        if header != MANIFEST_HEADER {
            return Err(IrohaZipError::Backend(format!(
                "unsupported backend manifest header: {header:?}"
            )));
        }

        let mut executable = None;
        let mut files = BTreeMap::new();
        for (index, line) in lines.enumerate() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let line_number = index + 2;
            let mut fields = line.split('\t');
            match (fields.next(), fields.next(), fields.next(), fields.next()) {
                (Some("executable"), Some(value), None, None) => {
                    let relative = validate_manifest_path(value)?;
                    if executable.replace(relative).is_some() {
                        return Err(IrohaZipError::Backend(
                            "backend manifest contains multiple executable entries".to_owned(),
                        ));
                    }
                }
                (Some("sha256"), Some(hash), Some(value), None) => {
                    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                        return Err(IrohaZipError::Backend(format!(
                            "invalid SHA-256 on manifest line {line_number}"
                        )));
                    }
                    let relative = validate_manifest_path(value)?;
                    if files.len() >= MAX_BACKEND_MANIFEST_FILES {
                        return Err(IrohaZipError::Backend(format!(
                            "backend manifest exceeds the {MAX_BACKEND_MANIFEST_FILES} file limit"
                        )));
                    }
                    if files
                        .insert(relative.clone(), hash.to_ascii_lowercase())
                        .is_some()
                    {
                        return Err(IrohaZipError::Backend(format!(
                            "duplicate manifest path: {}",
                            relative.display()
                        )));
                    }
                }
                _ => {
                    return Err(IrohaZipError::Backend(format!(
                        "invalid backend manifest line {line_number}: {line:?}"
                    )));
                }
            }
        }

        let executable = executable.ok_or_else(|| {
            IrohaZipError::Backend("backend manifest has no executable entry".to_owned())
        })?;
        if files.is_empty() {
            return Err(IrohaZipError::Backend(
                "backend manifest has no hashed files".to_owned(),
            ));
        }
        if !files.contains_key(&executable) {
            return Err(IrohaZipError::Backend(format!(
                "manifest executable is not listed as a hashed file: {}",
                executable.display()
            )));
        }
        Ok(Self { executable, files })
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn file_hash(&self, path: &Path) -> Option<&str> {
        self.files.get(path).map(String::as_str)
    }
}

#[derive(Clone, Debug)]
pub struct BackendBundle {
    root: PathBuf,
    executable: PathBuf,
    files: BTreeMap<PathBuf, String>,
}

impl BackendBundle {
    pub fn verify(root: &Path) -> Result<Self> {
        platform::validate_directory_security(root).map_err(|error| {
            IrohaZipError::Backend(format!(
                "invalid backend directory {}: {error}",
                root.display()
            ))
        })?;
        let root = fs::canonicalize(root).map_err(|error| {
            IrohaZipError::io_path("cannot open backend directory", root, error)
        })?;
        platform::validate_directory_security(&root).map_err(|error| {
            IrohaZipError::Backend(format!(
                "invalid resolved backend directory {}: {error}",
                root.display()
            ))
        })?;

        let manifest_path = root.join(MANIFEST_FILE);
        let manifest_metadata = fs::symlink_metadata(&manifest_path).map_err(|error| {
            IrohaZipError::io_path("cannot inspect backend manifest", &manifest_path, error)
        })?;
        if !manifest_metadata.is_file() || manifest_metadata.file_type().is_symlink() {
            return Err(IrohaZipError::Backend(format!(
                "backend manifest must be a regular file: {}",
                manifest_path.display()
            )));
        }
        platform::validate_extracted_entry_security(&manifest_path, &manifest_metadata).map_err(
            |error| IrohaZipError::Backend(format!("invalid backend manifest: {error}")),
        )?;
        let manifest = read_manifest(&manifest_path)?;

        let actual = collect_files(&root)?;
        let expected: BTreeSet<PathBuf> = manifest.files.keys().cloned().collect();
        if actual != expected {
            let unexpected: Vec<String> = actual
                .difference(&expected)
                .map(|path| path.display().to_string())
                .collect();
            let missing: Vec<String> = expected
                .difference(&actual)
                .map(|path| path.display().to_string())
                .collect();
            return Err(IrohaZipError::Backend(format!(
                "backend directory does not exactly match its manifest; unexpected={unexpected:?}, missing={missing:?}"
            )));
        }

        for (relative, expected_hash) in &manifest.files {
            let path = root.join(relative);
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                IrohaZipError::io_path("cannot inspect backend file", &path, error)
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(IrohaZipError::Backend(format!(
                    "backend entry must be a regular file: {}",
                    path.display()
                )));
            }
            platform::validate_extracted_entry_security(&path, &metadata).map_err(|error| {
                IrohaZipError::Backend(format!("unsafe backend entry {}: {error}", path.display()))
            })?;
            let actual_hash = sha256_file(&path)?;
            if !actual_hash.eq_ignore_ascii_case(expected_hash) {
                return Err(IrohaZipError::Backend(format!(
                    "SHA-256 mismatch for {}: expected {}, got {}",
                    relative.display(),
                    expected_hash,
                    actual_hash
                )));
            }
        }

        let executable_path = root.join(&manifest.executable);
        let bundle = Self {
            root,
            executable: executable_path,
            files: manifest.files,
        };
        if bundle
            .root
            .join(crate::backend_evidence::EVIDENCE_DIRECTORY)
            .exists()
        {
            crate::backend_evidence::BackendEvidence::verify(&bundle)?;
        }
        Ok(bundle)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn files(&self) -> impl ExactSizeIterator<Item = (&Path, &str)> {
        self.files
            .iter()
            .map(|(path, hash)| (path.as_path(), hash.as_str()))
    }

    pub fn executable_relative(&self) -> Result<&Path> {
        self.executable.strip_prefix(&self.root).map_err(|_| {
            IrohaZipError::Backend("backend executable escaped backend root".to_owned())
        })
    }

    pub fn copied_entry_count(&self) -> Result<u64> {
        let mut directories = BTreeSet::<PathBuf>::new();
        for relative in self.files.keys() {
            let mut parent = relative.parent();
            while let Some(path) = parent {
                if path.as_os_str().is_empty() {
                    break;
                }
                directories.insert(path.to_path_buf());
                parent = path.parent();
            }
        }
        u64::try_from(self.files.len())
            .ok()
            .and_then(|files| {
                u64::try_from(directories.len())
                    .ok()
                    .and_then(|count| files.checked_add(count))
            })
            .ok_or_else(|| IrohaZipError::Backend("backend entry count overflow".to_owned()))
    }

    pub fn copy_verified_to(&self, destination: &Path) -> Result<PathBuf> {
        fs::create_dir_all(destination).map_err(|error| {
            IrohaZipError::io_path(
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
                    IrohaZipError::io_path(
                        "cannot create sandbox backend subdirectory",
                        parent,
                        error,
                    )
                })?;
            }
            let source_metadata = fs::symlink_metadata(&source).map_err(|error| {
                IrohaZipError::io_path("cannot inspect backend source", &source, error)
            })?;
            copy_file_new_exact(&source, &target, source_metadata.len())?;
            #[cfg(unix)]
            if relative == self.executable_relative()? {
                use std::os::unix::fs::PermissionsExt;

                fs::set_permissions(&target, fs::Permissions::from_mode(0o500)).map_err(
                    |error| {
                        IrohaZipError::io_path(
                            "cannot make sandbox backend executable",
                            &target,
                            error,
                        )
                    },
                )?;
            }
            let copied_hash = sha256_file(&target)?;
            if !copied_hash.eq_ignore_ascii_case(expected_hash) {
                return Err(IrohaZipError::Backend(format!(
                    "backend changed while being copied: {}",
                    relative.display()
                )));
            }
        }

        let executable = destination.join(self.executable_relative()?);
        platform::prepare_backend_executable(&executable)?;
        Ok(executable)
    }
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)
        .map_err(|error| IrohaZipError::io_path("cannot open file for SHA-256", path, error))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| IrohaZipError::io_path("cannot hash file", path, error))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_lower(hasher.finalize()))
}

fn read_manifest(path: &Path) -> Result<BackendManifest> {
    let mut file = File::open(path)
        .map_err(|error| IrohaZipError::io_path("cannot open backend manifest", path, error))?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_BACKEND_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| IrohaZipError::io_path("cannot read backend manifest", path, error))?;
    BackendManifest::parse(&bytes)
}

pub(crate) fn validate_manifest_path(value: &str) -> Result<PathBuf> {
    if value.is_empty()
        || value.len() > MAX_BACKEND_MANIFEST_PATH_BYTES
        || value
            .chars()
            .any(|character| matches!(character, '\0' | '\t' | '\r' | '\n' | ':' | '\\'))
    {
        return Err(IrohaZipError::Backend(format!(
            "invalid or non-normalized manifest path: {value:?}"
        )));
    }
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() > MAX_BACKEND_MANIFEST_PATH_DEPTH
        || parts
            .iter()
            .any(|part| part.is_empty() || matches!(*part, "." | ".."))
    {
        return Err(IrohaZipError::Backend(format!(
            "invalid or non-normalized manifest path: {value:?}"
        )));
    }
    for part in &parts {
        policy::validate_component(std::ffi::OsStr::new(part)).map_err(|_| {
            IrohaZipError::Backend(format!(
                "manifest path contains an invalid Windows filename: {value:?}"
            ))
        })?;
    }
    let path = PathBuf::from(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(IrohaZipError::Backend(format!(
            "manifest path must be relative and normalized: {value:?}"
        )));
    }
    Ok(path)
}

fn collect_files(root: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).map_err(|error| {
            IrohaZipError::io_path("cannot enumerate backend", &directory, error)
        })? {
            let entry = entry.map_err(|error| {
                IrohaZipError::io_path("cannot enumerate backend entry", &directory, error)
            })?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                IrohaZipError::io_path("cannot inspect backend entry", &path, error)
            })?;
            if metadata.file_type().is_symlink() {
                return Err(IrohaZipError::Backend(format!(
                    "backend symlinks are forbidden: {}",
                    path.display()
                )));
            }
            platform::validate_extracted_entry_security(&path, &metadata).map_err(|error| {
                IrohaZipError::Backend(format!("unsafe backend entry {}: {error}", path.display()))
            })?;
            if metadata.is_dir()
                && directory == root
                && entry.file_name()
                    == std::ffi::OsStr::new(crate::backend_evidence::EVIDENCE_DIRECTORY)
            {
                continue;
            }
            if metadata.is_dir() {
                stack.push(path);
            } else if metadata.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| IrohaZipError::Backend("backend file escaped root".to_owned()))?;
                if relative != Path::new(MANIFEST_FILE) {
                    files.insert(relative.to_path_buf());
                }
            } else {
                return Err(IrohaZipError::Backend(format!(
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
        .map_err(|error| IrohaZipError::io_path("cannot open backend source", source, error))?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|error| {
            IrohaZipError::io_path("cannot create sandbox backend file", target, error)
        })?;

    let read_limit = expected_size.saturating_add(1);
    let copy_result = std::io::copy(&mut input.by_ref().take(read_limit), &mut output);
    let copied = match copy_result {
        Ok(copied) => copied,
        Err(error) => {
            drop(output);
            let _ = fs::remove_file(target);
            return Err(IrohaZipError::io_path(
                "cannot copy backend file",
                target,
                error,
            ));
        }
    };
    if copied != expected_size {
        drop(output);
        let _ = fs::remove_file(target);
        return Err(IrohaZipError::Backend(format!(
            "backend file changed while being copied: {}",
            source.display()
        )));
    }
    output
        .sync_all()
        .map_err(|error| IrohaZipError::io_path("cannot flush backend file", target, error))?;
    Ok(())
}
