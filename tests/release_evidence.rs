use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy)]
struct ReleaseSnapshot {
    root: &'static str,
    version: &'static str,
}

const RELEASE_SNAPSHOTS: [ReleaseSnapshot; 2] = [
    ReleaseSnapshot {
        root: "evidence/releases/v0.6.1",
        version: "0.6.1",
    },
    ReleaseSnapshot {
        root: "evidence/releases/v0.6.2",
        version: "0.6.2",
    },
];

fn snapshot_root(snapshot: ReleaseSnapshot) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(snapshot.root)
}

fn parse_sha256_manifest(path: &Path) -> BTreeMap<String, String> {
    let contents = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let mut entries = BTreeMap::new();
    for (index, line) in contents.lines().enumerate() {
        let (digest, name) = line.split_once("  ").unwrap_or_else(|| {
            panic!(
                "{}:{} is not a SHA-256 manifest line",
                path.display(),
                index + 1
            )
        });
        assert!(
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
            "{}:{} has a non-canonical digest",
            path.display(),
            index + 1
        );
        assert!(
            !name.is_empty()
                && Path::new(name)
                    .components()
                    .all(|component| matches!(component, Component::Normal(_))),
            "{}:{} has an unsafe name",
            path.display(),
            index + 1
        );
        assert!(
            entries.insert(name.to_owned(), digest.to_owned()).is_none(),
            "{} contains duplicate name {name}",
            path.display()
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

fn expected_asset_names(version: &str) -> BTreeSet<String> {
    [
        format!("iroha-zip-{version}-windows-arm64.exe"),
        format!("iroha-zip-{version}-windows-arm64.zip"),
        format!("iroha-zip-{version}-windows-arm64.zip.sha256"),
        format!("iroha-zip-{version}-windows-x64.exe"),
        format!("iroha-zip-{version}-windows-x64.zip"),
        format!("iroha-zip-{version}-windows-x64.zip.sha256"),
        format!("iroha-zip-settings-{version}-windows-arm64.exe"),
        format!("iroha-zip-settings-{version}-windows-x64.exe"),
        format!("iroha-zip-shell-{version}-windows-arm64.exe"),
        format!("iroha-zip-shell-{version}-windows-x64.exe"),
        "SHA256SUMS.txt".to_owned(),
    ]
    .into_iter()
    .collect()
}

fn validate_release_snapshot(snapshot: ReleaseSnapshot) {
    let root = snapshot_root(snapshot);
    let metadata_bytes = fs::read(root.join("release.json")).expect("release evidence must exist");
    let metadata: Value =
        serde_json::from_slice(&metadata_bytes).expect("release evidence must be valid JSON");
    let version = snapshot.version;
    let tag = format!("v{version}");

    assert_eq!(metadata["schema_version"], 1);
    assert_eq!(metadata["repository"], "hjosugi/iroha-zip");
    assert_eq!(metadata["tag"]["name"], tag);
    let commit = metadata["tag"]["commit_sha"]
        .as_str()
        .expect("tag commit must be a string");
    assert_eq!(commit.len(), 40);
    assert!(commit.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let tag_object = metadata["tag"]["annotated_tag_object_sha"]
        .as_str()
        .expect("annotated tag object must be recorded");
    assert_eq!(tag_object.len(), 40);
    assert_ne!(tag_object, commit);

    let release = &metadata["release"];
    assert_eq!(release["name"], format!("iroha-zip {version}"));
    assert_eq!(release["draft"], false);
    assert_eq!(release["prerelease"], false);
    assert_eq!(release["immutable"], true);
    assert_eq!(release["latest"], true);
    assert_eq!(release["asset_count"], 11);

    let expected_assets = expected_asset_names(version);
    let downloaded_hashes = parse_sha256_manifest(&root.join("RELEASE_ASSETS_SHA256SUMS.txt"));
    assert_eq!(
        downloaded_hashes.keys().cloned().collect::<BTreeSet<_>>(),
        expected_assets
    );

    let assets = metadata["assets"]
        .as_array()
        .expect("release assets must be an array");
    assert_eq!(assets.len(), 11);
    let mut recorded_assets = BTreeSet::new();
    let mut executable_count = 0;
    for asset in assets {
        let name = asset["name"].as_str().expect("asset name must be a string");
        assert!(
            recorded_assets.insert(name.to_owned()),
            "duplicate asset {name}"
        );
        assert!(asset["id"].as_u64().is_some_and(|id| id > 0));
        assert!(asset["size"].as_u64().is_some_and(|size| size > 0));
        assert_eq!(asset["state"], "uploaded");
        let api_digest = asset["digest"]
            .as_str()
            .and_then(|value| value.strip_prefix("sha256:"))
            .expect("asset digest must have the sha256 prefix");
        assert_eq!(
            downloaded_hashes.get(name).map(String::as_str),
            Some(api_digest),
            "downloaded and API digests differ for {name}"
        );
        if Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        {
            executable_count += 1;
        }
    }
    assert_eq!(recorded_assets, expected_assets);
    assert_eq!(executable_count, 6);

    let workflows = metadata["workflows"]
        .as_object()
        .expect("workflow evidence must be an object");
    assert_eq!(workflows.len(), 6);
    for (name, workflow) in workflows {
        assert_eq!(workflow["commit_sha"], commit, "{name} used another commit");
        assert_eq!(workflow["conclusion"], "success", "{name} did not pass");
        assert!(workflow["run_id"].as_u64().is_some_and(|id| id > 0));
    }
    assert_eq!(workflows["dry_run"]["publish"], false);
    assert_eq!(workflows["dry_run"]["ref"], "main");
    assert_eq!(workflows["release"]["publish"], true);
    assert_eq!(workflows["release"]["ref"], tag);
    assert_eq!(workflows["tag_ci"]["ref"], tag);

    let workflow_artifacts = metadata["workflow_artifacts"]
        .as_object()
        .expect("workflow artifact evidence must be an object");
    for (name, expected_count) in [("dry_run", 3), ("release", 3), ("tag_ci", 1)] {
        let artifacts = workflow_artifacts[name]
            .as_array()
            .unwrap_or_else(|| panic!("{name} artifacts must be an array"));
        assert_eq!(artifacts.len(), expected_count);
        for artifact in artifacts {
            assert!(artifact["id"].as_u64().is_some_and(|id| id > 0));
            assert!(artifact["size"].as_u64().is_some_and(|size| size > 0));
            let digest = artifact["digest"]
                .as_str()
                .and_then(|value| value.strip_prefix("sha256:"))
                .expect("artifact digest must have the sha256 prefix");
            assert_eq!(digest.len(), 64);
        }
    }

    let attested = metadata["attestations"]["subjects"]
        .as_array()
        .expect("attestation subjects must be an array")
        .iter()
        .map(|subject| {
            subject
                .as_str()
                .expect("attestation subject must be a string")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    let sidecars = [
        format!("iroha-zip-{version}-windows-arm64.zip.sha256"),
        format!("iroha-zip-{version}-windows-x64.zip.sha256"),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let expected_attested = expected_assets
        .iter()
        .filter(|name| !sidecars.contains(*name))
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(attested, expected_attested);
    assert_eq!(attested.len(), 9);
    assert_eq!(metadata["attestations"]["verified"], true);
    assert_eq!(
        metadata["attestations"]["source_ref"],
        format!("refs/tags/{tag}")
    );
    assert_eq!(metadata["attestations"]["deny_self_hosted_runners"], true);

    let checks = &metadata["independent_verification"];
    for field in [
        "all_public_assets_downloaded",
        "release_api_size_and_digest_match",
        "exact_asset_inventory_match",
        "zip_to_standalone_executables_byte_identical",
        "backend_binaries_absent",
        "bilingual_package_documents_match_source",
        "release_body_matches_source",
        "public_pages_match_source",
    ] {
        assert_eq!(
            checks[field], true,
            "independent check {field} did not pass"
        );
    }
    assert_eq!(checks["combined_checksum_subject_count"], 8);
    assert_eq!(checks["zip_sidecar_count"], 2);
    assert_eq!(checks["x64_pe_machine"], "0x8664");
    assert_eq!(checks["arm64_pe_machine"], "0xAA64");
    assert_eq!(checks["distinct_executable_count"], 6);
    assert_eq!(checks["empty_authenticode_certificate_table_count"], 6);
    assert_eq!(checks["authenticode_signed"], false);

    let snapshot_hashes = parse_sha256_manifest(&root.join("SHA256SUMS.txt"));
    assert_eq!(snapshot_hashes.len(), 2);
    for name in ["release.json", "RELEASE_ASSETS_SHA256SUMS.txt"] {
        let bytes = fs::read(root.join(name)).expect("snapshot file must exist");
        assert_eq!(
            snapshot_hashes.get(name).map(String::as_str),
            Some(lowercase_sha256(&bytes).as_str()),
            "snapshot hash differs for {name}"
        );
    }
}

#[test]
fn completed_release_evidence_is_complete_and_self_consistent() {
    for snapshot in RELEASE_SNAPSHOTS {
        validate_release_snapshot(snapshot);
    }
}
