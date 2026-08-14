use sha2::{Digest, Sha256};

use iroha_zip::util::hex_lower;

const BLOCK: usize = 512;

#[derive(Clone)]
struct ZipEntry {
    name: &'static str,
    data: Vec<u8>,
    unix_mode: u32,
}

impl ZipEntry {
    fn file(name: &'static str, data: impl Into<Vec<u8>>) -> Self {
        Self {
            name,
            data: data.into(),
            unix_mode: 0o100_644,
        }
    }

    fn symlink(name: &'static str, target: &'static str) -> Self {
        Self {
            name,
            data: target.as_bytes().to_vec(),
            unix_mode: 0o120_777,
        }
    }
}

fn zip(entries: &[ZipEntry]) -> Vec<u8> {
    let mut archive = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        let name = entry.name.as_bytes();
        let name_length = u16::try_from(name.len()).unwrap();
        let data_length = u32::try_from(entry.data.len()).unwrap();
        let offset = u32::try_from(archive.len()).unwrap();
        let checksum = crc32(&entry.data);

        push_u32(&mut archive, 0x0403_4B50);
        push_u16(&mut archive, 20);
        push_u16(&mut archive, 0x0800);
        push_u16(&mut archive, 0);
        push_u16(&mut archive, 0);
        push_u16(&mut archive, 0x0021);
        push_u32(&mut archive, checksum);
        push_u32(&mut archive, data_length);
        push_u32(&mut archive, data_length);
        push_u16(&mut archive, name_length);
        push_u16(&mut archive, 0);
        archive.extend_from_slice(name);
        archive.extend_from_slice(&entry.data);

        push_u32(&mut central, 0x0201_4B50);
        push_u16(&mut central, 0x0314);
        push_u16(&mut central, 20);
        push_u16(&mut central, 0x0800);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0x0021);
        push_u32(&mut central, checksum);
        push_u32(&mut central, data_length);
        push_u32(&mut central, data_length);
        push_u16(&mut central, name_length);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u32(&mut central, entry.unix_mode << 16);
        push_u32(&mut central, offset);
        central.extend_from_slice(name);
    }
    let central_offset = u32::try_from(archive.len()).unwrap();
    let central_size = u32::try_from(central.len()).unwrap();
    archive.extend_from_slice(&central);
    push_u32(&mut archive, 0x0605_4B50);
    push_u16(&mut archive, 0);
    push_u16(&mut archive, 0);
    let count = u16::try_from(entries.len()).unwrap();
    push_u16(&mut archive, count);
    push_u16(&mut archive, count);
    push_u32(&mut archive, central_size);
    push_u32(&mut archive, central_offset);
    push_u16(&mut archive, 0);
    archive
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn crc32(input: &[u8]) -> u32 {
    let mut value = !0u32;
    for byte in input {
        value ^= u32::from(*byte);
        for _ in 0..8 {
            value = (value >> 1) ^ (0xEDB8_8320 & 0u32.wrapping_sub(value & 1));
        }
    }
    !value
}

#[derive(Clone)]
struct TarEntry {
    name: String,
    data: Vec<u8>,
    kind: u8,
    link_name: String,
}

impl TarEntry {
    fn file(name: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
            kind: b'0',
            link_name: String::new(),
        }
    }

    fn symlink(name: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: Vec::new(),
            kind: b'2',
            link_name: target.into(),
        }
    }

    fn hardlink(name: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: Vec::new(),
            kind: b'1',
            link_name: target.into(),
        }
    }
}

fn tar(entries: &[TarEntry]) -> Vec<u8> {
    let mut archive = Vec::new();
    for entry in entries {
        let mut header = [0u8; BLOCK];
        write_ustar_name(&mut header, &entry.name);
        write_octal(&mut header[100..108], 0o644);
        write_octal(&mut header[108..116], 0);
        write_octal(&mut header[116..124], 0);
        let stored_size = if entry.kind == b'0' {
            u64::try_from(entry.data.len()).unwrap()
        } else {
            0
        };
        write_octal(&mut header[124..136], stored_size);
        write_octal(&mut header[136..148], 0);
        header[148..156].fill(b' ');
        header[156] = entry.kind;
        write_bytes(&mut header[157..257], entry.link_name.as_bytes());
        write_bytes(&mut header[257..263], b"ustar\0");
        write_bytes(&mut header[263..265], b"00");
        write_bytes(&mut header[265..297], b"iroha-zip");
        write_bytes(&mut header[297..329], b"iroha-zip");
        finish_tar_checksum(&mut header);
        archive.extend_from_slice(&header);
        if entry.kind == b'0' {
            archive.extend_from_slice(&entry.data);
            pad_tar_data(&mut archive);
        }
    }
    archive.resize(archive.len() + 2 * BLOCK, 0);
    archive
}

