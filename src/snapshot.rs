use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use crate::error::{IrohaZipError, Result};
use crate::platform::{self, FileIdentity};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileFingerprint {
    identity: Option<FileIdentity>,
    length: u64,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
    sha256: [u8; 32],
}

impl FileFingerprint {
    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn identity(&self) -> Option<&FileIdentity> {
        self.identity.as_ref()
    }

    pub fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

pub struct AuditedFile {
    path: PathBuf,
    file: File,
    fingerprint: FileFingerprint,
}

impl AuditedFile {
    pub fn open(path: &Path, max_bytes: u64) -> Result<Self> {
        platform::validate_regular_file_security(path)?;
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|error| IrohaZipError::io("cannot read current directory", error))?
                .join(path)
        };
        let file_name = absolute.file_name().ok_or_else(|| {
            IrohaZipError::Policy(format!(
                "snapshot source has no final filename: {}",
                path.display()
            ))
        })?;
        let parent = absolute.parent().ok_or_else(|| {
            IrohaZipError::Policy(format!("snapshot source has no parent: {}", path.display()))
        })?;
        platform::validate_directory_security(parent)?;
        let parent = fs::canonicalize(parent).map_err(|error| {
            IrohaZipError::io_path("cannot resolve snapshot source parent", parent, error)
        })?;
        platform::validate_directory_security(&parent)?;
        let path = parent.join(file_name);
        platform::validate_regular_file_security(&path)?;

        let mut file = platform::open_snapshot_source(&path)?;
        platform::validate_open_snapshot_source(&path, &file)?;
        let before = metadata_state(&path, &file)?;
        if before.length > max_bytes {
            return Err(IrohaZipError::Policy(format!(
                "snapshot source is {} bytes; limit is {max_bytes} bytes: {}",
                before.length,
                path.display()
            )));
        }
        verify_current_path(&path, before.identity.as_ref())?;

        let (length, sha256) = hash_open_file(&path, &mut file, max_bytes)?;
        let after = metadata_state(&path, &file)?;
        if before != after || length != before.length {
            return Err(changed_error(&path));
        }
        platform::validate_open_snapshot_source(&path, &file)?;
        verify_current_path(&path, after.identity.as_ref())?;
        file.seek(SeekFrom::Start(0)).map_err(|error| {
            IrohaZipError::io_path("cannot rewind snapshot source", &path, error)
        })?;

        Ok(Self {
            path,
            file,
            fingerprint: FileFingerprint {
                identity: after.identity,
                length,
                modified: after.modified,
                created: after.created,
                sha256,
            },
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn fingerprint(&self) -> &FileFingerprint {
        &self.fingerprint
    }

    pub fn copy_to_new(&mut self, target: &Path) -> Result<u64> {
        let result = self.copy_to_new_inner(target);
        if result.is_err() {
            let _ = fs::remove_file(target);
        }
        result
    }

    fn copy_to_new_inner(&mut self, target: &Path) -> Result<u64> {
        self.verify_unchanged()?;
        self.file.seek(SeekFrom::Start(0)).map_err(|error| {
            IrohaZipError::io_path("cannot rewind snapshot source", &self.path, error)
        })?;
        let mut output = platform::create_snapshot_target(target)?;
        let mut hasher = Sha256::new();
        let mut copied = 0u64;
        let mut buffer = [0u8; 128 * 1024];
        loop {
            let read = self.file.read(&mut buffer).map_err(|error| {
                IrohaZipError::io_path("cannot read snapshot source", &self.path, error)
            })?;
            if read == 0 {
                break;
            }
            copied = copied
                .checked_add(read as u64)
                .ok_or_else(|| changed_error(&self.path))?;
            if copied > self.fingerprint.length {
                return Err(changed_error(&self.path));
            }
            hasher.update(&buffer[..read]);
            output.write_all(&buffer[..read]).map_err(|error| {
                IrohaZipError::io_path("cannot write snapshot target", target, error)
            })?;
        }
        let copied_hash: [u8; 32] = hasher.finalize().into();
        if copied != self.fingerprint.length || copied_hash != self.fingerprint.sha256 {
            return Err(changed_error(&self.path));
        }
        output
            .flush()
            .and_then(|()| output.sync_all())
            .map_err(|error| {
                IrohaZipError::io_path("cannot flush snapshot target", target, error)
            })?;

        self.verify_unchanged()?;
        platform::validate_open_snapshot_source(target, &output)?;
        let target_state = metadata_state(target, &output)?;
        if target_state.length != self.fingerprint.length {
            return Err(IrohaZipError::Policy(format!(
                "snapshot target length mismatch: {}",
                target.display()
            )));
        }
        verify_current_path(target, target_state.identity.as_ref())?;
        let (target_length, target_hash) = hash_open_file(target, &mut output, copied)?;
        if target_length != copied || target_hash != copied_hash {
            return Err(IrohaZipError::Policy(format!(
                "snapshot target content mismatch: {}",
                target.display()
            )));
        }
        Ok(copied)
    }

    fn verify_unchanged(&self) -> Result<()> {
        platform::validate_open_snapshot_source(&self.path, &self.file)?;
        let current = metadata_state(&self.path, &self.file)?;
        if current.identity != self.fingerprint.identity
            || current.length != self.fingerprint.length
            || current.modified != self.fingerprint.modified
            || current.created != self.fingerprint.created
        {
            return Err(changed_error(&self.path));
        }
        verify_current_path(&self.path, self.fingerprint.identity.as_ref())
    }
}

#[derive(Eq, PartialEq)]
struct MetadataState {
    identity: Option<FileIdentity>,
    length: u64,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
}

fn metadata_state(path: &Path, file: &File) -> Result<MetadataState> {
    let metadata = file
        .metadata()
        .map_err(|error| IrohaZipError::io_path("cannot inspect snapshot handle", path, error))?;
    Ok(MetadataState {
        identity: platform::file_identity_from_handle(path, file)?,
        length: metadata.len(),
        modified: metadata.modified().ok(),
        created: metadata.created().ok(),
    })
}

fn verify_current_path(path: &Path, expected: Option<&FileIdentity>) -> Result<()> {
    platform::validate_regular_file_security(path)?;
    let current = platform::file_identity(path)?;
    if expected.is_some() && current.as_ref() != expected {
        return Err(changed_error(path));
    }
    Ok(())
}

fn hash_open_file(path: &Path, file: &mut File, max_bytes: u64) -> Result<(u64, [u8; 32])> {
    file.seek(SeekFrom::Start(0))
        .map_err(|error| IrohaZipError::io_path("cannot rewind file for SHA-256", path, error))?;
    let mut hasher = Sha256::new();
    let mut length = 0u64;
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| IrohaZipError::io_path("cannot hash open file", path, error))?;
        if read == 0 {
            break;
        }
        length = length
            .checked_add(read as u64)
            .ok_or_else(|| changed_error(path))?;
        if length > max_bytes {
            return Err(IrohaZipError::Policy(format!(
                "file grew beyond the {max_bytes} byte snapshot limit: {}",
                path.display()
            )));
        }
        hasher.update(&buffer[..read]);
    }
    Ok((length, hasher.finalize().into()))
}

fn changed_error(path: &Path) -> IrohaZipError {
    IrohaZipError::Policy(format!(
        "source identity, timestamps, length, or content changed during snapshot: {}",
        path.display()
    ))
}
