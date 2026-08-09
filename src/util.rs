use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Result, SafeArcError};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn unique_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}-{:x}", std::process::id(), nanos, counter)
}

pub fn create_unique_dir(parent: &Path, prefix: &str) -> Result<PathBuf> {
    fs::create_dir_all(parent)
        .map_err(|error| SafeArcError::io_path("cannot create parent directory", parent, error))?;
    for _ in 0..128 {
        let path = parent.join(format!("{prefix}{}", unique_token()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(SafeArcError::io_path(
                    "cannot create unique directory",
                    &path,
                    error,
                ));
            }
        }
    }
    Err(SafeArcError::Io {
        context: format!(
            "cannot allocate a unique directory under {}",
            parent.display()
        ),
        source: std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "too many unique-name collisions",
        ),
    })
}

pub fn smart_destination(archive: &Path) -> Result<PathBuf> {
    let parent = archive.parent().ok_or_else(|| {
        SafeArcError::Usage(format!("archive has no parent: {}", archive.display()))
    })?;
    let filename = archive
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| SafeArcError::Usage("archive filename must be Unicode".to_owned()))?;
    let base = archive_base_name(filename);

    for index in 0..10_000u32 {
        let candidate = if index == 0 {
            parent.join(&base)
        } else {
            parent.join(format!("{base} ({index})"))
        };
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(SafeArcError::Usage(format!(
        "cannot find a free destination name beside {}",
        archive.display()
    )))
}

pub fn archive_base_name(filename: &str) -> String {
    const COMPOUND_EXTENSIONS: &[&str] = &[
        ".tar.gz",
        ".tar.bz2",
        ".tar.xz",
        ".tar.zst",
        ".tar.lz",
        ".tar.lzma",
        ".tar.Z",
        ".tbz2",
        ".tbz",
        ".tgz",
        ".txz",
        ".tzst",
    ];
    for extension in COMPOUND_EXTENSIONS {
        if filename.len() > extension.len()
            && filename
                .get(filename.len() - extension.len()..)
                .is_some_and(|tail| tail.eq_ignore_ascii_case(extension))
        {
            return filename[..filename.len() - extension.len()].to_owned();
        }
    }
    Path::new(filename)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("archive")
        .to_owned()
}

pub fn copy_file_new_limited(source: &Path, target: &Path, max_bytes: u64) -> Result<u64> {
    let mut input = File::open(source)
        .map_err(|error| SafeArcError::io_path("cannot open source file", source, error))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|error| SafeArcError::io_path("cannot create target file", target, error))?;

    let read_limit = max_bytes.saturating_add(1);
    let copied_result = std::io::copy(
        &mut std::io::Read::by_ref(&mut input).take(read_limit),
        &mut output,
    );
    let copied = match copied_result {
        Ok(copied) => copied,
        Err(error) => {
            drop(output);
            let _ = fs::remove_file(target);
            return Err(SafeArcError::io_path("cannot copy file", target, error));
        }
    };
    if copied > max_bytes {
        drop(output);
        let _ = fs::remove_file(target);
        return Err(SafeArcError::Policy(format!(
            "file grew beyond the {} byte copy limit: {}",
            max_bytes,
            source.display()
        )));
    }
    if let Err(error) = output.sync_all() {
        drop(output);
        let _ = fs::remove_file(target);
        return Err(SafeArcError::io_path("cannot flush file", target, error));
    }
    Ok(copied)
}

pub fn copy_file_new_exact(source: &Path, target: &Path, expected_bytes: u64) -> Result<()> {
    let copied = copy_file_new_limited(source, target, expected_bytes)?;
    if copied != expected_bytes {
        let _ = fs::remove_file(target);
        return Err(SafeArcError::Policy(format!(
            "file size changed while being copied: {}",
            source.display()
        )));
    }
    Ok(())
}

pub fn read_limited(path: &Path, max_bytes: u64) -> Result<String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
        Err(error) => {
            return Err(SafeArcError::io_path(
                "cannot read process log",
                path,
                error,
            ));
        }
    };
    let mut bytes = Vec::new();
    file.take(max_bytes)
        .read_to_end(&mut bytes)
        .map_err(|error| SafeArcError::io_path("cannot read process log", path, error))?;
    Ok(String::from_utf8_lossy(&bytes).trim().to_owned())
}

pub fn write_all_new(path: &Path, data: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| SafeArcError::io_path("cannot create file", path, error))?;
    file.write_all(data)
        .map_err(|error| SafeArcError::io_path("cannot write file", path, error))?;
    file.sync_all()
        .map_err(|error| SafeArcError::io_path("cannot flush file", path, error))?;
    Ok(())
}
