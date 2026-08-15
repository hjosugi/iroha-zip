use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

const SNAPSHOT_ROOT: &str = "evidence/windows/31868019031";
const REPORTS: [&str; 11] = [
    "windows-arm64-native/windows-arm64-e2e.json",
    "windows-arm64-native/windows-arm64-malicious-corpus.json",
    "windows-arm64-native/windows-arm64-native.json",
    "windows-arm64-native/windows-arm64-settings-en.json",
    "windows-arm64-native/windows-arm64-settings-ja.json",
    "windows-e2e-windows-2022/malicious-corpus.json",
    "windows-e2e-windows-2022/settings-e2e.json",
    "windows-e2e-windows-2022/windows-e2e.json",
    "windows-e2e-windows-2025/malicious-corpus.json",
    "windows-e2e-windows-2025/settings-e2e.json",
    "windows-e2e-windows-2025/windows-e2e.json",
];

fn snapshot_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(SNAPSHOT_ROOT)
}

fn parse_sha256_manifest(name: &str) -> BTreeMap<String, String> {
    let contents = fs::read_to_string(snapshot_root().join(name))
        .unwrap_or_else(|error| panic!("cannot read {name}: {error}"));
    let mut entries = BTreeMap::new();
    for (index, line) in contents.lines().enumerate() {
        let (digest, path) = line
            .split_once("  ")
            .unwrap_or_else(|| panic!("{name}:{} is not a SHA-256 manifest line", index + 1));
        assert!(
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
            "{name}:{} has a non-canonical digest",
            index + 1
        );
        assert!(
            !path.is_empty()
                && Path::new(path)
                    .components()
                    .all(|component| matches!(component, Component::Normal(_))),
            "{name}:{} has an unsafe path",
            index + 1
        );
        assert!(
            entries.insert(path.to_owned(), digest.to_owned()).is_none(),
            "{name} contains a duplicate path: {path}"
        );
    }
    entries
}

