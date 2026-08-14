use std::fs;
use std::path::{Path, PathBuf};

use iroha_zip::policy::Limits;
use iroha_zip::preview::{self, ArchiveEntry, ArchiveEntryKind};
use iroha_zip::selection;
use iroha_zip::util;
#[cfg(unix)]
use iroha_zip::{
    backend::{self, BackendBundle},
    config::{Config, FilenameEncoding},
    extract::{self, ExtractRequest},
    preview::PreviewRequest,
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "iroha-zip-preview-selection-{}",
            util::unique_token()
        ));
        fs::create_dir(&path).unwrap();
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
fn preview_inventory_is_sorted_typed_and_bounded() {
    let directory = TestDirectory::new();
    let root = directory.path().join("root");
    fs::create_dir_all(root.join("資料")).unwrap();
    fs::write(root.join("z.txt"), b"z").unwrap();
    fs::write(root.join("資料").join("readme.txt"), b"hello").unwrap();

    let result = preview::inventory_tree(&root, &Limits::default()).unwrap();
    assert_eq!(
        result.entries,
        [
            ArchiveEntry {
                path: PathBuf::from("z.txt"),
                kind: ArchiveEntryKind::File,
                size: 1,
            },
            ArchiveEntry {
                path: PathBuf::from("資料"),
                kind: ArchiveEntryKind::Directory,
                size: 0,
            },
            ArchiveEntry {
                path: PathBuf::from("資料/readme.txt"),
                kind: ArchiveEntryKind::File,
                size: 5,
            },
        ]
    );
    assert_eq!(result.summary.files, 2);
    assert_eq!(result.summary.directories, 1);
    assert_eq!(result.summary.total_bytes, 6);
}

#[test]
fn selected_files_and_directories_are_reaudited_into_a_minimal_tree() {
    let directory = TestDirectory::new();
    let source = directory.path().join("source");
    let target = directory.path().join("selected");
    fs::create_dir_all(source.join("docs")).unwrap();
    fs::write(source.join("top.txt"), b"top").unwrap();
    fs::write(source.join("docs").join("a.txt"), b"alpha").unwrap();
    fs::write(source.join("docs").join("b.txt"), b"bravo").unwrap();
    fs::write(source.join("ignored.txt"), b"ignore").unwrap();

    let summary = selection::materialize_selection(
        &source,
        &target,
        &[PathBuf::from("docs"), PathBuf::from("top.txt")],
        &Limits::default(),
    )
    .unwrap();

    assert_eq!(summary.files, 3);
    assert_eq!(summary.directories, 1);
    assert_eq!(summary.total_bytes, 13);
    assert_eq!(fs::read(target.join("top.txt")).unwrap(), b"top");
    assert_eq!(
        fs::read(target.join("docs").join("a.txt")).unwrap(),
        b"alpha"
    );
    assert!(!target.join("ignored.txt").exists());
}

#[test]
fn unsafe_missing_duplicate_and_overlapping_selections_fail_without_output() {
    let directory = TestDirectory::new();
    let source = directory.path().join("source");
    fs::create_dir_all(source.join("docs")).unwrap();
    fs::write(source.join("docs").join("a.txt"), b"alpha").unwrap();
    let limits = Limits::default();

    for (name, selectors) in [
        ("unsafe", vec![PathBuf::from("../escape.txt")]),
        ("missing", vec![PathBuf::from("missing.txt")]),
        (
            "duplicate",
            vec![PathBuf::from("docs"), PathBuf::from("docs")],
        ),
        (
            "case-duplicate",
            vec![PathBuf::from("docs"), PathBuf::from("DOCS")],
        ),
        (
            "overlap",
            vec![PathBuf::from("docs"), PathBuf::from("docs/a.txt")],
        ),
        ("not-normalized", vec![PathBuf::from("docs/./a.txt")]),
    ] {
        let target = directory.path().join(name);
        assert!(
            selection::materialize_selection(&source, &target, &selectors, &limits).is_err(),
            "selection case should fail: {name}"
        );
        assert!(!target.exists());
    }
}

