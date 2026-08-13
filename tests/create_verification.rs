#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use iroha_zip::backend::{self, BackendBundle};
use iroha_zip::cli::CreateFormat;
use iroha_zip::config::Config;
use iroha_zip::create;
use iroha_zip::util;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "iroha-zip-create-verification-{}",
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

fn source_tree(root: &Path) -> PathBuf {
    let source = root.join("source");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("alpha.txt"), b"alpha").unwrap();
    fs::write(source.join("nested").join("beta.txt"), b"beta").unwrap();
    source
}

fn fake_backend(root: &Path, behavior: &str) -> BackendBundle {
    let backend_root = root.join(format!("backend-{behavior}"));
    fs::create_dir(&backend_root).unwrap();
    let executable = backend_root.join("fake-bsdtar");
    let script = format!(
        r#"#!/bin/sh
set -eu
mode=
archive=
directory=
source_archive=
while [ "$#" -gt 0 ]; do
    case "$1" in
        -c) mode=create ;;
        -t) mode=list ;;
        -x) mode=extract ;;
        -f)
            shift
            archive=$1
            ;;
        -C)
            shift
            directory=$1
            ;;
        @*)
            source_archive=${{1#@}}
            case "$source_archive" in
                */*) directory=${{source_archive%/*}}/source ;;
                *) directory=$PWD/source ;;
            esac
            ;;
    esac
    shift
done

case "$mode" in
    create)
        if [ "{behavior}" = "mutate-source" ]; then
            printf 'bravo' > "$directory/alpha.txt"
        fi
        if [ "{behavior}" = "mutate-stream" ]; then
            printf 'corrupt' > "$source_archive"
        fi
        printf 'archive-{behavior}' > "$archive"
        ;;
    list)
        if [ "{behavior}" = "empty" ]; then
            printf './\n'
        elif [ "{behavior}" = "single-archive-root" ]; then
            printf './\narchive/\narchive/item.txt\n'
        elif [ "{behavior}" = "unsafe-listing" ]; then
            printf '../escape.txt\n'
        else
            printf './\nalpha.txt\nnested/\nnested/beta.txt\n'
        fi
        ;;
    extract)
        if [ "{behavior}" = "empty" ]; then
            :
        elif [ "{behavior}" = "single-archive-root" ]; then
            /bin/mkdir -p "$directory/archive"
            printf 'item' > "$directory/archive/item.txt"
        else
            /bin/mkdir -p "$directory/nested"
            printf 'alpha' > "$directory/alpha.txt"
            if [ "{behavior}" = "mismatched-content" ]; then
                printf 'evil' > "$directory/nested/beta.txt"
            else
                printf 'beta' > "$directory/nested/beta.txt"
            fi
        fi
        ;;
    *)
        printf 'unsupported fake backend invocation\n' >&2
        exit 64
        ;;
esac
"#
    );
    fs::write(&executable, script).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o500)).unwrap();
    let hash = backend::sha256_file(&executable).unwrap();
    fs::write(
        backend_root.join("backend-manifest.tsv"),
        format!(
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tfake-bsdtar\nsha256\t{hash}\tfake-bsdtar\n"
        ),
    )
    .unwrap();
    BackendBundle::verify(&backend_root).unwrap()
}

#[test]
fn created_archive_is_reextracted_and_matched_before_publication() {
    let directory = TestDirectory::new();
    let source = source_tree(directory.path());
    let backend = fake_backend(directory.path(), "match");
    let output = directory.path().join("verified.zip");

    let created = create::create_archive(
        &backend,
        &Config::default(),
        CreateFormat::Zip,
        &source,
        &output,
        true,
    )
    .unwrap();

    assert_eq!(created, output);
    assert_eq!(fs::read(&created).unwrap(), b"archive-match");
    assert_eq!(fs::read(source.join("alpha.txt")).unwrap(), b"alpha");
}

#[test]
fn pax_container_overhead_does_not_consume_the_source_single_file_limit() {
    let directory = TestDirectory::new();
    let source = source_tree(directory.path());
    let backend = fake_backend(directory.path(), "match");
    let output = directory.path().join("verified.zip");
    let mut config = Config::default();
    config.limits.max_single_file_bytes = 5;
    config.limits.max_total_bytes = 9;

    create::create_archive(&backend, &config, CreateFormat::Zip, &source, &output, true).unwrap();

    assert!(output.exists());
}

#[test]
fn empty_source_root_marker_is_verified_without_weakening_external_listing_policy() {
    let directory = TestDirectory::new();
    let source = directory.path().join("empty-source");
    fs::create_dir(&source).unwrap();
    let backend = fake_backend(directory.path(), "empty");
    let output = directory.path().join("empty.zip");

    create::create_archive(
        &backend,
        &Config::default(),
        CreateFormat::Zip,
        &source,
        &output,
        true,
    )
    .unwrap();

    assert_eq!(fs::read(output).unwrap(), b"archive-empty");
}

#[test]
fn verification_compares_the_full_root_without_destination_name_stripping() {
    let directory = TestDirectory::new();
    let source = directory.path().join("source");
    fs::create_dir_all(source.join("archive")).unwrap();
    fs::write(source.join("archive").join("item.txt"), b"item").unwrap();
    let backend = fake_backend(directory.path(), "single-archive-root");
    let output = directory.path().join("verified.zip");

    create::create_archive(
        &backend,
        &Config::default(),
        CreateFormat::Zip,
        &source,
        &output,
        true,
    )
    .unwrap();

    assert_eq!(fs::read(output).unwrap(), b"archive-single-archive-root");
}

#[test]
fn reextraction_content_mismatch_cannot_publish() {
    let directory = TestDirectory::new();
    let source = source_tree(directory.path());
    let backend = fake_backend(directory.path(), "mismatched-content");
    let output = directory.path().join("rejected.zip");

    let error = create::create_archive(
        &backend,
        &Config::default(),
        CreateFormat::Zip,
        &source,
        &output,
        true,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("does not reproduce the audited source tree"));
    assert!(!output.exists());
}

#[test]
fn backend_source_mutation_cannot_publish() {
    let directory = TestDirectory::new();
    let source = source_tree(directory.path());
    let backend = fake_backend(directory.path(), "mutate-source");
    let output = directory.path().join("rejected.zip");

    let error = create::create_archive(
        &backend,
        &Config::default(),
        CreateFormat::Zip,
        &source,
        &output,
        true,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("modified the staged source tree"));
    assert!(!output.exists());
    assert_eq!(fs::read(source.join("alpha.txt")).unwrap(), b"alpha");
}

#[test]
fn backend_pax_stream_mutation_cannot_publish() {
    let directory = TestDirectory::new();
    let source = source_tree(directory.path());
    let backend = fake_backend(directory.path(), "mutate-stream");
    let output = directory.path().join("rejected.zip");

    let error = create::create_archive(
        &backend,
        &Config::default(),
        CreateFormat::Zip,
        &source,
        &output,
        true,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("modified the bounded PAX source stream"));
    assert!(!output.exists());
}

#[test]
#[ignore = "requires /usr/bin/bsdtar from a system libarchive installation"]
fn real_libarchive_accepts_the_bounded_pax_archive_for_all_create_formats() {
    let directory = TestDirectory::new();
    let source = source_tree(directory.path());
    fs::create_dir(source.join("empty")).unwrap();
    fs::write(source.join("日本語.txt"), "いろは".as_bytes()).unwrap();

    let backend_root = directory.path().join("real-backend");
    fs::create_dir(&backend_root).unwrap();
    let executable = backend_root.join("bsdtar");
    fs::copy("/usr/bin/bsdtar", &executable).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o500)).unwrap();
    let hash = backend::sha256_file(&executable).unwrap();
    fs::write(
        backend_root.join("backend-manifest.tsv"),
        format!("IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tbsdtar\nsha256\t{hash}\tbsdtar\n"),
    )
    .unwrap();
    let backend = BackendBundle::verify(&backend_root).unwrap();

    for format in [
        CreateFormat::Zip,
        CreateFormat::SevenZip,
        CreateFormat::Tar,
        CreateFormat::TarGz,
    ] {
        let output = directory
            .path()
            .join(format!("verified.{}", format.expected_extension()));
        create::create_archive(&backend, &Config::default(), format, &source, &output, true)
            .unwrap();
        assert!(fs::metadata(output).unwrap().len() > 0);
    }

    let empty_source = directory.path().join("empty-source");
    fs::create_dir(&empty_source).unwrap();
    let empty_output = directory.path().join("empty.zip");
    create::create_archive(
        &backend,
        &Config::default(),
        CreateFormat::Zip,
        &empty_source,
        &empty_output,
        true,
    )
    .unwrap();
    assert!(fs::metadata(empty_output).unwrap().len() > 0);
}

#[test]
fn unsafe_created_member_listing_cannot_reach_extraction_or_publication() {
    let directory = TestDirectory::new();
    let source = source_tree(directory.path());
    let backend = fake_backend(directory.path(), "unsafe-listing");
    let output = directory.path().join("rejected.zip");

    let error = create::create_archive(
        &backend,
        &Config::default(),
        CreateFormat::Zip,
        &source,
        &output,
        true,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("invalid filename component"), "{error}");
    assert!(!output.exists());
    assert!(!directory.path().join("escape.txt").exists());
}
