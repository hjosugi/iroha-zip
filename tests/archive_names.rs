use iroha_zip::util::archive_base_name;

#[test]
fn strips_compound_archive_extensions_case_insensitively() {
    assert_eq!(archive_base_name("backup.tar.gz"), "backup");
    assert_eq!(archive_base_name("backup.TAR.XZ"), "backup");
    assert_eq!(archive_base_name("backup.tar.Z"), "backup");
    assert_eq!(archive_base_name("backup.tzst"), "backup");
}

#[test]
fn strips_simple_extension_and_keeps_useful_name() {
    assert_eq!(archive_base_name("photos.zip"), "photos");
    assert_eq!(archive_base_name("archive"), "archive");
    assert_eq!(archive_base_name(".zip"), ".zip");
}
