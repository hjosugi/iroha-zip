use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use sha1::Sha1;
use sha2::Digest;

use crate::backend::{BackendBundle, MANIFEST_FILE, sha256_file, validate_manifest_path};
use crate::error::{IrohaZipError, Result};
use crate::platform;
use crate::util::hex_lower;

pub const EVIDENCE_DIRECTORY: &str = ".iroha-zip-evidence";
pub const PROVENANCE_FILE: &str = "backend-provenance.json";
pub const SPDX_FILE: &str = "backend.spdx.json";
pub const LICENSE_INVENTORY_FILE: &str = "backend-license-inventory.json";
pub const NOTICES_FILE: &str = "THIRD-PARTY-NOTICES.md";
pub const MAX_EVIDENCE_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_EVIDENCE_FILES: usize = 1024;
pub const MAX_EVIDENCE_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
const MAX_EVIDENCE_ENTRIES: usize = 2048;
const MAX_PACKAGES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendEvidence {
    root: PathBuf,
    source_kind: String,
    verification_method: String,
    supported: bool,
    package_count: usize,
    file_count: usize,
}

impl BackendEvidence {
    pub fn verify(backend: &BackendBundle) -> Result<Self> {
        let root = backend.root().join(EVIDENCE_DIRECTORY);
        validate_evidence_root(&root)?;
        let evidence_tree = collect_evidence_tree(&root)?;

        let provenance: ProvenanceDocument = read_json(&root.join(PROVENANCE_FILE))?;
        let sbom: SpdxDocument = read_json(&root.join(SPDX_FILE))?;
        let inventory: LicenseInventory = read_json(&root.join(LICENSE_INVENTORY_FILE))?;

        let expected_payload = payload_map(backend)?;
        let provenance_state = validate_provenance(backend, &provenance, &expected_payload)?;
        validate_spdx(backend, &provenance_state, &sbom, &expected_payload)?;
        let expected_evidence =
            validate_inventory(&root, &provenance_state, &inventory, &expected_payload)?;
        validate_evidence_tree(&evidence_tree, &expected_evidence)?;

        Ok(Self {
            root,
            source_kind: provenance.source.kind,
            verification_method: provenance.source.verification.method,
            supported: provenance.source.supported,
            package_count: provenance_state.packages.len(),
            file_count: expected_payload.len(),
        })
    }

