use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use crate::error::{IrohaZipError, Result};
use crate::platform;
use crate::policy::{self, Limits};
use crate::snapshot::{AuditedFile, FileFingerprint};

const BLOCK_BYTES: usize = 512;
const USTAR_SIZE_MAX: u64 = 0o77_777_777_777;

#[derive(Clone, Copy)]
enum EntryKind {
    Directory,
    File,
}

pub fn write_tree_archive(
    source_root: &Path,
    output: &Path,
    limits: &Limits,
) -> Result<FileFingerprint> {
    let entries = enumerate_tree(source_root, limits)?;
    let source_root = fs::canonicalize(source_root).map_err(|error| {
        IrohaZipError::io_path("cannot resolve PAX source root", source_root, error)
    })?;
    let result = write_tree_archive_inner(&source_root, output, limits, &entries);
    if result.is_err() {
        let _ = fs::remove_file(output);
    }
    result
}

fn write_tree_archive_inner(
    source_root: &Path,
    output: &Path,
    limits: &Limits,
    entries: &BTreeMap<PathBuf, EntryKind>,
) -> Result<FileFingerprint> {
    let file = platform::create_snapshot_target(output)?;
    let mut archive = ArchiveWriter::new(file);
    for (relative, kind) in entries {
        let path = source_root.join(relative);
        match kind {
            EntryKind::Directory => {
                let metadata = fs::symlink_metadata(&path).map_err(|error| {
                    IrohaZipError::io_path("cannot inspect PAX directory", &path, error)
                })?;
                platform::validate_extracted_entry_security(&path, &metadata)?;
                if !metadata.is_dir() {
                    return Err(IrohaZipError::Policy(format!(
                        "PAX source directory changed type: {}",
                        path.display()
                    )));
                }
                archive.write_entry(&path, relative, *kind, 0, limits)?;
            }
            EntryKind::File => {
                let mut snapshot = AuditedFile::open(&path, limits.max_single_file_bytes)?;
                archive.write_entry(
                    &path,
                    relative,
                    *kind,
                    snapshot.fingerprint().length(),
                    limits,
                )?;
                let copied = snapshot.copy_to_writer(&mut archive)?;
                if copied != snapshot.fingerprint().length() {
                    return Err(IrohaZipError::Policy(format!(
                        "PAX source length changed: {}",
                        path.display()
                    )));
                }
                archive.pad_to_block()?;
            }
        }
    }
    archive.finish()?;

    let length = fs::metadata(output)
        .map_err(|error| IrohaZipError::io_path("cannot inspect PAX archive", output, error))?
        .len();
    let snapshot = AuditedFile::open(output, length)?;
    Ok(snapshot.fingerprint().clone())
}

fn enumerate_tree(source_root: &Path, limits: &Limits) -> Result<BTreeMap<PathBuf, EntryKind>> {
    platform::validate_directory_security(source_root)?;
    let source_root = fs::canonicalize(source_root).map_err(|error| {
        IrohaZipError::io_path("cannot resolve PAX source tree", source_root, error)
    })?;
    platform::validate_directory_security(&source_root)?;

    let max_entries = limits
        .max_files
        .checked_add(limits.max_directories)
        .ok_or_else(|| IrohaZipError::Config("PAX entry limit overflow".to_owned()))?;
    let mut entries = BTreeMap::new();
    let mut files = 0u64;
    let mut directories = 0u64;
    let mut total_bytes = 0u64;
    let mut stack = vec![platform::DirectorySnapshot::open(&source_root)?];
    while let Some(directory) = stack.pop() {
        if !directory.path().starts_with(&source_root) {
            return Err(IrohaZipError::Policy(format!(
                "PAX source directory escaped its root: {}",
                directory.path().display()
            )));
        }
        for name in directory.entries(max_entries)? {
            policy::validate_component(&name)?;
            let path = directory.path().join(name);
            let relative = path.strip_prefix(&source_root).map_err(|_| {
                IrohaZipError::Policy(format!(
                    "PAX source entry escaped its root: {}",
                    path.display()
                ))
            })?;
            policy::validate_relative_path(relative, limits)?;
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                IrohaZipError::io_path("cannot inspect PAX source entry", &path, error)
            })?;
            platform::validate_extracted_entry_security(&path, &metadata)?;
            let kind = if metadata.is_dir() {
                directories = checked_increment(directories, "PAX directory count")?;
                if directories > limits.max_directories {
                    return Err(IrohaZipError::Policy(format!(
                        "directory count exceeds {}",
                        limits.max_directories
                    )));
                }
                stack.push(platform::DirectorySnapshot::open(&path)?);
                EntryKind::Directory
            } else if metadata.is_file() {
                files = checked_increment(files, "PAX file count")?;
                if files > limits.max_files {
                    return Err(IrohaZipError::Policy(format!(
                        "file count exceeds {}",
                        limits.max_files
                    )));
                }
                let size = metadata.len();
                if size > limits.max_single_file_bytes {
                    return Err(IrohaZipError::Policy(format!(
                        "file exceeds {} bytes: {}",
                        limits.max_single_file_bytes,
                        path.display()
                    )));
                }
                total_bytes = total_bytes
                    .checked_add(size)
                    .ok_or_else(|| IrohaZipError::Policy("PAX byte count overflow".to_owned()))?;
                if total_bytes > limits.max_total_bytes {
                    return Err(IrohaZipError::Policy(format!(
                        "total file size exceeds {} bytes",
                        limits.max_total_bytes
                    )));
                }
                EntryKind::File
            } else {
                return Err(IrohaZipError::Policy(format!(
                    "special PAX source entry is rejected: {}",
                    path.display()
                )));
            };
            if entries.insert(relative.to_path_buf(), kind).is_some() {
                return Err(IrohaZipError::Policy(format!(
                    "duplicate PAX source entry: {}",
                    relative.display()
                )));
            }
        }
    }
    Ok(entries)
}

