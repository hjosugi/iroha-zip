use iroha_zip::policy::{self, Limits};

#[test]
fn accepts_normalized_files_and_directory_markers() {
    let listing = b"docs/\ndocs/readme.txt\nimages\\logo.png\r\n";
    assert_eq!(
        policy::validate_archive_listing(listing, &Limits::default()).unwrap(),
        3
    );
    assert_eq!(
        policy::validate_archive_listing(b"", &Limits::default()).unwrap(),
        0
    );
}

#[test]
fn rejects_unsafe_raw_archive_names_before_backend_normalization() {
    for listing in [
        b"../escape.txt\n".as_slice(),
        b"/absolute.txt\n",
        b"C:/drive.txt\n",
        b"\\\\server\\share\\unc.txt\n",
        b"safe.txt:stream\n",
        b"CON.txt\n",
        b"alias.\n",
        b"alias \n",
        b"bad?.txt\n",
        b"./\n",
        b"./not-normalized.txt\n",
        b"folder//empty.txt\n",
        b"\n",
        b"line\rbreak.txt\n",
        b"invalid-utf8-\xff\n",
    ] {
        assert!(
            policy::validate_archive_listing(listing, &Limits::default()).is_err(),
            "unsafe listing was accepted: {listing:?}"
        );
    }
}

#[test]
fn rejects_duplicates_and_case_aliases_across_separator_styles() {
    for listing in [
        b"duplicate.txt\nduplicate.txt\n".as_slice(),
        b"Folder/File.txt\nfolder/file.TXT\n",
        b"folder\\file.txt\nfolder/file.txt\n",
        b"folder\nfolder/\n",
    ] {
        assert!(policy::validate_archive_listing(listing, &Limits::default()).is_err());
    }
}

#[test]
fn enforces_listing_count_depth_and_path_limits() {
    let limits = Limits {
        max_files: 1,
        max_directories: 1,
        max_depth: 2,
        max_path_bytes: 12,
        ..Limits::default()
    };
    assert!(policy::validate_archive_listing(b"a\nb\nc\n", &limits).is_err());
    assert!(policy::validate_archive_listing(b"a/b/c\n", &limits).is_err());
    assert!(policy::validate_archive_listing(b"123456789012\n", &limits).is_err());
}
