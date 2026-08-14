#[test]
fn packaged_release_documents_match_the_crate_version() {
    let version = env!("CARGO_PKG_VERSION");
    let components = version.split('.').collect::<Vec<_>>();
    assert_eq!(components.len(), 3, "release version must be X.Y.Z");
    assert!(
        components.iter().all(|component| !component.is_empty()
            && component.chars().all(|value| value.is_ascii_digit())),
        "release version must contain only numeric components"
    );

    let supported_line = format!("{}.{}.x", components[0], components[1]);
    let tag = format!("v{version}");
    let x64_zip = format!("iroha-zip-{version}-windows-x64.zip");
    let arm64_zip = format!("iroha-zip-{version}-windows-arm64.zip");

    let security = include_str!("../SECURITY.md");
    for expected in [
        format!("iroha-zip {supported_line} は"),
        format!("最新の `main` と `{supported_line}` を対象とします。"),
        format!("iroha-zip {supported_line} is"),
        format!("The latest `main` branch and `{supported_line}` are supported."),
    ] {
        assert!(
            security.contains(&expected),
            "SECURITY.md is missing the current support marker: {expected}"
        );
    }

    let readme_ja = include_str!("../README.md");
    let readme_en = include_str!("../README.en.md");
    assert!(readme_ja.contains(&tag), "README.md is missing {tag}");
    assert!(readme_en.contains(&tag), "README.en.md is missing {tag}");
    assert!(readme_ja.contains(&x64_zip));
    assert!(readme_en.contains(&x64_zip));

    let release_notes = include_str!("../docs/RELEASE_NOTES.md");
    assert!(release_notes.starts_with(&format!("# iroha-zip {version}\n")));
    assert!(release_notes.contains(&x64_zip));
    assert!(release_notes.contains(&arm64_zip));
    assert_eq!(release_notes.matches("## 日本語").count(), 1);
    assert_eq!(release_notes.matches("## English").count(), 1);

    let verification = include_str!("../docs/RELEASE_VERIFICATION.md");
    assert!(verification.contains(&format!("Version `{version}` contains exactly 11 assets.")));
    assert!(verification.contains(&format!("refs/tags/{tag}")));
    assert!(verification.contains(&arm64_zip));

    let unsigned = include_str!("../docs/UNSIGNED_RELEASE.md");
    assert!(unsigned.contains(&x64_zip));
    assert!(unsigned.contains(&arm64_zip));

    let arm64 = include_str!("../docs/ARM64.md");
    assert!(arm64.contains(&arm64_zip));

    let updater = include_str!("../docs/UPDATER.md");
    assert!(updater.contains(&tag));
}
