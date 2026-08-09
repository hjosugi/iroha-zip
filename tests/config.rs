use std::fs;

use iroha_zip::config::{Config, FilenameEncoding, IsolationMode};
use iroha_zip::util;

#[test]
fn default_configuration_uses_the_application_directory() {
    let path = iroha_zip::config::default_config_path().unwrap();
    assert_eq!(path.file_name().unwrap(), "config.toml");
    assert_eq!(path.parent().unwrap().file_name().unwrap(), "iroha-zip");
}

#[test]
fn default_config_round_trips_through_toml() {
    let original = Config::default();
    let encoded = toml::to_string_pretty(&original).unwrap();
    let decoded: Config = toml::from_str(&encoded).unwrap();

    assert_eq!(
        decoded.sandbox.timeout_seconds,
        original.sandbox.timeout_seconds
    );
    assert_eq!(
        decoded.sandbox.memory_limit_mib,
        original.sandbox.memory_limit_mib
    );
    assert_eq!(decoded.sandbox.isolation, original.sandbox.isolation);
    assert_eq!(decoded.limits.max_files, original.limits.max_files);
    assert_eq!(
        decoded.behavior.preserve_mark_of_the_web,
        original.behavior.preserve_mark_of_the_web
    );
}

#[test]
fn documented_example_is_complete_and_valid() {
    let config: Config = toml::from_str(include_str!("../config.example.toml")).unwrap();
    config.validate().unwrap();

    assert_eq!(
        config.backend.directory.as_deref(),
        Some(std::path::Path::new("backend/libarchive"))
    );
    assert_eq!(
        config.behavior.default_filename_encoding,
        FilenameEncoding::Auto
    );
    assert_eq!(config.sandbox.isolation, IsolationMode::AppContainer);
}

#[test]
fn unknown_configuration_fields_are_rejected() {
    let text = r"
[backend]
unknown_backend_switch = true
";
    assert!(toml::from_str::<Config>(text).is_err());
}

#[test]
fn missing_new_fields_use_safe_defaults() {
    let text = r"
[behavior]
preserve_mark_of_the_web = true
open_after_double_click = false
";
    let config: Config = toml::from_str(text).unwrap();
    assert_eq!(
        config.behavior.default_filename_encoding,
        FilenameEncoding::Auto
    );
    assert_eq!(config.sandbox.isolation, IsolationMode::AppContainer);
}

#[test]
fn lpac_mode_is_explicit_and_unknown_modes_are_rejected() {
    let lpac: Config = toml::from_str(
        r#"
[sandbox]
isolation = "lpac"
"#,
    )
    .unwrap();
    assert_eq!(lpac.sandbox.isolation, IsolationMode::Lpac);

    let invalid = r#"
[sandbox]
isolation = "automatic"
"#;
    assert!(toml::from_str::<Config>(invalid).is_err());
}

#[test]
fn validation_rejects_unsafe_or_inconsistent_limits() {
    let mut config = Config::default();
    config.sandbox.memory_limit_mib = 63;
    assert!(config.validate().is_err());

    config = Config::default();
    config.sandbox.timeout_seconds = 0;
    assert!(config.validate().is_err());

    config = Config::default();
    config.limits.max_single_file_bytes = config.limits.max_total_bytes + 1;
    assert!(config.validate().is_err());

    config = Config::default();
    config.limits.max_files = 0;
    assert!(config.validate().is_err());
}

#[test]
fn save_replaces_configuration_and_preserves_all_settings() {
    let directory = std::env::temp_dir().join(format!("iroha-zip-config-{}", util::unique_token()));
    let path = directory.join("config.toml");
    let mut config = Config::default();
    config.backend.directory = Some("custom/backend".into());
    config.sandbox.timeout_seconds = 42;
    config.sandbox.memory_limit_mib = 1_024;
    config.sandbox.isolation = IsolationMode::Lpac;
    config.limits.max_files = 123_456;
    config.behavior.open_after_double_click = false;
    config.behavior.default_filename_encoding = FilenameEncoding::Cp932;

    config.save(&path).unwrap();
    assert_eq!(Config::load(&path).unwrap(), config);

    config.sandbox.timeout_seconds = 43;
    config.save(&path).unwrap();
    assert_eq!(Config::load(&path).unwrap(), config);
    assert!(
        fs::read_dir(&directory)
            .unwrap()
            .all(|entry| entry.unwrap().file_name() == "config.toml")
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn invalid_configuration_is_not_written() {
    let directory = std::env::temp_dir().join(format!("iroha-zip-config-{}", util::unique_token()));
    let path = directory.join("config.toml");
    let valid = Config::default();
    valid.save(&path).unwrap();
    let original = fs::read_to_string(&path).unwrap();

    let mut invalid = valid;
    invalid.limits.max_total_bytes = 0;
    assert!(invalid.save(&path).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);

    fs::remove_dir_all(directory).unwrap();
}
