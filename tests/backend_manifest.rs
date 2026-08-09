use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use iroha_zip::backend::{
    BackendBundle, BackendManifest, MAX_BACKEND_MANIFEST_BYTES, MAX_BACKEND_MANIFEST_FILES,
    sha256_file,
};
use iroha_zip::util;

const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const UPPER_HASH: &str = "ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD";

fn parse_error(input: &[u8]) -> String {
    BackendManifest::parse(input).unwrap_err().to_string()
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "iroha-zip-backend-manifest-test-{}",
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

fn write_bundle(directory: &Path, declared_hash: Option<&str>) {
    let executable = directory.join("bsdtar.exe");
    fs::write(&executable, b"test backend").unwrap();
    let actual_hash = sha256_file(&executable).unwrap();
    let hash = declared_hash.unwrap_or(&actual_hash);
    let manifest = format!(
        "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tbsdtar.exe\nsha256\t{hash}\tbsdtar.exe\n"
    );
    fs::write(directory.join("backend-manifest.tsv"), manifest).unwrap();
}

#[test]
fn parses_crlf_comments_and_normalizes_hash_case() {
    let input = format!(
        "IROHA-ZIP-BACKEND-MANIFEST\t1\r\n# generated\r\nexecutable\tbin/bsdtar.exe\r\nsha256\t{UPPER_HASH}\tbin/bsdtar.exe\r\nsha256\t{ZERO_HASH}\tlib/archive.dll\r\n"
    );

    let manifest = BackendManifest::parse(input.as_bytes()).unwrap();

    assert_eq!(manifest.executable(), Path::new("bin/bsdtar.exe"));
    assert_eq!(manifest.file_count(), 2);
    assert_eq!(
        manifest.file_hash(Path::new("bin/bsdtar.exe")),
        Some(UPPER_HASH.to_ascii_lowercase().as_str())
    );
}

#[test]
fn rejects_missing_or_ambiguous_required_entries() {
    let cases = [
        ("", "manifest is empty"),
        ("wrong\t1\n", "unsupported backend manifest header"),
        (
            &format!("IROHA-ZIP-BACKEND-MANIFEST\t1\nsha256\t{ZERO_HASH}\tbsdtar.exe\n"),
            "has no executable entry",
        ),
        (
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tbsdtar.exe\n",
            "has no hashed files",
        ),
        (
            &format!(
                "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tbsdtar.exe\nsha256\t{ZERO_HASH}\tarchive.dll\n"
            ),
            "executable is not listed",
        ),
        (
            &format!(
                "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tbsdtar.exe\nexecutable\tother.exe\nsha256\t{ZERO_HASH}\tbsdtar.exe\n"
            ),
            "multiple executable entries",
        ),
    ];

    for (input, expected) in cases {
        let error = parse_error(input.as_bytes());
        assert!(
            error.contains(expected),
            "expected {expected:?} in {error:?}"
        );
    }
}

#[test]
fn rejects_invalid_hashes_duplicate_paths_and_unknown_records() {
    let cases = [
        "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tbsdtar.exe\nsha256\tshort\tbsdtar.exe\n"
            .to_owned(),
        format!(
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tbsdtar.exe\nsha256\t{}z\tbsdtar.exe\n",
            &ZERO_HASH[..63]
        ),
        format!(
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tbsdtar.exe\nsha256\t{ZERO_HASH}\tbsdtar.exe\nsha256\t{ZERO_HASH}\tbsdtar.exe\n"
        ),
        format!(
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tbsdtar.exe\nsize\t1\tbsdtar.exe\nsha256\t{ZERO_HASH}\tbsdtar.exe\n"
        ),
    ];

    for input in &cases[..2] {
        assert!(parse_error(input.as_bytes()).contains("invalid SHA-256"));
    }
    assert!(parse_error(cases[2].as_bytes()).contains("duplicate manifest path"));
    assert!(parse_error(cases[3].as_bytes()).contains("invalid backend manifest line"));
}

#[test]
fn rejects_non_normalized_or_windows_unsafe_paths() {
    let unsafe_paths = [
        "",
        "/bsdtar.exe",
        "C:/bsdtar.exe",
        "../bsdtar.exe",
        "bin/./bsdtar.exe",
        "bin//bsdtar.exe",
        "bin\\bsdtar.exe",
        "bin/trailing.",
        "bin/trailing ",
        "bin/CON.dll",
        "bin/name?.dll",
    ];

    for path in unsafe_paths {
        let input = format!(
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\t{path}\nsha256\t{ZERO_HASH}\t{path}\n"
        );
        assert!(
            parse_error(input.as_bytes()).contains("manifest path"),
            "unsafe path was not identified: {path:?}"
        );
    }
}

#[test]
fn rejects_invalid_utf8_and_bounded_resource_overflows() {
    assert!(parse_error(&[0xff]).contains("not valid UTF-8"));

    let oversized = vec![b'x'; MAX_BACKEND_MANIFEST_BYTES + 1];
    assert!(parse_error(&oversized).contains("byte limit"));

    for path in ["a/".repeat(64) + "file.dll", "a".repeat(4097)] {
        let input = format!(
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\t{path}\nsha256\t{ZERO_HASH}\t{path}\n"
        );
        assert!(parse_error(input.as_bytes()).contains("manifest path"));
    }

    let mut too_many_files =
        String::from("IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tfile-0.exe\n");
    for index in 0..=MAX_BACKEND_MANIFEST_FILES {
        writeln!(too_many_files, "sha256\t{ZERO_HASH}\tfile-{index}.exe").unwrap();
    }
    assert!(parse_error(too_many_files.as_bytes()).contains("file limit"));
}

#[test]
fn bundle_verification_requires_an_exact_regular_file_tree() {
    let directory = TestDirectory::new();
    write_bundle(directory.path(), None);

    let bundle = BackendBundle::verify(directory.path()).unwrap();
    assert_eq!(
        bundle.executable_relative().unwrap(),
        Path::new("bsdtar.exe")
    );

    fs::write(directory.path().join("unexpected.dll"), b"extra").unwrap();
    let error = BackendBundle::verify(directory.path())
        .unwrap_err()
        .to_string();
    assert!(error.contains("does not exactly match its manifest"));
    assert!(error.contains("unexpected.dll"));
}

#[test]
fn bundle_verification_rejects_a_digest_mismatch() {
    let directory = TestDirectory::new();
    write_bundle(directory.path(), Some(ZERO_HASH));

    let error = BackendBundle::verify(directory.path())
        .unwrap_err()
        .to_string();
    assert!(error.contains("SHA-256 mismatch"));
    assert!(error.contains("bsdtar.exe"));
}