    pub fn directory_for(backend_root: &Path) -> PathBuf {
        backend_root.join(EVIDENCE_DIRECTORY)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn source_kind(&self) -> &str {
        &self.source_kind
    }

    pub fn verification_method(&self) -> &str {
        &self.verification_method
    }

    pub fn is_supported(&self) -> bool {
        self.supported
    }

    pub fn package_count(&self) -> usize {
        self.package_count
    }

    pub fn file_count(&self) -> usize {
        self.file_count
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProvenanceDocument {
    schema_version: u32,
    created_at_utc: String,
    source: ProvenanceSource,
    manifest: EvidenceDigest,
    packages: Vec<ProvenancePackage>,
    files: Vec<PayloadRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProvenanceSource {
    kind: String,
    supported: bool,
    repository: String,
    verification: Verification,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Verification {
    status: String,
    method: String,
    keyring_package: Option<String>,
    keyring_version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EvidenceDigest {
    path: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProvenancePackage {
    id: String,
    name: String,
    version: String,
    architecture: String,
    repository: String,
    download_url: Option<String>,
    archive_sha256: Option<String>,
    signature: Option<String>,
    licenses: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PayloadRecord {
    path: String,
    sha256: String,
    package_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LicenseInventory {
    schema_version: u32,
    notice: EvidenceDigest,
    packages: Vec<InventoryPackage>,
    files: Vec<PayloadRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InventoryPackage {
    id: String,
    name: String,
    version: String,
    licenses: Vec<String>,
    license_files: Vec<EvidenceDigest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SpdxDocument {
    spdx_version: String,
    data_license: String,
    #[serde(rename = "SPDXID")]
    spdx_id: String,
    name: String,
    document_namespace: String,
    creation_info: SpdxCreationInfo,
    packages: Vec<SpdxPackage>,
    files: Vec<SpdxFile>,
    relationships: Vec<SpdxRelationship>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SpdxCreationInfo {
    created: String,
    creators: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SpdxPackage {
    #[serde(rename = "SPDXID")]
    spdx_id: String,
    name: String,
    version_info: String,
    download_location: String,
    files_analyzed: bool,
    package_verification_code: SpdxVerificationCode,
    license_concluded: String,
    license_info_from_files: Vec<String>,
    license_declared: String,
    license_comments: String,
    copyright_text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SpdxVerificationCode {
    package_verification_code_value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SpdxFile {
    #[serde(rename = "SPDXID")]
    spdx_id: String,
    file_name: String,
    checksums: Vec<SpdxChecksum>,
    file_types: Vec<String>,
    license_concluded: String,
    license_info_in_files: Vec<String>,
    copyright_text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SpdxChecksum {
    algorithm: String,
    checksum_value: String,
}

#[derive(Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SpdxRelationship {
    spdx_element_id: String,
    relationship_type: String,
    related_spdx_element: String,
}

struct ProvenanceState {
    packages: BTreeMap<String, ProvenancePackage>,
    payload_owners: BTreeMap<String, String>,
    payload_order: Vec<String>,
    created_at_utc: String,
}

struct EvidenceTree {
    files: BTreeSet<PathBuf>,
    directories: BTreeSet<PathBuf>,
}

fn evidence_error(message: impl Into<String>) -> IrohaZipError {
    IrohaZipError::Backend(format!("backend evidence is invalid: {}", message.into()))
}

fn validate_evidence_root(root: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|error| IrohaZipError::io_path("cannot inspect backend evidence", root, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(evidence_error(format!(
            "evidence root must be a regular directory: {}",
            root.display()
        )));
    }
    platform::validate_directory_security(root)
        .map_err(|error| evidence_error(format!("unsafe evidence root: {error}")))
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        IrohaZipError::io_path("cannot inspect backend evidence document", path, error)
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(evidence_error(format!(
            "evidence document must be a regular file: {}",
            path.display()
        )));
    }
    platform::validate_extracted_entry_security(path, &metadata).map_err(|error| {
        evidence_error(format!(
            "unsafe evidence document {}: {error}",
            path.display()
        ))
    })?;
    let mut input = File::open(path).map_err(|error| {
        IrohaZipError::io_path("cannot open backend evidence document", path, error)
    })?;
    let mut bytes = Vec::new();
    input
        .by_ref()
        .take(MAX_EVIDENCE_DOCUMENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            IrohaZipError::io_path("cannot read backend evidence document", path, error)
        })?;
    if bytes.len() > MAX_EVIDENCE_DOCUMENT_BYTES {
        return Err(evidence_error(format!(
            "document exceeds the {MAX_EVIDENCE_DOCUMENT_BYTES} byte limit: {}",
            path.display()
        )));
    }
    serde_json::from_slice(&bytes)
        .map_err(|error| evidence_error(format!("cannot parse {}: {error}", path.display())))
}

fn payload_map(backend: &BackendBundle) -> Result<BTreeMap<String, String>> {
    backend
        .files()
        .map(|(path, hash)| {
            let path = path
                .to_str()
                .ok_or_else(|| evidence_error("manifest path cannot be represented as UTF-8"))?;
            Ok((path.replace('\\', "/"), hash.to_owned()))
        })
        .collect()
}

fn validate_provenance(
    backend: &BackendBundle,
    document: &ProvenanceDocument,
    expected_payload: &BTreeMap<String, String>,
) -> Result<ProvenanceState> {
    if document.schema_version != 1 || !is_utc_second_timestamp(&document.created_at_utc) {
        return Err(evidence_error(
            "unsupported provenance schemaVersion or invalid creation timestamp",
        ));
    }
    let source = &document.source;
    if source.supported {
        if source.kind != "msys2-ucrt64-pacman"
            || source.repository != "ucrt64"
            || source.verification.status != "verified"
            || source.verification.method != "pacman-required-trusted-only"
            || source.verification.keyring_package.as_deref() != Some("msys2-keyring")
            || source
                .verification
                .keyring_version
                .as_deref()
                .is_none_or(|version| !is_bounded_text(version, 256))
        {
            return Err(evidence_error(
                "supported provenance must be verified MSYS2 UCRT64 pacman evidence",
            ));
        }
    } else if source.kind != "unverified-local-bundle"
        || source.repository != "unverified-local"
        || source.verification.status != "unverified"
        || source.verification.method != "explicit-user-accepted-local-bundle"
        || source.verification.keyring_package.is_some()
        || source.verification.keyring_version.is_some()
    {
        return Err(evidence_error(
            "unsupported provenance does not carry the required explicit warning state",
        ));
    }

    if document.manifest.path != MANIFEST_FILE || !is_sha256(&document.manifest.sha256) {
        return Err(evidence_error("manifest evidence record is invalid"));
    }
    let manifest_hash = sha256_file(&backend.root().join(MANIFEST_FILE))?;
    if document.manifest.sha256 != manifest_hash {
        return Err(evidence_error("manifest SHA-256 does not match"));
    }

    if document.packages.is_empty() || document.packages.len() > MAX_PACKAGES {
        return Err(evidence_error("provenance package count is out of bounds"));
    }
    let mut packages = BTreeMap::new();
    let mut previous_id: Option<&str> = None;
    for package in &document.packages {
        validate_package(package, source.supported)?;
        if previous_id.is_some_and(|previous| previous >= package.id.as_str()) {
            return Err(evidence_error(
                "provenance packages must be uniquely sorted by id",
            ));
        }
        previous_id = Some(&package.id);
        packages.insert(package.id.clone(), package.clone());
    }
    if !source.supported {
        let Some(package) = packages.get("unverified-local-bundle") else {
            return Err(evidence_error(
                "unsupported provenance must use the unverified local package identity",
            ));
        };
        if packages.len() != 1
            || package.name != "unverified-local-bundle"
            || package.version != "NOASSERTION"
            || package.architecture != "windows"
            || package.repository != "unverified-local"
            || package.licenses != ["NOASSERTION"]
        {
            return Err(evidence_error(
                "unsupported provenance package metadata is not explicit and canonical",
            ));
        }
    }

    let (payload, owners) = payload_records(&document.files, &packages)?;
    if &payload != expected_payload {
        return Err(evidence_error(
            "provenance file list does not exactly match the backend manifest",
        ));
    }
    let represented_packages: BTreeSet<_> = owners.values().collect();
    if represented_packages.len() != packages.len()
        || packages
            .keys()
            .any(|package_id| !represented_packages.contains(package_id))
    {
        return Err(evidence_error(
            "provenance contains a package that owns no payload file",
        ));
    }

    Ok(ProvenanceState {
        packages,
        payload_owners: owners,
        payload_order: document
            .files
            .iter()
            .map(|record| record.path.clone())
            .collect(),
        created_at_utc: document.created_at_utc.clone(),
    })
}

fn validate_package(package: &ProvenancePackage, supported: bool) -> Result<()> {
    if package.id.is_empty()
        || package.id.len() > 256
        || !package
            .id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
        || !is_bounded_text(&package.name, 512)
        || !is_bounded_text(&package.version, 256)
        || !is_bounded_text(&package.architecture, 64)
        || !is_bounded_text(&package.repository, 128)
        || package.licenses.is_empty()
        || package.licenses.len() > 64
        || package
            .licenses
            .iter()
            .any(|license| !is_bounded_text(license, 256))
    {
        return Err(evidence_error(format!(
            "invalid package metadata for {:?}",
            package.id
        )));
    }
    if supported {
        if package.repository != "ucrt64"
            || package.name != package.id
            || !package.id.starts_with("mingw-w64-ucrt-x86_64-")
            || package
                .download_url
                .as_deref()
                .is_none_or(|value| !value.starts_with("https://") || !is_bounded_text(value, 4096))
            || package
                .archive_sha256
                .as_deref()
                .is_none_or(|value| !is_sha256(value))
            || package.signature.as_deref().is_none_or(|value| {
                value.len() < 32
                    || value.len() > 32 * 1024
                    || !value.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')
                    })
            })
        {
            return Err(evidence_error(format!(
                "verified package metadata is incomplete for {:?}",
                package.id
            )));
        }
    } else if package.download_url.is_some()
        || package.archive_sha256.is_some()
        || package.signature.is_some()
    {
        return Err(evidence_error(
            "unsupported package metadata must not imply verified archive evidence",
        ));
    }
    Ok(())
}

fn payload_records(
    records: &[PayloadRecord],
    packages: &BTreeMap<String, ProvenancePackage>,
) -> Result<(BTreeMap<String, String>, BTreeMap<String, String>)> {
    let mut payload = BTreeMap::new();
    let mut owners = BTreeMap::new();
    for record in records {
        validate_manifest_path(&record.path)
            .map_err(|_| evidence_error(format!("invalid payload path: {:?}", record.path)))?;
        if !is_sha256(&record.sha256) || !packages.contains_key(&record.package_id) {
            return Err(evidence_error(format!(
                "invalid payload evidence for {:?}",
                record.path
            )));
        }
        if payload
            .insert(record.path.clone(), record.sha256.clone())
            .is_some()
        {
            return Err(evidence_error("payload evidence contains duplicate paths"));
        }
        owners.insert(record.path.clone(), record.package_id.clone());
    }
    Ok((payload, owners))
}

fn validate_spdx(
    backend: &BackendBundle,
    provenance: &ProvenanceState,
    document: &SpdxDocument,
    expected_payload: &BTreeMap<String, String>,
) -> Result<()> {
    let manifest_hash = sha256_file(&backend.root().join(MANIFEST_FILE))?;
    let provenance_hash = sha256_file(
        &backend
            .root()
            .join(EVIDENCE_DIRECTORY)
            .join(PROVENANCE_FILE),
    )?;
    if document.spdx_version != "SPDX-2.3"
        || document.data_license != "CC0-1.0"
        || document.spdx_id != "SPDXRef-DOCUMENT"
        || document.name != "iroha-zip backend"
        || document.document_namespace
            != format!(
                "https://github.com/hjosugi/iroha-zip/backend-evidence/{manifest_hash}-{provenance_hash}"
            )
        || document.creation_info.created != provenance.created_at_utc
        || document.creation_info.creators != ["Tool: iroha-zip-backend-evidence"]
    {
        return Err(evidence_error("SPDX document metadata is invalid"));
    }

    if document.packages.len() != provenance.packages.len()
        || document.files.len() != expected_payload.len()
    {
        return Err(evidence_error(
            "SPDX package or file count does not match provenance",
        ));
    }

    let package_entries: Vec<_> = provenance.packages.values().collect();
    for (index, (package, spdx)) in package_entries.iter().zip(&document.packages).enumerate() {
        let package_spdx_id = format!("SPDXRef-Package-{}", index + 1);
        let paths: Vec<&str> = provenance
            .payload_owners
            .iter()
            .filter_map(|(path, owner)| (owner == &package.id).then_some(path.as_str()))
            .collect();
        let expected_verification_code = package_verification_code(backend.root(), &paths)?;
        let expected_download = package.download_url.as_deref().unwrap_or("NOASSERTION");
        let expected_license_comment = format!(
            "Package-manager/source license metadata: {}",
            package.licenses.join(", ")
        );
        if spdx.spdx_id != package_spdx_id
            || spdx.name != package.name
            || spdx.version_info != package.version
            || spdx.download_location != expected_download
            || !spdx.files_analyzed
            || spdx
                .package_verification_code
                .package_verification_code_value
                != expected_verification_code
            || spdx.license_concluded != "NOASSERTION"
            || spdx.license_info_from_files != ["NOASSERTION"]
            || spdx.license_declared != "NOASSERTION"
            || spdx.license_comments != expected_license_comment
            || spdx.copyright_text != "NOASSERTION"
        {
            return Err(evidence_error(format!(
                "SPDX package does not match provenance: {:?}",
                package.id
            )));
        }
    }

    let payload_entries: Vec<_> = provenance
        .payload_order
        .iter()
        .map(|path| {
            (
                path,
                expected_payload
                    .get(path)
                    .expect("validated provenance path must exist in manifest"),
            )
        })
        .collect();
    for (index, ((path, hash), file)) in payload_entries.iter().zip(&document.files).enumerate() {
        if file.spdx_id != format!("SPDXRef-File-{}", index + 1)
            || file.file_name != format!("./{path}")
            || file.checksums.len() != 1
            || file.checksums[0].algorithm != "SHA256"
            || file.checksums[0].checksum_value != **hash
            || file.file_types != ["BINARY"]
            || file.license_concluded != "NOASSERTION"
            || file.license_info_in_files != ["NOASSERTION"]
            || file.copyright_text != "NOASSERTION"
        {
            return Err(evidence_error(format!(
                "SPDX file does not match payload: {path:?}"
            )));
        }
    }

    let mut expected_relationships = BTreeSet::new();
    for (package_index, package) in package_entries.iter().enumerate() {
        let package_spdx_id = format!("SPDXRef-Package-{}", package_index + 1);
        expected_relationships.insert(SpdxRelationship {
            spdx_element_id: "SPDXRef-DOCUMENT".to_owned(),
            relationship_type: "DESCRIBES".to_owned(),
            related_spdx_element: package_spdx_id.clone(),
        });
        for (file_index, (path, _)) in payload_entries.iter().enumerate() {
            if provenance.payload_owners.get(*path) == Some(&package.id) {
                expected_relationships.insert(SpdxRelationship {
                    spdx_element_id: package_spdx_id.clone(),
                    relationship_type: "CONTAINS".to_owned(),
                    related_spdx_element: format!("SPDXRef-File-{}", file_index + 1),
                });
            }
        }
    }
    let actual_relationships: BTreeSet<_> = document
        .relationships
        .iter()
        .map(|relationship| SpdxRelationship {
            spdx_element_id: relationship.spdx_element_id.clone(),
            relationship_type: relationship.relationship_type.clone(),
            related_spdx_element: relationship.related_spdx_element.clone(),
        })
        .collect();
    if actual_relationships.len() != document.relationships.len()
        || actual_relationships != expected_relationships
    {
        return Err(evidence_error(
            "SPDX relationships do not exactly describe package ownership",
        ));
    }
    Ok(())
}

fn validate_inventory(
    root: &Path,
    provenance: &ProvenanceState,
    inventory: &LicenseInventory,
    expected_payload: &BTreeMap<String, String>,
) -> Result<BTreeSet<PathBuf>> {
    if inventory.schema_version != 1
        || inventory.notice.path != NOTICES_FILE
        || !is_sha256(&inventory.notice.sha256)
        || inventory.packages.len() != provenance.packages.len()
    {
        return Err(evidence_error("license inventory metadata is invalid"));
    }
    let notice_hash = sha256_file(&root.join(NOTICES_FILE))?;
    if notice_hash != inventory.notice.sha256 {
        return Err(evidence_error("third-party notice SHA-256 does not match"));
    }

    let mut expected_evidence = BTreeSet::from([
        PathBuf::from(PROVENANCE_FILE),
        PathBuf::from(SPDX_FILE),
        PathBuf::from(LICENSE_INVENTORY_FILE),
        PathBuf::from(NOTICES_FILE),
    ]);
    let package_entries: Vec<_> = provenance.packages.values().collect();
    for (expected, actual) in package_entries.iter().zip(&inventory.packages) {
        if actual.id != expected.id
            || actual.name != expected.name
            || actual.version != expected.version
            || actual.licenses != expected.licenses
        {
            return Err(evidence_error(format!(
                "license inventory package does not match provenance: {:?}",
                expected.id
            )));
        }
        for license_file in &actual.license_files {
            let relative = validate_manifest_path(&license_file.path).map_err(|_| {
                evidence_error(format!(
                    "invalid license evidence path: {:?}",
                    license_file.path
                ))
            })?;
            let package_license_root = Path::new("licenses").join(&actual.id);
            if !relative.starts_with(&package_license_root)
                || relative.components().count() < 3
                || !is_sha256(&license_file.sha256)
            {
                return Err(evidence_error(format!(
                    "invalid or unsorted license evidence: {:?}",
                    license_file.path
                )));
            }
            let actual_hash = sha256_file(&root.join(&relative))?;
            if actual_hash != license_file.sha256 || !expected_evidence.insert(relative) {
                return Err(evidence_error(format!(
                    "license evidence does not match inventory: {:?}",
                    license_file.path
                )));
            }
        }
    }

    let (inventory_payload, inventory_owners) =
        payload_records(&inventory.files, &provenance.packages)?;
    if &inventory_payload != expected_payload || inventory_owners != provenance.payload_owners {
        return Err(evidence_error(
            "license inventory file ownership does not exactly match provenance and manifest",
        ));
    }
    Ok(expected_evidence)
}

fn collect_evidence_tree(root: &Path) -> Result<EvidenceTree> {
    let mut actual = BTreeSet::new();
    let mut actual_directories = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    let mut total_bytes = 0u64;
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).map_err(|error| {
            IrohaZipError::io_path("cannot enumerate backend evidence", &directory, error)
        })? {
            let entry = entry.map_err(|error| {
                IrohaZipError::io_path("cannot enumerate backend evidence entry", &directory, error)
            })?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                IrohaZipError::io_path("cannot inspect backend evidence entry", &path, error)
            })?;
            if metadata.file_type().is_symlink() {
                return Err(evidence_error(format!(
                    "evidence symlinks are forbidden: {}",
                    path.display()
                )));
            }
            platform::validate_extracted_entry_security(&path, &metadata).map_err(|error| {
                evidence_error(format!("unsafe evidence entry {}: {error}", path.display()))
            })?;
            if metadata.is_dir() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| evidence_error("evidence directory escaped its root"))?;
                actual_directories.insert(relative.to_path_buf());
                stack.push(path);
            } else if metadata.is_file() {
                total_bytes = total_bytes
                    .checked_add(metadata.len())
                    .ok_or_else(|| evidence_error("evidence byte count overflow"))?;
                if total_bytes > MAX_EVIDENCE_TOTAL_BYTES {
                    return Err(evidence_error(format!(
                        "evidence exceeds the {MAX_EVIDENCE_TOTAL_BYTES} byte total limit"
                    )));
                }
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| evidence_error("evidence file escaped its root"))?;
                if !actual.insert(relative.to_path_buf()) || actual.len() > MAX_EVIDENCE_FILES {
                    return Err(evidence_error(format!(
                        "evidence exceeds the {MAX_EVIDENCE_FILES} file limit"
                    )));
                }
            } else {
                return Err(evidence_error(format!(
                    "evidence contains a special file: {}",
                    path.display()
                )));
            }
            if actual.len() + actual_directories.len() > MAX_EVIDENCE_ENTRIES {
                return Err(evidence_error(format!(
                    "evidence exceeds the {MAX_EVIDENCE_ENTRIES} entry limit"
                )));
            }
        }
    }
    Ok(EvidenceTree {
        files: actual,
        directories: actual_directories,
    })
}

fn validate_evidence_tree(actual: &EvidenceTree, expected: &BTreeSet<PathBuf>) -> Result<()> {
    let mut expected_directories = BTreeSet::new();
    for path in expected {
        let mut parent = path.parent();
        while let Some(directory) = parent {
            if directory.as_os_str().is_empty() {
                break;
            }
            expected_directories.insert(directory.to_path_buf());
            parent = directory.parent();
        }
    }
    if &actual.files != expected || actual.directories != expected_directories {
        let unexpected: Vec<_> = actual
            .files
            .difference(expected)
            .map(|path| path.display().to_string())
            .collect();
        let missing: Vec<_> = expected
            .difference(&actual.files)
            .map(|path| path.display().to_string())
            .collect();
        let unexpected_directories: Vec<_> = actual
            .directories
            .difference(&expected_directories)
            .map(|path| path.display().to_string())
            .collect();
        let missing_directories: Vec<_> = expected_directories
            .difference(&actual.directories)
            .map(|path| path.display().to_string())
            .collect();
        return Err(evidence_error(format!(
            "evidence tree does not exactly match its inventory; unexpected={unexpected:?}, missing={missing:?}, unexpected_directories={unexpected_directories:?}, missing_directories={missing_directories:?}"
        )));
    }
    Ok(())
}

fn package_verification_code(root: &Path, paths: &[&str]) -> Result<String> {
    let mut file_hashes = Vec::with_capacity(paths.len());
    for path in paths {
        let mut file = File::open(root.join(path)).map_err(|error| {
            IrohaZipError::io_path(
                "cannot open backend file for SPDX SHA-1",
                &root.join(path),
                error,
            )
        })?;
        let mut hasher = Sha1::new();
        let mut buffer = [0u8; 128 * 1024];
        loop {
            let read = file.read(&mut buffer).map_err(|error| {
                IrohaZipError::io_path("cannot hash backend file for SPDX", &root.join(path), error)
            })?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        file_hashes.push(hex_lower(hasher.finalize()));
    }
    file_hashes.sort_unstable();
    let mut package_hasher = Sha1::new();
    for hash in file_hashes {
        package_hasher.update(hash.as_bytes());
    }
    Ok(hex_lower(package_hasher.finalize()))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_bounded_text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn is_utc_second_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || bytes.iter().enumerate().any(|(index, byte)| {
            !matches!(index, 4 | 7 | 10 | 13 | 16 | 19) && !byte.is_ascii_digit()
        })
    {
        return false;
    }
    let two_digits =
        |start: usize| u16::from(bytes[start] - b'0') * 10 + u16::from(bytes[start + 1] - b'0');
    let year = bytes[..4]
        .iter()
        .fold(0u16, |year, digit| year * 10 + u16::from(*digit - b'0'));
    let month = two_digits(5);
    let day = two_digits(8);
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let maximum_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    year != 0
        && (1..=maximum_day).contains(&day)
        && two_digits(11) <= 23
        && two_digits(14) <= 59
        && two_digits(17) <= 60
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use serde_json::json;
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::util;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "iroha-zip-backend-evidence-test-{}",
                util::unique_token()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_json(path: &Path, value: &serde_json::Value) {
        fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    }

    fn fixture(supported: bool) -> (TestDirectory, BackendBundle) {
        let directory = TestDirectory::new();
        let payload = directory.0.join("bsdtar.exe");
        fs::write(&payload, b"test backend").unwrap();
        let payload_hash = sha256_file(&payload).unwrap();
        let manifest = format!(
            "IROHA-ZIP-BACKEND-MANIFEST\t1\nexecutable\tbsdtar.exe\nsha256\t{payload_hash}\tbsdtar.exe\n"
        );
        fs::write(directory.0.join(MANIFEST_FILE), manifest).unwrap();
        let manifest_hash = sha256_file(&directory.0.join(MANIFEST_FILE)).unwrap();
        let evidence = directory.0.join(EVIDENCE_DIRECTORY);
        fs::create_dir(&evidence).unwrap();
        let notices = "# Backend third-party notices\n\nTest package.\n";
        fs::write(evidence.join(NOTICES_FILE), notices).unwrap();
        let notice_hash = sha256_file(&evidence.join(NOTICES_FILE)).unwrap();
        let package = if supported {
            json!({
                "id": "mingw-w64-ucrt-x86_64-libarchive",
                "name": "mingw-w64-ucrt-x86_64-libarchive",
                "version": "3.8.9-1",
                "architecture": "any",
                "repository": "ucrt64",
                "downloadUrl": "https://repo.msys2.org/test.pkg.tar.zst",
                "archiveSha256": "a".repeat(64),
                "signature": "A".repeat(32),
                "licenses": ["BSD"]
            })
        } else {
            json!({
                "id": "unverified-local-bundle",
                "name": "unverified-local-bundle",
                "version": "NOASSERTION",
                "architecture": "windows",
                "repository": "unverified-local",
                "downloadUrl": null,
                "archiveSha256": null,
                "signature": null,
                "licenses": ["NOASSERTION"]
            })
        };
        let package_id = package["id"].as_str().unwrap();
        let license_directory = evidence.join("licenses").join(package_id);
        fs::create_dir_all(&license_directory).unwrap();
        let license_path = license_directory.join("LICENSE.txt");
        fs::write(&license_path, b"test license\n").unwrap();
        let license_hash = sha256_file(&license_path).unwrap();
        let source = if supported {
            json!({
                "kind": "msys2-ucrt64-pacman",
                "supported": true,
                "repository": "ucrt64",
                "verification": {
                    "status": "verified",
                    "method": "pacman-required-trusted-only",
                    "keyringPackage": "msys2-keyring",
                    "keyringVersion": "1~20260214-1"
                }
            })
        } else {
            json!({
                "kind": "unverified-local-bundle",
                "supported": false,
                "repository": "unverified-local",
                "verification": {
                    "status": "unverified",
                    "method": "explicit-user-accepted-local-bundle",
                    "keyringPackage": null,
                    "keyringVersion": null
                }
            })
        };
        let payload_record = json!({
            "path": "bsdtar.exe",
            "sha256": payload_hash,
            "packageId": package_id
        });
        let provenance_path = evidence.join(PROVENANCE_FILE);
        write_json(
            &provenance_path,
            &json!({
                "schemaVersion": 1,
                "createdAtUtc": "2026-08-10T00:00:00Z",
                "source": source,
                "manifest": {"path": MANIFEST_FILE, "sha256": manifest_hash},
                "packages": [package.clone()],
                "files": [payload_record.clone()]
            }),
        );
        let provenance_hash = sha256_file(&provenance_path).unwrap();

        let verification_code = package_verification_code(&directory.0, &["bsdtar.exe"]).unwrap();
        let download_location = package["downloadUrl"].as_str().unwrap_or("NOASSERTION");
        write_json(
            &evidence.join(SPDX_FILE),
            &json!({
                "spdxVersion": "SPDX-2.3",
                "dataLicense": "CC0-1.0",
                "SPDXID": "SPDXRef-DOCUMENT",
                "name": "iroha-zip backend",
                "documentNamespace": format!("https://github.com/hjosugi/iroha-zip/backend-evidence/{manifest_hash}-{provenance_hash}"),
                "creationInfo": {
                    "created": "2026-08-10T00:00:00Z",
                    "creators": ["Tool: iroha-zip-backend-evidence"]
                },
                "packages": [{
                    "SPDXID": "SPDXRef-Package-1",
                    "name": package["name"],
                    "versionInfo": package["version"],
                    "downloadLocation": download_location,
                    "filesAnalyzed": true,
                    "packageVerificationCode": {"packageVerificationCodeValue": verification_code},
                    "licenseConcluded": "NOASSERTION",
                    "licenseInfoFromFiles": ["NOASSERTION"],
                    "licenseDeclared": "NOASSERTION",
                    "licenseComments": format!("Package-manager/source license metadata: {}", package["licenses"][0].as_str().unwrap()),
                    "copyrightText": "NOASSERTION"
                }],
                "files": [{
                    "SPDXID": "SPDXRef-File-1",
                    "fileName": "./bsdtar.exe",
                    "checksums": [{"algorithm": "SHA256", "checksumValue": payload_hash}],
                    "fileTypes": ["BINARY"],
                    "licenseConcluded": "NOASSERTION",
                    "licenseInfoInFiles": ["NOASSERTION"],
                    "copyrightText": "NOASSERTION"
                }],
                "relationships": [
                    {"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": "SPDXRef-Package-1"},
                    {"spdxElementId": "SPDXRef-Package-1", "relationshipType": "CONTAINS", "relatedSpdxElement": "SPDXRef-File-1"}
                ]
            }),
        );
        write_json(
            &evidence.join(LICENSE_INVENTORY_FILE),
            &json!({
                "schemaVersion": 1,
                "notice": {"path": NOTICES_FILE, "sha256": notice_hash},
                "packages": [{
                    "id": package["id"],
                    "name": package["name"],
                    "version": package["version"],
                    "licenses": package["licenses"],
                    "licenseFiles": [{
                        "path": format!("licenses/{package_id}/LICENSE.txt"),
                        "sha256": license_hash
                    }]
                }],
                "files": [payload_record]
            }),
        );

        let backend = BackendBundle::verify(&directory.0).unwrap();
        (directory, backend)
    }

    #[test]
    fn verifies_supported_and_explicitly_unsupported_evidence() {
        for supported in [true, false] {
            let (_directory, backend) = fixture(supported);
            let evidence = BackendEvidence::verify(&backend).unwrap();
            assert_eq!(evidence.is_supported(), supported);
            assert_eq!(evidence.package_count(), 1);
            assert_eq!(evidence.file_count(), 1);
        }
    }

    #[test]
    fn rejects_payload_sbom_inventory_and_evidence_tree_drift() {
        let (directory, backend) = fixture(true);
        let evidence = directory.0.join(EVIDENCE_DIRECTORY);

        fs::write(evidence.join("unexpected.txt"), b"unexpected").unwrap();
        let error = BackendEvidence::verify(&backend).unwrap_err().to_string();
        assert!(error.contains("evidence tree does not exactly match"));
        fs::remove_file(evidence.join("unexpected.txt")).unwrap();

        fs::create_dir(evidence.join("unexpected-empty-directory")).unwrap();
        let error = BackendEvidence::verify(&backend).unwrap_err().to_string();
        assert!(error.contains("unexpected_directories"));
        fs::remove_dir(evidence.join("unexpected-empty-directory")).unwrap();

        let mut sbom: serde_json::Value =
            serde_json::from_slice(&fs::read(evidence.join(SPDX_FILE)).unwrap()).unwrap();
        sbom["files"][0]["checksums"][0]["checksumValue"] = json!("0".repeat(64));
        write_json(&evidence.join(SPDX_FILE), &sbom);
        let error = BackendEvidence::verify(&backend).unwrap_err().to_string();
        assert!(error.contains("SPDX file does not match payload"));
    }

    #[test]
    fn rejects_tampered_notice_and_missing_provenance() {
        let (directory, backend) = fixture(false);
        let evidence = directory.0.join(EVIDENCE_DIRECTORY);
        fs::write(evidence.join(NOTICES_FILE), b"tampered").unwrap();
        let error = BackendEvidence::verify(&backend).unwrap_err().to_string();
        assert!(error.contains("notice SHA-256 does not match"));
        let error = BackendBundle::verify(&directory.0).unwrap_err().to_string();
        assert!(error.contains("notice SHA-256 does not match"));

        let (directory, backend) = fixture(false);
        let license = directory
            .0
            .join(EVIDENCE_DIRECTORY)
            .join("licenses/unverified-local-bundle/LICENSE.txt");
        fs::write(license, b"tampered license").unwrap();
        let error = BackendEvidence::verify(&backend).unwrap_err().to_string();
        assert!(error.contains("license evidence does not match inventory"));

        let (directory, backend) = fixture(false);
        fs::remove_file(directory.0.join(EVIDENCE_DIRECTORY).join(PROVENANCE_FILE)).unwrap();
        let error = BackendEvidence::verify(&backend).unwrap_err().to_string();
        assert!(error.contains("cannot inspect backend evidence document"));
    }

    #[test]
    fn package_verification_code_matches_spdx_algorithm() {
        let directory = TestDirectory::new();
        fs::write(directory.0.join("a"), b"a").unwrap();
        fs::write(directory.0.join("b"), b"b").unwrap();
        let code = package_verification_code(&directory.0, &["a", "b"]).unwrap();

        let mut hashes = [hex_lower(Sha1::digest(b"a")), hex_lower(Sha1::digest(b"b"))];
        hashes.sort_unstable();
        let expected = hex_lower(Sha1::digest(
            format!("{}{}", hashes[0], hashes[1]).as_bytes(),
        ));
        assert_eq!(code, expected);

        let lowercase_sha256 = hex_lower(Sha256::digest(b"test"));
        assert!(is_sha256(&lowercase_sha256));
        assert!(!is_sha256(&lowercase_sha256.to_uppercase()));
        assert!(is_utc_second_timestamp("2024-02-29T23:59:60Z"));
        assert!(!is_utc_second_timestamp("2023-02-29T00:00:00Z"));
        assert!(!is_utc_second_timestamp("2026-04-31T00:00:00Z"));
        assert!(!is_utc_second_timestamp("0000-01-01T00:00:00Z"));
    }
}
