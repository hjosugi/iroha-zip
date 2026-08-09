use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use safearc::policy::{self, Limits};
use safearc::transfer;
use safearc::util;

#[test]
fn accepts_normal_and_japanese_names() {
    policy::validate_component(OsStr::new("資料_2026.txt")).unwrap();
    policy::validate_relative_path(Path::new("資料/写真/夏.jpg"), &Limits::default()).unwrap();
}

#[test]
fn rejects_parent_absolute_and_windows_unsafe_components() {
    let limits = Limits::default();
    assert!(policy::validate_relative_path(Path::new("../escape.txt"), &limits).is_err());
    assert!(policy::validate_relative_path(Path::new("/absolute.txt"), &limits).is_err());

    for name in [
        ".",
        "..",
        "CON",
        "con.txt",
        "COM1.log",
        "LPT9",
        "COM¹.txt",
        "name:stream",
        "trailing.",
        "trailing ",
        "bad?.txt",
        "bad*.txt",
        "bad|name",
        "bad\\name",
    ] {
        assert!(
            policy::validate_component(OsStr::new(name)).is_err(),
            "component should be rejected: {name:?}"
        );
    }
}

#[test]
fn enforces_path_depth_and_utf8_length() {
    let mut limits = Limits {
        max_depth: 2,
        ..Limits::default()
    };
    assert!(policy::validate_relative_path(Path::new("a/b"), &limits).is_ok());
    assert!(policy::validate_relative_path(Path::new("a/b/c"), &limits).is_err());

    limits.max_depth = 64;
    limits.max_path_bytes = 8;
    assert!(policy::validate_relative_path(Path::new("abcd"), &limits).is_ok());
    assert!(policy::validate_relative_path(Path::new("長い名前.txt"), &limits).is_err());
}

#[test]
fn audited_copy_preserves_only_regular_tree_content() {
    let parent = std::env::temp_dir().join(format!("safearc-test-{}", util::unique_token()));
    let source = parent.join("source");
    let target = parent.join("target");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("hello.txt"), b"hello").unwrap();
    fs::write(source.join("nested").join("world.txt"), b"world").unwrap();

    let copied = transfer::copy_audited_tree(&source, &target, &Limits::default()).unwrap();
    assert_eq!(copied.files, 2);
    assert_eq!(copied.directories, 1);
    assert_eq!(copied.total_bytes, 10);
    assert_eq!(fs::read(target.join("hello.txt")).unwrap(), b"hello");
    assert_eq!(
        fs::read(target.join("nested").join("world.txt")).unwrap(),
        b"world"
    );

    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn audit_rejects_hard_links() {
    let parent = std::env::temp_dir().join(format!("safearc-test-{}", util::unique_token()));
    fs::create_dir_all(&parent).unwrap();
    let first = parent.join("first.txt");
    let second = parent.join("second.txt");
    fs::write(&first, b"same inode").unwrap();
    fs::hard_link(&first, &second).unwrap();

    assert!(policy::audit_tree(&parent, &Limits::default()).is_err());
    fs::remove_dir_all(parent).unwrap();
}