#[cfg(unix)]
#[test]
fn shared_staging_drives_preview_and_selected_publication_end_to_end() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TestDirectory::new();
    let backend_root = directory.path().join("backend");
    fs::create_dir(&backend_root).unwrap();
    let executable = backend_root.join("fake-bsdtar");
    fs::write(
        &executable,
        br#"#!/bin/sh
output=
list=false
while [ "$#" -gt 0 ]; do
    if [ "$1" = "-t" ]; then
        list=true
    fi
    if [ "$1" = "-C" ]; then
        shift
        output=$1
    fi
    shift
done
if [ "$list" = true ]; then
    printf 'archive/\narchive/docs/\narchive/docs/readme.txt\narchive/images/\narchive/images/photo.jpg\n'
    exit 0
fi
/bin/mkdir -p "$output/archive/docs" "$output/archive/images"
printf 'readme' > "$output/archive/docs/readme.txt"
printf 'image' > "$output/archive/images/photo.jpg"
"#,
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o500)).unwrap();
    let hash = backend::sha256_file(&executable).unwrap();
    fs::write(
        backend_root.join("backend-manifest.tsv"),
        format!(
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tfake-bsdtar\nsha256\t{hash}\tfake-bsdtar\n"
        ),
    )
    .unwrap();
    let backend = BackendBundle::verify(&backend_root).unwrap();
    let archive = directory.path().join("archive.zip");
    fs::write(&archive, b"fake archive bytes").unwrap();
    let config = Config::default();

    let preview = preview::preview(PreviewRequest {
        backend: &backend,
        config: &config,
        archive: &archive,
        encoding: FilenameEncoding::Auto,
        allow_unsandboxed: true,
    })
    .unwrap();
    assert_eq!(preview.summary.files, 2);
    assert!(
        preview
            .entries
            .iter()
            .any(|entry| entry.path == Path::new("docs/readme.txt"))
    );

    let destination = directory.path().join("published");
    let extracted = extract::extract(ExtractRequest {
        backend: &backend,
        config: &config,
        archive: &archive,
        output: Some(&destination),
        encoding: FilenameEncoding::Auto,
        selections: &[PathBuf::from("docs/readme.txt")],
        open: false,
        allow_unsandboxed: true,
    })
    .unwrap();
    assert_eq!(extracted.destination, destination);
    assert_eq!(
        fs::read(destination.join("docs").join("readme.txt")).unwrap(),
        b"readme"
    );
    assert!(!destination.join("images").exists());
}

#[cfg(unix)]
#[test]
fn unsafe_raw_member_listing_stops_before_extraction_or_publication() {
    use std::os::unix::fs::PermissionsExt;

    use iroha_zip::error::IrohaZipError;

    let directory = TestDirectory::new();
    let backend_root = directory.path().join("backend");
    fs::create_dir(&backend_root).unwrap();
    let executable = backend_root.join("fake-bsdtar");
    fs::write(
        &executable,
        br#"#!/bin/sh
for argument in "$@"; do
    if [ "$argument" = "-t" ]; then
        printf '/absolute-would-be-normalized.txt\n'
        exit 0
    fi
done
exit 42
"#,
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o500)).unwrap();
    let hash = backend::sha256_file(&executable).unwrap();
    fs::write(
        backend_root.join("backend-manifest.tsv"),
        format!(
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tfake-bsdtar\nsha256\t{hash}\tfake-bsdtar\n"
        ),
    )
    .unwrap();
    let backend = BackendBundle::verify(&backend_root).unwrap();
    let archive = directory.path().join("unsafe.zip");
    fs::write(&archive, b"fake archive bytes").unwrap();
    let destination = directory.path().join("must-not-exist");

    let result = extract::extract(ExtractRequest {
        backend: &backend,
        config: &Config::default(),
        archive: &archive,
        output: Some(&destination),
        encoding: FilenameEncoding::Auto,
        selections: &[],
        open: false,
        allow_unsandboxed: true,
    });
    let Err(error) = result else {
        panic!("unsafe raw member listing was unexpectedly published");
    };
    assert!(
        matches!(&error, IrohaZipError::Policy(message) if message.contains("relative path")),
        "unexpected preflight result: {error}"
    );
    assert!(!destination.exists());
}