struct ArchiveWriter {
    file: File,
    written: u64,
    entry_index: u64,
}

impl ArchiveWriter {
    fn new(file: File) -> Self {
        Self {
            file,
            written: 0,
            entry_index: 0,
        }
    }

    fn write_entry(
        &mut self,
        path: &Path,
        relative: &Path,
        kind: EntryKind,
        size: u64,
        limits: &Limits,
    ) -> Result<()> {
        let metadata = fs::symlink_metadata(path).map_err(|error| {
            IrohaZipError::io_path("cannot recheck PAX source entry", path, error)
        })?;
        platform::validate_extracted_entry_security(path, &metadata)?;
        let archive_path = archive_path(relative, kind, limits)?;
        let mut attributes = pax_record("path", &archive_path)?;
        if matches!(kind, EntryKind::File) {
            attributes.extend_from_slice(&pax_record("size", &size.to_string())?);
        }
        attributes.extend_from_slice(&pax_record("mtime", "0")?);

        let pax_name = format!("PaxHeaders/{}", self.entry_index);
        let pax_size = u64::try_from(attributes.len())
            .map_err(|_| IrohaZipError::Policy("PAX attributes are too large".to_owned()))?;
        self.write_header(&pax_name, 0o644, pax_size, b'x')?;
        self.write_all(&attributes)
            .map_err(|error| IrohaZipError::io("cannot write PAX attributes", error))?;
        self.pad_to_block()?;

        let stored_size = if size <= USTAR_SIZE_MAX { size } else { 0 };
        let header_name = format!("IrohaZipEntry/{}", self.entry_index);
        let (mode, type_flag) = match kind {
            EntryKind::Directory => (0o755, b'5'),
            EntryKind::File => (0o644, b'0'),
        };
        self.write_header(&header_name, mode, stored_size, type_flag)?;
        self.entry_index = self
            .entry_index
            .checked_add(1)
            .ok_or_else(|| IrohaZipError::Policy("PAX entry index overflow".to_owned()))?;
        Ok(())
    }

    fn write_header(&mut self, name: &str, mode: u64, size: u64, type_flag: u8) -> Result<()> {
        let mut header = [0u8; BLOCK_BYTES];
        write_bytes(&mut header[..100], name.as_bytes(), "PAX header name")?;
        write_octal(&mut header[100..108], mode, "PAX mode")?;
        write_octal(&mut header[108..116], 0, "PAX uid")?;
        write_octal(&mut header[116..124], 0, "PAX gid")?;
        write_octal(&mut header[124..136], size, "PAX size")?;
        write_octal(&mut header[136..148], 0, "PAX mtime")?;
        header[148..156].fill(b' ');
        header[156] = type_flag;
        write_bytes(&mut header[257..263], b"ustar\0", "PAX magic")?;
        write_bytes(&mut header[263..265], b"00", "PAX version")?;
        write_bytes(&mut header[265..297], b"iroha-zip", "PAX user")?;
        write_bytes(&mut header[297..329], b"iroha-zip", "PAX group")?;
        let checksum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
        let digits = format!("{checksum:06o}");
        if digits.len() != 6 {
            return Err(IrohaZipError::Policy(
                "PAX header checksum overflow".to_owned(),
            ));
        }
        header[148..154].copy_from_slice(digits.as_bytes());
        header[154] = 0;
        header[155] = b' ';
        self.write_all(&header)
            .map_err(|error| IrohaZipError::io("cannot write PAX header", error))
    }