fn oldgnu_sparse_tar(name: &str, logical_size: u64, data_offset: u64, data: &[u8]) -> Vec<u8> {
    assert!(data_offset + u64::try_from(data.len()).unwrap() <= logical_size);
    let mut header = [0u8; BLOCK];
    write_bytes(&mut header[..100], name.as_bytes());
    write_octal(&mut header[100..108], 0o644);
    write_octal(&mut header[108..116], 0);
    write_octal(&mut header[116..124], 0);
    write_octal(&mut header[124..136], u64::try_from(data.len()).unwrap());
    write_octal(&mut header[136..148], 0);
    header[148..156].fill(b' ');
    header[156] = b'S';
    write_bytes(&mut header[257..265], b"ustar  \0");
    write_bytes(&mut header[265..297], b"iroha-zip");
    write_bytes(&mut header[297..329], b"iroha-zip");
    write_octal(&mut header[386..398], data_offset);
    write_octal(&mut header[398..410], u64::try_from(data.len()).unwrap());
    header[482] = 0;
    write_octal(&mut header[483..495], logical_size);
    finish_tar_checksum(&mut header);

    let mut archive = header.to_vec();
    archive.extend_from_slice(data);
    pad_tar_data(&mut archive);
    archive.resize(archive.len() + 2 * BLOCK, 0);
    archive
}

fn write_ustar_name(header: &mut [u8; BLOCK], name: &str) {
    let bytes = name.as_bytes();
    if bytes.len() <= 100 {
        write_bytes(&mut header[..100], bytes);
        return;
    }
    let split = bytes
        .iter()
        .enumerate()
        .filter(|(index, byte)| **byte == b'/' && *index <= 155 && bytes.len() - index - 1 <= 100)
        .map(|(index, _)| index)
        .next_back()
        .expect("test tar name must fit the ustar prefix/name fields");
    write_bytes(&mut header[..100], &bytes[split + 1..]);
    write_bytes(&mut header[345..500], &bytes[..split]);
}

fn write_bytes(field: &mut [u8], value: &[u8]) {
    assert!(value.len() <= field.len());
    field[..value.len()].copy_from_slice(value);
}

fn write_octal(field: &mut [u8], value: u64) {
    let digits_len = field.len() - 1;
    let digits = format!("{value:0digits_len$o}");
    assert_eq!(digits.len(), digits_len);
    field[..digits_len].copy_from_slice(digits.as_bytes());
    field[digits_len] = 0;
}

fn finish_tar_checksum(header: &mut [u8; BLOCK]) {
    header[148..156].fill(b' ');
    let checksum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    let digits = format!("{checksum:06o}");
    assert_eq!(digits.len(), 6);
    header[148..154].copy_from_slice(digits.as_bytes());
    header[154] = 0;
    header[155] = b' ';
}

fn pad_tar_data(archive: &mut Vec<u8>) {
    let remainder = archive.len() % BLOCK;
    if remainder != 0 {
        archive.resize(archive.len() + BLOCK - remainder, 0);
    }
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    hex_lower(digest.finalize())
}