#[cfg(unix)]
#[test]
fn listing_cannot_replace_the_sandbox_archive_between_passes() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TestDirectory::new();
    let backend_root = directory.path().join("backend");
    fs::create_dir(&backend_root).unwrap();
    let executable = backend_root.join("fake-bsdtar");
    fs::write(
        &executable,
        br#"#!/bin/sh
archive=
list=false
while [ "$#" -gt 0 ]; do
    if [ "$1" = "-t" ]; then list=true; fi
    if [ "$1" = "-f" ]; then shift; archive=$1; fi
    shift
done
if [ "$list" = true ]; then
    printf 'replaced archive bytes' > "$archive"
    printf 'safe.txt\n'
    exit 0
fi
exit 99
"#,
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o500)).unwrap();
    let hash = backend::sha256_file(&executable).unwrap();
    fs::write(
        backend_root.join("backend-manifest.tsv"),
        format!(
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tfake-bsdtar\nsha256\t{hash}\tfake-bsdtar\n"
        ),
    )
    .unwrap();
    let backend = BackendBundle::verify(&backend_root).unwrap();
    let archive = directory.path().join("input.zip");
    fs::write(&archive, b"original archive bytes").unwrap();
    let destination = directory.path().join("must-not-exist");

    let result = extract::extract(ExtractRequest {
        backend: &backend,
        config: &Config::default(),
        archive: &archive,
        output: Some(&destination),
        encoding: FilenameEncoding::Auto,
        selections: &[],
        open: false,
        allow_unsandboxed: true,
    });
    let Err(error) = result else {
        panic!("a replaced sandbox archive was unexpectedly published");
    };
    let error = error.to_string();

    assert!(error.contains("sandbox archive changed"), "{error}");
    assert_eq!(fs::read(archive).unwrap(), b"original archive bytes");
    assert!(!destination.exists());
}

#[cfg(unix)]
#[test]
fn listing_cannot_preseed_the_extraction_directory() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TestDirectory::new();
    let backend_root = directory.path().join("backend");
    fs::create_dir(&backend_root).unwrap();
    let executable = backend_root.join("fake-bsdtar");
    fs::write(
        &executable,
        br#"#!/bin/sh
for argument in "$@"; do
    if [ "$argument" = "-t" ]; then
        /bin/mkdir "$PWD/output"
        printf 'planted' > "$PWD/output/planted.txt"
        printf 'safe.txt\n'
        exit 0
    fi
done
exit 99
"#,
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o500)).unwrap();
    let hash = backend::sha256_file(&executable).unwrap();
    fs::write(
        backend_root.join("backend-manifest.tsv"),
        format!(
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tfake-bsdtar\nsha256\t{hash}\tfake-bsdtar\n"
        ),
    )
    .unwrap();
    let backend = BackendBundle::verify(&backend_root).unwrap();
    let archive = directory.path().join("input.zip");
    fs::write(&archive, b"original archive bytes").unwrap();
    let destination = directory.path().join("must-not-exist");

    let result = extract::extract(ExtractRequest {
        backend: &backend,
        config: &Config::default(),
        archive: &archive,
        output: Some(&destination),
        encoding: FilenameEncoding::Auto,
        selections: &[],
        open: false,
        allow_unsandboxed: true,
    });

    assert!(result.is_err());
    assert!(!destination.exists());
}
