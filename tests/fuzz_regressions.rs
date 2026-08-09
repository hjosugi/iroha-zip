#![cfg(feature = "fuzzing")]

use std::fs;
use std::path::Path;

type Harness = fn(&[u8]);

#[test]
fn minimized_fuzz_regressions_remain_fixed() {
    for (target, harness) in [
        (
            "backend_manifest",
            iroha_zip::fuzzing::backend_manifest as Harness,
        ),
        ("windows_paths", iroha_zip::fuzzing::windows_paths),
        ("archive_name", iroha_zip::fuzzing::archive_name),
        ("command_line", iroha_zip::fuzzing::command_line),
        ("config_round_trip", iroha_zip::fuzzing::config_round_trip),
    ] {
        run_target_regressions(target, harness);
    }
}

fn run_target_regressions(target: &str, harness: Harness) {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fuzz")
        .join("regressions")
        .join(target);
    if !directory.exists() {
        return;
    }

    let mut paths: Vec<_> = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
        .map(|entry| entry.expect("cannot read regression entry").path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| !name.to_string_lossy().starts_with('.'))
        })
        .collect();
    paths.sort();

    for path in paths {
        let metadata = fs::symlink_metadata(&path)
            .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", path.display()));
        assert!(
            metadata.file_type().is_file(),
            "regression entry must be a regular file: {}",
            path.display()
        );
        let input = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        assert!(
            input.len() <= 65_536,
            "regression input exceeds the 65,536-byte fuzz limit: {}",
            path.display()
        );
        let digest = iroha_zip::backend::sha256_file(&path)
            .unwrap_or_else(|error| panic!("cannot hash {}: {error}", path.display()));
        let expected_name = format!("{digest}.bin");
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some(expected_name.as_str()),
            "regression filename must match its SHA-256: {}",
            path.display()
        );
        harness(&input);
    }
}
