use clap::{CommandFactory, Parser};
use iroha_zip::cli::{Cli, Command, FilenameEncoding};

#[test]
fn command_name_matches_the_application_name() {
    assert_eq!(Cli::command().get_name(), "iroha-zip");
}

#[test]
fn extraction_uses_configuration_encoding_when_not_overridden() {
    let cli = Cli::try_parse_from(["iroha-zip", "extract", "archive.zip"]).unwrap();
    let Command::Extract { encoding, .. } = cli.command else {
        panic!("expected extract command");
    };
    assert_eq!(encoding, None);
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
