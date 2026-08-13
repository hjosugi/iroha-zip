use iroha_zip::config::{AttachmentHandoffPolicy, Config, FilenameEncoding, IsolationMode};
use iroha_zip::settings::{
    SettingsAction, SettingsField, SettingsForm, control_id, format_byte_count, scale_logical,
};

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
    config.behavior.attachment_handoff = AttachmentHandoffPolicy::BestEffort;
    config.behavior.open_after_double_click = false;
    config.behavior.default_filename_encoding = FilenameEncoding::Cp932;

    let form = SettingsForm::from_config(&config);
    assert_eq!(form.max_archive_bytes, "16 GiB");
    assert_eq!(form.into_config().unwrap(), config);
}

#[test]
fn default_backend_display_round_trips_without_a_false_dirty_change() {
    let config = Config::default();
    assert_eq!(
        SettingsForm::from_config(&config).into_config().unwrap(),
        config
    );
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
    assert!(error.to_string().contains("メモリ上限"));
    assert!(error.english().contains("Memory limit"));
    assert!(error.english().contains("64 through 1048576"));

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

#[test]
fn native_control_ids_are_stable_unique_and_disjoint() {
    let mut ids = control_id::SETTING_CONTROLS.to_vec();
    ids.extend(control_id::ACTION_BUTTONS);
    assert!(ids.iter().all(|id| *id > 0));
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 26);
    assert!(
        control_id::SETTING_CONTROLS
            .iter()
            .all(|id| control_id::is_setting(*id))
    );
    assert!(
        control_id::ACTION_BUTTONS
            .iter()
            .all(|id| !control_id::is_setting(*id))
    );
}

#[test]
fn every_action_button_has_an_exhaustive_dispatch_mapping() {
    let mapped = SettingsAction::ALL.map(SettingsAction::control_id);
    assert_eq!(mapped, control_id::ACTION_BUTTONS);
    for action in SettingsAction::ALL {
        assert_eq!(
            SettingsAction::from_control_id(action.control_id()),
            Some(action)
        );
    }
    assert_eq!(SettingsAction::from_control_id(0), None);
    assert_eq!(
        SettingsAction::from_control_id(control_id::BACKEND_DIRECTORY),
        None
    );
}

#[test]
fn logical_layout_scaling_covers_100_through_300_percent() {
    for (dpi, expected) in [(96, 100), (120, 125), (144, 150), (192, 200), (288, 300)] {
        assert_eq!(scale_logical(100, dpi), expected);
        assert_eq!(scale_logical(-100, dpi), -expected);
    }
    assert_eq!(scale_logical(100, 0), 100);
}