    fn pad_to_block(&mut self) -> Result<()> {
        let remainder = self.written % BLOCK_BYTES as u64;
        if remainder == 0 {
            return Ok(());
        }
        let padding = usize::try_from(BLOCK_BYTES as u64 - remainder)
            .map_err(|_| IrohaZipError::Policy("PAX padding overflow".to_owned()))?;
        self.write_all(&[0u8; BLOCK_BYTES][..padding])
            .map_err(|error| IrohaZipError::io("cannot write PAX padding", error))
    }

    fn finish(mut self) -> Result<()> {
        self.write_all(&[0u8; 2 * BLOCK_BYTES])
            .map_err(|error| IrohaZipError::io("cannot finish PAX archive", error))?;
        self.file
            .flush()
            .and_then(|()| self.file.sync_all())
            .map_err(|error| IrohaZipError::io("cannot flush PAX archive", error))
    }
}

impl Write for ArchiveWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.file.write(buffer)?;
        self.written = self
            .written
            .checked_add(written as u64)
            .ok_or_else(|| io::Error::other("PAX archive length overflow"))?;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

fn archive_path(relative: &Path, kind: EntryKind, limits: &Limits) -> Result<String> {
    let mut value = String::from("./");
    policy::validate_relative_path(relative, limits)?;
    let mut first = true;
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(IrohaZipError::Policy(format!(
                "invalid PAX path component: {}",
                relative.display()
            )));
        };
        if !first {
            value.push('/');
        }
        let text = name.to_str().ok_or_else(|| {
            IrohaZipError::Policy(format!(
                "non-Unicode PAX path is rejected: {}",
                relative.display()
            ))
        })?;
        value.push_str(text);
        first = false;
    }
    if matches!(kind, EntryKind::Directory) && !value.ends_with('/') {
        value.push('/');
    }
    Ok(value)
}

fn pax_record(key: &str, value: &str) -> Result<Vec<u8>> {
    let body = format!("{key}={value}\n");
    let mut length = body
        .len()
        .checked_add(2)
        .ok_or_else(|| IrohaZipError::Policy("PAX record length overflow".to_owned()))?;
    loop {
        let digits = length.to_string().len();
        let adjusted = body
            .len()
            .checked_add(digits + 1)
            .ok_or_else(|| IrohaZipError::Policy("PAX record length overflow".to_owned()))?;
        if adjusted == length {
            break;
        }
        length = adjusted;
    }
    let record = format!("{length} {body}").into_bytes();
    if record.len() != length {
        return Err(IrohaZipError::Policy(
            "PAX record length is inconsistent".to_owned(),
        ));
    }
    Ok(record)
}

fn write_bytes(field: &mut [u8], value: &[u8], label: &str) -> Result<()> {
    if value.len() > field.len() {
        return Err(IrohaZipError::Policy(format!("{label} is too long")));
    }
    field[..value.len()].copy_from_slice(value);
    Ok(())
}

fn write_octal(field: &mut [u8], value: u64, label: &str) -> Result<()> {
    let width = field
        .len()
        .checked_sub(1)
        .ok_or_else(|| IrohaZipError::Policy(format!("{label} has no terminator space")))?;
    let digits = format!("{value:0width$o}");
    if digits.len() != width {
        return Err(IrohaZipError::Policy(format!("{label} overflow")));
    }
    field[..width].copy_from_slice(digits.as_bytes());
    field[width] = 0;
    Ok(())
}

fn checked_increment(value: u64, label: &str) -> Result<u64> {
    value
        .checked_add(1)
        .ok_or_else(|| IrohaZipError::Policy(format!("{label} overflow")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pax_record_declares_its_exact_byte_length() {
        for value in ["a", "日本語.txt", &"x".repeat(128)] {
            let record = pax_record("path", value).unwrap();
            let declared = std::str::from_utf8(&record)
                .unwrap()
                .split_once(' ')
                .unwrap()
                .0
                .parse::<usize>()
                .unwrap();
            assert_eq!(declared, record.len());
            assert!(record.ends_with(b"\n"));
        }
    }

    #[test]
    fn archive_paths_use_portable_separators_and_directory_markers() {
        let limits = Limits::default();
        assert_eq!(
            archive_path(Path::new("nested/file.txt"), EntryKind::File, &limits).unwrap(),
            "./nested/file.txt"
        );
        assert_eq!(
            archive_path(Path::new("nested"), EntryKind::Directory, &limits).unwrap(),
            "./nested/"
        );
    }
}
