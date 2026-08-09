use iroha_zip::config::{Config, FilenameEncoding, IsolationMode};
use iroha_zip::settings::{SettingsField, SettingsForm, format_byte_count};

#[test]
fn form_round_trip_preserves_every_configuration_field() {
    let mut config = Config::default();
    config.backend.directory = Some("C:/iroha-zip/backend".into());
    config.sandbox.timeout_seconds = 42;
    config.sandbox.memory_limit_mib = 2_048;
    config.sandbox.isolation = IsolationMode::Lpac;
    config.limits.max_archive_bytes = 16 * 1024_u64.pow(3);
    config.limits.max_files = 12_345;
    config.limits.max_directories = 2_345;
    config.limits.max_total_bytes = 32 * 1024_u64.pow(3);
    config.limits.max_single_file_bytes = 8 * 1024_u64.pow(3);
    config.limits.max_depth = 48;
    config.limits.max_path_bytes = 2_048;
    config.behavior.preserve_mark_of_the_web = false;
    config.behavior.open_after_double_click = false;
    config.behavior.default_filename_encoding = FilenameEncoding::Cp932;

    let form = SettingsForm::from_config(&config);
    assert_eq!(form.max_archive_bytes, "16 GiB");
    assert_eq!(form.into_config().unwrap(), config);
}

#[test]
fn byte_fields_accept_binary_units_and_underscores() {
    let mut form = SettingsForm::from_config(&Config::default());
    form.max_archive_bytes = "1_024 MiB".to_owned();
    form.max_total_bytes = "2 GiB".to_owned();
    form.max_single_file_bytes = "512 MiB".to_owned();

    let config = form.into_config().unwrap();
    assert_eq!(config.limits.max_archive_bytes, 1024_u64.pow(3));
    assert_eq!(config.limits.max_total_bytes, 2 * 1024_u64.pow(3));
    assert_eq!(config.limits.max_single_file_bytes, 512 * 1024_u64.pow(2));
}

#[test]
fn invalid_field_is_reported_for_ui_focus() {
    let mut form = SettingsForm::from_config(&Config::default());
    form.memory_limit_mib = "32".to_owned();
    let error = form.into_config().unwrap_err();
    assert_eq!(error.field, SettingsField::MemoryLimitMib);

    let mut form = SettingsForm::from_config(&Config::default());
    form.max_single_file_bytes = "64 GiB".to_owned();
    let error = form.into_config().unwrap_err();
    assert_eq!(error.field, SettingsField::MaxSingleFileBytes);
}

#[test]
fn byte_format_uses_the_largest_exact_binary_unit() {
    assert_eq!(format_byte_count(16 * 1024_u64.pow(3)), "16 GiB");
    assert_eq!(format_byte_count(1_536), "1536 B");
}
