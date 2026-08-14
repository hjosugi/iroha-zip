use clap::{CommandFactory, Parser};
use iroha_zip::cli::{Cli, Command, FilenameEncoding};

#[test]
fn command_name_matches_the_application_name() {
    assert_eq!(Cli::command().get_name(), "iroha-zip");
}

#[test]
fn extraction_uses_configuration_encoding_when_not_overridden() {
    let cli = Cli::try_parse_from(["iroha-zip", "extract", "archive.zip"]).unwrap();
    let Command::Extract {
        encoding, select, ..
    } = cli.command
    else {
        panic!("expected extract command");
    };
    assert_eq!(encoding, None);
    assert!(select.is_empty());
}

#[test]
fn extraction_accepts_repeated_preview_relative_selections() {
    let cli = Cli::try_parse_from([
        "iroha-zip",
        "extract",
        "archive.zip",
        "--select",
        "資料/readme.txt",
        "--select",
        "写真",
    ])
    .unwrap();
    let Command::Extract { select, .. } = cli.command else {
        panic!("expected extract command");
    };
    assert_eq!(
        select,
        [
            std::path::PathBuf::from("資料/readme.txt"),
            std::path::PathBuf::from("写真")
        ]
    );
}

#[test]
fn extraction_accepts_an_explicit_encoding_override() {
    let cli = Cli::try_parse_from(["iroha-zip", "extract", "archive.zip", "--encoding", "cp932"])
        .unwrap();
    let Command::Extract { encoding, .. } = cli.command else {
        panic!("expected extract command");
    };
    assert_eq!(encoding, Some(FilenameEncoding::Cp932));
}

#[test]
fn settings_subcommand_is_available() {
    let cli = Cli::try_parse_from(["iroha-zip", "settings"]).unwrap();
    assert!(matches!(cli.command, Command::Settings));
}

#[test]
fn backend_evidence_validation_can_require_a_supported_source() {
    let cli = Cli::try_parse_from([
        "iroha-zip",
        "verify-backend-evidence",
        "backend/libarchive",
        "--require-supported",
    ])
    .unwrap();
    let Command::VerifyBackendEvidence {
        backend,
        require_supported,
    } = cli.command
    else {
        panic!("expected backend evidence command");
    };
    assert_eq!(backend, std::path::Path::new("backend/libarchive"));
    assert!(require_supported);
}

#[test]
fn isolation_report_is_available_for_machine_readable_windows_evidence() {
    let cli = Cli::try_parse_from(["iroha-zip", "isolation-report"]).unwrap();
    assert!(matches!(cli.command, Command::IsolationReport));
}

#[test]
fn internal_failure_probes_are_hidden_but_parseable() {
    let sleep = Cli::try_parse_from(["iroha-zip", "internal-sleep-probe", "5000"]).unwrap();
    assert!(matches!(
        sleep.command,
        Command::InternalSleepProbe {
            milliseconds: 5_000
        }
    ));

    let memory = Cli::try_parse_from(["iroha-zip", "internal-memory-probe", "268435456"]).unwrap();
    assert!(matches!(
        memory.command,
        Command::InternalMemoryProbe { bytes: 268_435_456 }
    ));

    let staging =
        Cli::try_parse_from(["iroha-zip", "internal-staging-write-probe", "source"]).unwrap();
    assert!(matches!(
        staging.command,
        Command::InternalStagingWriteProbe { root } if root == std::path::Path::new("source")
    ));

    let listing = Cli::try_parse_from([
        "iroha-zip",
        "internal-archive-listing",
        "backend",
        "candidates.txt",
        "archive.bin",
        "--encoding",
        "utf8",
        "--max-entries",
        "125001",
        "--max-path-bytes",
        "4096",
    ])
    .unwrap();
    assert!(matches!(
        listing.command,
        Command::InternalArchiveListing {
            encoding: FilenameEncoding::Utf8,
            max_entries: 125_001,
            max_path_bytes: 4_096,
            allow_unsandboxed: false,
            ..
        }
    ));
}

#[test]
fn preview_uses_the_same_encoding_and_isolation_controls_as_extract() {
    let cli = Cli::try_parse_from([
        "iroha-zip",
        "preview",
        "archive.lzh",
        "--encoding",
        "cp932",
        "--allow-unsandboxed",
    ])
    .unwrap();
    let Command::Preview {
        archive,
        encoding,
        allow_unsandboxed,
    } = cli.command
    else {
        panic!("expected preview command");
    };
    assert_eq!(archive, std::path::Path::new("archive.lzh"));
    assert_eq!(encoding, Some(FilenameEncoding::Cp932));
    assert!(allow_unsandboxed);
}