struct CorpusSample {
    id: &'static str,
    threat: &'static str,
    extension: &'static str,
    bytes: Vec<u8>,
    expected: Expected,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Expected {
    AcceptControl,
    RejectBeforePublication,
}

fn corpus() -> Vec<CorpusSample> {
    let deep_path = format!(
        "{}/leaf.txt",
        (0..9).map(|_| "deep").collect::<Vec<_>>().join("/")
    );
    let count_entries = (0..9)
        .map(|index| TarEntry::file(format!("count/{index:02}.txt"), b"x".to_vec()))
        .collect::<Vec<_>>();
    vec![
        CorpusSample {
            id: "control-zip",
            threat: "benign harness control",
            extension: "zip",
            bytes: zip(&[ZipEntry::file("ok.txt", b"control".to_vec())]),
            expected: Expected::AcceptControl,
        },
        CorpusSample {
            id: "zip-parent-traversal",
            threat: "parent traversal",
            extension: "zip",
            bytes: zip(&[ZipEntry::file("../escape.txt", b"escape".to_vec())]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "zip-absolute-path",
            threat: "absolute path",
            extension: "zip",
            bytes: zip(&[ZipEntry::file("/absolute.txt", b"absolute".to_vec())]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "zip-drive-path",
            threat: "Windows drive prefix",
            extension: "zip",
            bytes: zip(&[ZipEntry::file("C:/drive.txt", b"drive".to_vec())]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "zip-unc-path",
            threat: "UNC path",
            extension: "zip",
            bytes: zip(&[ZipEntry::file("//server/share/unc.txt", b"unc".to_vec())]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "zip-ads-name",
            threat: "alternate data stream name",
            extension: "zip",
            bytes: zip(&[ZipEntry::file("safe.txt:payload", b"ads".to_vec())]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "zip-device-name",
            threat: "Windows device name",
            extension: "zip",
            bytes: zip(&[ZipEntry::file("CON.txt", b"device".to_vec())]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "zip-trailing-dot-alias",
            threat: "trailing-dot alias",
            extension: "zip",
            bytes: zip(&[ZipEntry::file("alias.", b"alias".to_vec())]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "zip-trailing-space-alias",
            threat: "trailing-space alias",
            extension: "zip",
            bytes: zip(&[ZipEntry::file("alias ", b"alias".to_vec())]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "zip-invalid-character",
            threat: "Windows-invalid path character",
            extension: "zip",
            bytes: zip(&[ZipEntry::file("bad?.txt", b"invalid".to_vec())]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "zip-duplicate-name",
            threat: "duplicate path",
            extension: "zip",
            bytes: zip(&[
                ZipEntry::file("duplicate.txt", b"first".to_vec()),
                ZipEntry::file("duplicate.txt", b"second".to_vec()),
            ]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "zip-symlink",
            threat: "symbolic-link/reparse entry",
            extension: "zip",
            bytes: zip(&[
                ZipEntry::file("target.txt", b"target".to_vec()),
                ZipEntry::symlink("link.txt", "target.txt"),
            ]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "tar-symlink",
            threat: "symbolic-link/reparse entry",
            extension: "tar",
            bytes: tar(&[
                TarEntry::file("target.txt", b"target".to_vec()),
                TarEntry::symlink("link.txt", "target.txt"),
            ]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "tar-hardlink",
            threat: "hardlink/duplicate file identity",
            extension: "tar",
            bytes: tar(&[
                TarEntry::file("target.txt", b"target".to_vec()),
                TarEntry::hardlink("hardlink.txt", "target.txt"),
            ]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "tar-depth-limit",
            threat: "path-depth bomb",
            extension: "tar",
            bytes: tar(&[TarEntry::file(deep_path, b"deep".to_vec())]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "tar-file-count-limit",
            threat: "file-count bomb",
            extension: "tar",
            bytes: tar(&count_entries),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "tar-single-file-limit",
            threat: "single-file expansion bomb",
            extension: "tar",
            bytes: tar(&[TarEntry::file("large.bin", vec![0x41; 131_073])]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "tar-total-size-limit",
            threat: "total expansion bomb",
            extension: "tar",
            bytes: tar(&[
                TarEntry::file("one.bin", vec![0x31; 90_000]),
                TarEntry::file("two.bin", vec![0x32; 90_000]),
            ]),
            expected: Expected::RejectBeforePublication,
        },
        CorpusSample {
            id: "tar-sparse-expansion",
            threat: "sparse logical-size expansion bomb",
            extension: "tar",
            bytes: oldgnu_sparse_tar("sparse.bin", 2 * 1024 * 1024, 2 * 1024 * 1024 - 1, &[0x5A]),
            expected: Expected::RejectBeforePublication,
        },
    ]
}

#[test]
fn generated_corpus_is_unique_bounded_and_structurally_tagged() {
    let samples = corpus();
    assert_eq!(samples.len(), 19);
    let mut ids = samples.iter().map(|sample| sample.id).collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), samples.len());
    for sample in samples {
        assert!(sample.bytes.len() <= 4 * 1024 * 1024, "{}", sample.id);
        assert!(!sample.threat.is_empty());
        assert!(matches!(
            sample.expected,
            Expected::AcceptControl | Expected::RejectBeforePublication
        ));
        assert_eq!(sha256(&sample.bytes).len(), 64);
        match sample.extension {
            "zip" => assert_eq!(&sample.bytes[..4], b"PK\x03\x04"),
            "tar" => assert_eq!(sample.bytes.len() % BLOCK, 0),
            extension => panic!("unexpected corpus extension: {extension}"),
        }
    }

    if let Some(output) = std::env::var_os("IROHA_ZIP_CORPUS_MATERIALIZE_DIR") {
        let output = std::path::PathBuf::from(output);
        std::fs::create_dir(&output)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", output.display()));
        for sample in corpus() {
            let path = output.join(format!("{}.{}", sample.id, sample.extension));
            std::fs::write(&path, sample.bytes)
                .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
        }
    }
}

#[cfg(windows)]
mod windows_e2e {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use iroha_zip::config::Config;
    use iroha_zip::policy::{self, Limits};
    use serde::Serialize;

    use super::*;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct CorpusReport {
        schema_version: u32,
        status: String,
        executable_sha256: String,
        backend_manifest_sha256: String,
        samples: Vec<SampleReport>,
        policy_fixtures: Vec<PolicyFixtureReport>,
        temporary_root_removed: bool,
        failure: Option<String>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SampleReport {
        id: String,
        threat: String,
        archive_sha256: String,
        archive_bytes: u64,
        expected: String,
        exit_code: i32,
        rejection_class: Option<String>,
        destination_published: bool,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PolicyFixtureReport {
        id: String,
        rejected: bool,
        error_class: String,
    }

    #[test]
    #[ignore = "requires a disposable Windows worker, release executable, and verified backend"]
    fn every_generated_attack_is_rejected_before_publication() {
        let executable = required_path("IROHA_ZIP_CORPUS_EXECUTABLE");
        let backend = required_path("IROHA_ZIP_CORPUS_BACKEND");
        let evidence = required_path("IROHA_ZIP_CORPUS_EVIDENCE");
        let root = env::temp_dir().join(format!(
            "iroha-zip-corpus-日本語-{}",
            iroha_zip::util::unique_token()
        ));
        fs::create_dir(&root).unwrap();

        let mut report = CorpusReport {
            schema_version: 1,
            status: "running".to_owned(),
            executable_sha256: file_sha256(&executable),
            backend_manifest_sha256: file_sha256(&backend.join("backend-manifest.tsv")),
            samples: Vec::new(),
            policy_fixtures: Vec::new(),
            temporary_root_removed: false,
            failure: None,
        };
        let result = run_corpus(&root, &executable, &backend, &mut report);
        let cleanup = fs::remove_dir_all(&root)
            .map_err(|error| format!("cannot remove corpus root {}: {error}", root.display()));
        report.temporary_root_removed = !root.exists();
        let failure = match (result, cleanup, report.temporary_root_removed) {
            (Ok(()), Ok(()), true) => None,
            (operation, cleanup, removed) => Some(format!(
                "operation={:?}; cleanup={:?}; temporaryRootRemoved={removed}",
                operation.err(),
                cleanup.err()
            )),
        };
        report.status = if failure.is_none() {
            "passed"
        } else {
            "failed"
        }
        .to_owned();
        report.failure.clone_from(&failure);
        if let Some(parent) = evidence.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            &evidence,
            format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
        )
        .unwrap();
        assert!(failure.is_none(), "{}", failure.unwrap_or_default());
    }

    fn run_corpus(
        root: &Path,
        executable: &Path,
        backend: &Path,
        report: &mut CorpusReport,
    ) -> Result<(), String> {
        let archive_root = root.join("archives");
        let destination_root = root.join("destinations");
        fs::create_dir(&archive_root).map_err(|error| error.to_string())?;
        fs::create_dir(&destination_root).map_err(|error| error.to_string())?;

        let config_path = root.join("config.toml");
        let mut config = Config::default();
        config.backend.directory = Some(backend.to_path_buf());
        config.behavior.preserve_mark_of_the_web = false;
        config.behavior.open_after_double_click = false;
        config.limits = Limits {
            max_archive_bytes: 4 * 1024 * 1024,
            max_files: 8,
            max_directories: 8,
            max_total_bytes: 163_840,
            max_single_file_bytes: 131_072,
            max_depth: 8,
            max_path_bytes: 512,
        };
        config
            .save(&config_path)
            .map_err(|error| error.to_string())?;

        for sample in corpus() {
            let archive = archive_root.join(format!("{}.{}", sample.id, sample.extension));
            let destination = destination_root.join(sample.id);
            fs::write(&archive, &sample.bytes).map_err(|error| error.to_string())?;
            let output = Command::new(executable)
                .arg("--config")
                .arg(&config_path)
                .arg("extract")
                .arg(&archive)
                .arg("--output")
                .arg(&destination)
                .output()
                .map_err(|error| format!("cannot run corpus sample {}: {error}", sample.id))?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            let rejection_class = classify_rejection(&stderr);
            let published = destination.exists();
            let exit_code = output.status.code().unwrap_or(-1);
            report.samples.push(SampleReport {
                id: sample.id.to_owned(),
                threat: sample.threat.to_owned(),
                archive_sha256: sha256(&sample.bytes),
                archive_bytes: u64::try_from(sample.bytes.len()).unwrap(),
                expected: match sample.expected {
                    Expected::AcceptControl => "accept-control",
                    Expected::RejectBeforePublication => "reject-before-publication",
                }
                .to_owned(),
                exit_code,
                rejection_class: rejection_class.map(str::to_owned),
                destination_published: published,
            });

            match sample.expected {
                Expected::AcceptControl => {
                    if !output.status.success()
                        || fs::read(destination.join("ok.txt")).ok().as_deref()
                            != Some(b"control".as_slice())
                    {
                        return Err(format!(
                            "control archive did not extract correctly: stderr={stderr:?}"
                        ));
                    }
                }
                Expected::RejectBeforePublication => {
                    if output.status.success() || published {
                        return Err(format!(
                            "{} was not rejected before publication: status={} published={published}",
                            sample.id, output.status
                        ));
                    }
                    if !matches!(rejection_class, Some("backend" | "policy")) {
                        return Err(format!(
                            "{} had an unexpected rejection class: stderr={stderr:?}",
                            sample.id
                        ));
                    }
                }
            }
        }
        run_policy_fixtures(root, report)
    }

    fn run_policy_fixtures(root: &Path, report: &mut CorpusReport) -> Result<(), String> {
        let limits = Limits::default();

        let hardlink_root = root.join("policy-hardlink");
        fs::create_dir(&hardlink_root).map_err(|error| error.to_string())?;
        fs::write(hardlink_root.join("one.txt"), b"same").map_err(|error| error.to_string())?;
        fs::hard_link(hardlink_root.join("one.txt"), hardlink_root.join("two.txt"))
            .map_err(|error| error.to_string())?;
        require_policy_rejection("duplicate-file-identity", &hardlink_root, &limits, report)?;

        let ads_root = root.join("policy-ads");
        fs::create_dir(&ads_root).map_err(|error| error.to_string())?;
        let ads_file = ads_root.join("plain.txt");
        fs::write(&ads_file, b"primary").map_err(|error| error.to_string())?;
        fs::write(format!("{}:payload", ads_file.display()), b"stream")
            .map_err(|error| error.to_string())?;
        require_policy_rejection("ntfs-alternate-data-stream", &ads_root, &limits, report)?;

        let junction_root = root.join("policy-junction");
        fs::create_dir(&junction_root).map_err(|error| error.to_string())?;
        let junction_target = junction_root.join("target");
        fs::create_dir(&junction_target).map_err(|error| error.to_string())?;
        let status = Command::new("cmd.exe")
            .current_dir(&junction_root)
            .args(["/d", "/s", "/c", "mklink /J mounted target"])
            .status()
            .map_err(|error| format!("cannot create junction fixture: {error}"))?;
        if !status.success() {
            return Err(format!("junction fixture creation failed with {status}"));
        }
        require_policy_rejection(
            "ntfs-junction-reparse-point",
            &junction_root,
            &limits,
            report,
        )
    }

    fn require_policy_rejection(
        id: &str,
        root: &Path,
        limits: &Limits,
        report: &mut CorpusReport,
    ) -> Result<(), String> {
        let result = policy::audit_tree(root, limits);
        let rejected = result.is_err();
        let error_class = match result {
            Err(iroha_zip::error::IrohaZipError::Policy(_)) => "policy",
            Err(error) => return Err(format!("{id} returned the wrong error class: {error}")),
            Ok(_) => "accepted",
        };
        report.policy_fixtures.push(PolicyFixtureReport {
            id: id.to_owned(),
            rejected,
            error_class: error_class.to_owned(),
        });
        if rejected {
            Ok(())
        } else {
            Err(format!("{id} was unexpectedly accepted"))
        }
    }

    fn classify_rejection(stderr: &str) -> Option<&'static str> {
        if stderr.contains("archive rejected:") {
            Some("policy")
        } else if stderr.contains("backend error:") {
            Some("backend")
        } else if stderr.contains("sandbox error:") {
            Some("sandbox")
        } else if stderr.contains("configuration error:") {
            Some("config")
        } else if stderr.contains("usage error:") {
            Some("usage")
        } else {
            None
        }
    }

    fn required_path(name: &str) -> PathBuf {
        env::var_os(name).map_or_else(
            || panic!("{name} must be set for the ignored Windows corpus test"),
            PathBuf::from,
        )
    }

    fn file_sha256(path: &Path) -> String {
        sha256(&fs::read(path).unwrap())
    }
}