fn lowercase_sha256(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn require_true(value: &Value, pointer: &str, report: &str) {
    assert_eq!(
        value.pointer(pointer).and_then(Value::as_bool),
        Some(true),
        "{report} is missing true at {pointer}"
    );
}

fn require_array_len(value: &Value, pointer: &str, length: usize, report: &str) {
    assert_eq!(
        value
            .pointer(pointer)
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(length),
        "{report} has an unexpected array length at {pointer}"
    );
}

fn validate_archive_e2e(value: &Value, report: &str) {
    assert_eq!(value["schemaVersion"], 5);
    assert_eq!(value["status"], "passed");
    assert!(value["failure"].is_null());
    require_true(value, "/cleanup/temporaryRootRemoved", report);
    require_true(value, "/sourceTree/includesJapaneseNames", report);
    require_true(value, "/sourceTree/includesEmptyDirectory", report);
    require_array_len(value, "/formats", 4, report);
    require_array_len(value, "/readFixtures", 14, report);
    require_array_len(value, "/encryptedArchives", 3, report);
    require_true(
        value,
        "/encryptedArchiveFailures/wrongPasswordRejected",
        report,
    );
    require_true(
        value,
        "/encryptedArchiveFailures/wrongPasswordDestinationAbsent",
        report,
    );
    require_true(value, "/encryptedArchiveFailures/cancelOutputEmpty", report);
    require_true(
        value,
        "/encryptedArchiveFailures/cancelDestinationAbsent",
        report,
    );
    for encrypted in value["encryptedArchives"]
        .as_array()
        .expect("encryptedArchives must be an array")
    {
        require_true(encrypted, "/nativeBilingualDialog", report);
        require_true(encrypted, "/passwordControlProtected", report);
        require_true(encrypted, "/passwordAbsentFromOutput", report);
        require_true(encrypted, "/oneUseChannel", report);
    }
    require_true(value, "/rawStreamNegative/filterMismatchRejected", report);
    require_true(value, "/rawStreamNegative/byteLimitRejected", report);
    require_true(
        value,
        "/rawStreamNegative/compressedPayloadCorruptionRejected",
        report,
    );
    require_true(value, "/rawStreamNegative/allDestinationsAbsent", report);
    require_true(value, "/pinnedFixtureDecoderSelfTest/passed", report);
    require_true(value, "/invalidArchive/rejected", report);
    require_true(value, "/invalidArchive/destinationAbsent", report);
    assert_eq!(value["isolation"]["schemaVersion"], 4);
    assert_eq!(value["isolation"]["requestedMode"], "appcontainer");
    require_true(value, "/isolation/token/isAppContainer", report);
    assert_eq!(value["isolation"]["token"]["capabilityCount"], 0);
    require_true(value, "/isolation/network/denied", report);
    require_true(value, "/isolation/timeout/rejected", report);
    require_true(value, "/isolation/memory/rejected", report);
    require_true(value, "/isolation/crash/terminatedWithoutSuccess", report);
    require_true(
        value,
        "/isolation/loaderFailure/createProcessRejected",
        report,
    );
    require_true(value, "/isolation/processTemp/rngSucceeded", report);
    require_true(
        value,
        "/isolation/processTemp/deleteOnCloseSucceeded",
        report,
    );
    require_true(value, "/isolation/stagingWriteSeal/aclApplied", report);
    require_array_len(value, "/isolation/cleanup", 7, report);
    for cleanup in value["isolation"]["cleanup"]
        .as_array()
        .expect("cleanup must be an array")
    {
        require_true(cleanup, "/profileDeleteSucceeded", report);
        require_true(cleanup, "/temporaryRootRemoved", report);
    }
    assert_eq!(value["lpac"]["supported"], false);
    assert_eq!(
        value["lpac"]["failureClass"],
        "token-query-invalid-parameter"
    );
    require_true(value, "/lpac/failClosed", report);
}

fn validate_corpus(value: &Value, report: &str) {
    assert_eq!(value["schemaVersion"], 1);
    assert_eq!(value["status"], "passed");
    assert!(value["failure"].is_null());
    require_true(value, "/temporaryRootRemoved", report);
    require_array_len(value, "/samples", 19, report);
    require_array_len(value, "/policyFixtures", 3, report);
    let samples = value["samples"]
        .as_array()
        .expect("samples must be an array");
    let control = samples
        .iter()
        .find(|sample| sample["id"] == "control-zip")
        .expect("the benign control must be present");
    assert_eq!(control["expected"], "accept-control");
    assert_eq!(control["exitCode"], 0);
    require_true(control, "/destinationPublished", report);
    for sample in samples
        .iter()
        .filter(|sample| sample["id"] != "control-zip")
    {
        assert_eq!(sample["expected"], "reject-before-publication");
        assert_ne!(sample["exitCode"], 0);
        assert_eq!(sample["destinationPublished"], false);
    }
    for fixture in value["policyFixtures"]
        .as_array()
        .expect("policyFixtures must be an array")
    {
        require_true(fixture, "/rejected", report);
        assert_eq!(fixture["errorClass"], "policy");
    }
}

fn validate_settings(value: &Value, report: &str) {
    assert_eq!(value["schemaVersion"], 1);
    assert_eq!(value["status"], "passed");
    assert_eq!(value["controlCount"], 26);
    assert_eq!(value["dpiAwareness"], "PerMonitorV2");
    assert_eq!(
        value["syntheticDpiTransitions"],
        serde_json::json!([96, 144, 96])
    );
    assert_eq!(value["safeFolderPickerCancellations"], 3);
    for pointer in [
        "/restoreDefaultsCancelAndConfirm",
        "/cancelButtonDiscardCancelAndConfirm",
        "/longAndNonAsciiPathEdited",
        "/unsavedChangeConfirmationExposed",
        "/backendPathSaved",
        "/backendDoctorSucceeded",
        "/temporaryRootRemoved",
    ] {
        require_true(value, pointer, report);
    }
}

fn validate_arm64_identity(value: &Value, report: &str) {
    assert_eq!(value["schemaVersion"], 2);
    assert_eq!(value["runner"]["osArchitecture"], "Arm64");
    assert_eq!(value["runner"]["processArchitecture"], "Arm64");
    assert_eq!(value["peMachine"], "0xAA64");
    assert_eq!(value["backendPeMachine"], "0xAA64");
    assert_eq!(value["binarySha256"], value["isolation"]["probeSha256"]);
    require_true(value, "/backendArchiveMatrixExecuted", report);
    require_true(value, "/maliciousCorpusExecuted", report);
    require_true(value, "/settingsMatrixExecuted", report);
    assert_eq!(value["releaseAssetPublished"], false);
}

#[test]
fn durable_windows_evidence_inventory_hashes_and_contracts_match() {
    let expected = REPORTS
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let source_manifest = parse_sha256_manifest("SOURCE_SHA256SUMS.txt");
    let canonical_manifest = parse_sha256_manifest("SHA256SUMS.txt");
    assert_eq!(
        source_manifest.keys().cloned().collect::<BTreeSet<_>>(),
        expected
    );
    assert_eq!(
        canonical_manifest.keys().cloned().collect::<BTreeSet<_>>(),
        expected
    );

    for report in REPORTS {
        let bytes = fs::read(snapshot_root().join(report))
            .unwrap_or_else(|error| panic!("cannot read {report}: {error}"));
        assert_eq!(
            lowercase_sha256(&bytes),
            canonical_manifest[report],
            "canonical snapshot hash changed for {report}"
        );
        let value: Value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("cannot parse {report}: {error}"));
        if report.ends_with("windows-e2e.json") || report.ends_with("windows-arm64-e2e.json") {
            validate_archive_e2e(&value, report);
        } else if report.contains("malicious-corpus") {
            validate_corpus(&value, report);
        } else if report.contains("settings-") || report.ends_with("settings-e2e.json") {
            validate_settings(&value, report);
        } else if report.ends_with("windows-arm64-native.json") {
            validate_arm64_identity(&value, report);
        } else {
            panic!("unclassified evidence report: {report}");
        }
    }

    let readme = fs::read_to_string(snapshot_root().join("README.md"))
        .expect("the evidence snapshot README must be present");
    for marker in [
        "31868019031",
        "5cbc6c27fb67466369b20180a9c5aa2fdd3f6713",
        "SOURCE_SHA256SUMS.txt",
        "SHA256SUMS.txt",
        "diagnostic evidence",
    ] {
        assert!(
            readme.contains(marker),
            "snapshot README is missing {marker}"
        );
    }
}
