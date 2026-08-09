use std::fs;
use std::path::{Path, PathBuf};

use iroha_zip::policy::Limits;
use iroha_zip::snapshot::AuditedFile;
use iroha_zip::transfer;
use iroha_zip::util;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("iroha-zip-snapshot-{}", util::unique_token()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn audited_file_copy_preserves_content_and_fingerprint() {
    let directory = TestDirectory::new();
    let source = directory.path().join("source.txt");
    let target = directory.path().join("target.txt");
    fs::write(&source, b"snapshot content").unwrap();

    let mut snapshot = AuditedFile::open(&source, 1024).unwrap();
    assert_eq!(snapshot.fingerprint().length(), 16);
    assert!(snapshot.fingerprint().identity().is_some());
    assert_eq!(snapshot.copy_to_new(&target).unwrap(), 16);
    assert_eq!(fs::read(&target).unwrap(), b"snapshot content");
}

#[cfg(not(windows))]
#[test]
fn same_size_source_mutation_is_rejected() {
    let directory = TestDirectory::new();
    let source = directory.path().join("source.txt");
    let target = directory.path().join("target.txt");
    fs::write(&source, b"alpha").unwrap();

    let mut snapshot = AuditedFile::open(&source, 1024).unwrap();
    fs::write(&source, b"bravo").unwrap();

    assert!(snapshot.copy_to_new(&target).is_err());
    assert!(!target.exists());
}

#[cfg(not(windows))]
#[test]
fn same_size_path_replacement_is_rejected() {
    let directory = TestDirectory::new();
    let source = directory.path().join("source.txt");
    let moved = directory.path().join("moved.txt");
    let target = directory.path().join("target.txt");
    fs::write(&source, b"alpha").unwrap();

    let mut snapshot = AuditedFile::open(&source, 1024).unwrap();
    fs::rename(&source, &moved).unwrap();
    fs::write(&source, b"bravo").unwrap();

    assert!(snapshot.copy_to_new(&target).is_err());
    assert!(!target.exists());
}

#[cfg(windows)]
#[test]
fn windows_snapshot_handle_blocks_source_replacement() {
    let directory = TestDirectory::new();
    let source = directory.path().join("source.txt");
    let moved = directory.path().join("moved.txt");
    let target = directory.path().join("target.txt");
    fs::write(&source, b"alpha").unwrap();

    let mut snapshot = AuditedFile::open(&source, 1024).unwrap();
    assert!(fs::write(&source, b"bravo").is_err());
    assert!(fs::rename(&source, &moved).is_err());
    snapshot.copy_to_new(&target).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"alpha");
}

#[test]
fn hard_linked_snapshot_source_is_rejected() {
    let directory = TestDirectory::new();
    let source = directory.path().join("source.txt");
    let alias = directory.path().join("alias.txt");
    fs::write(&source, b"shared").unwrap();
    fs::hard_link(&source, &alias).unwrap();

    assert!(AuditedFile::open(&source, 1024).is_err());
}

#[cfg(unix)]
#[test]
fn symbolic_link_snapshot_source_is_rejected() {
    use std::os::unix::fs::symlink;

    let directory = TestDirectory::new();
    let source = directory.path().join("source.txt");
    let alias = directory.path().join("alias.txt");
    fs::write(&source, b"content").unwrap();
    symlink(&source, &alias).unwrap();

    assert!(AuditedFile::open(&alias, 1024).is_err());
}

#[test]
fn tree_fingerprint_detects_same_size_content_and_path_changes() {
    let directory = TestDirectory::new();
    let root = directory.path().join("tree");
    fs::create_dir(&root).unwrap();
    let first = root.join("first.txt");
    let second = root.join("second.txt");
    fs::write(&first, b"alpha").unwrap();

    let original = transfer::fingerprint_tree(&root, &Limits::default()).unwrap();
    fs::write(&first, b"bravo").unwrap();
    let changed_content = transfer::fingerprint_tree(&root, &Limits::default()).unwrap();
    assert_ne!(changed_content, original);
    assert_eq!(changed_content.summary(), original.summary());

    fs::rename(&first, &second).unwrap();
    let renamed = transfer::fingerprint_tree(&root, &Limits::default()).unwrap();
    assert_ne!(renamed, changed_content);
    assert_eq!(renamed.summary(), changed_content.summary());
}
