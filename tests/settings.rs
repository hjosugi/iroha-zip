use iroha_zip::config::{AttachmentHandoffPolicy, Config, FilenameEncoding, IsolationMode};
use iroha_zip::settings::{
    SettingsAction, SettingsField, SettingsForm, control_id, format_byte_count, scale_between_dpi,
    scale_logical,
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
    let mut tab_ids = control_id::TAB_ORDER;
    tab_ids.sort_unstable();
    assert_eq!(tab_ids.as_slice(), ids.as_slice());
}

#[test]
fn native_ui_test_matches_and_exercises_the_complete_keyboard_tab_order() {
    let script = include_str!("../scripts/test-settings-ui.ps1");
    let (_, tab_order_source) = script
        .split_once("$tabOrder = @(")
        .expect("native UI test must declare the expected tab order");
    let (tab_order_source, _) = tab_order_source
        .split_once("\n    )")
        .expect("native UI test tab order must have a bounded literal");
    let tab_order = tab_order_source
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<usize>()
                .unwrap_or_else(|error| panic!("invalid scripted tab-order ID {value:?}: {error}"))
        })
        .collect::<Vec<_>>();
    assert_eq!(tab_order, control_id::TAB_ORDER);

    for marker in [
        "[IrohaZipUiAutomationNative]::SendTab($false)",
        "[IrohaZipUiAutomationNative]::SendTab($true)",
        "[IrohaZipUiAutomationNative]::SendKey([ushort]0x0D)",
        "[IrohaZipUiAutomationNative]::SendKey([ushort]0x1B)",
        "[IrohaZipUiAutomationNative]::ActivateAndClick(",
        "[IrohaZipUiAutomationNative]::SetAndVerifyThreadFocus(",
        "activationMethod = \"SendInputMouseClick\"",
        "GitHubHostedWindowsArm64NoForegroundFocus",
        "realKeyInput = $realKeyInput",
        "enterKeyVerified = $shortcutRealKeyInput",
        "escapeKeyVerified = $shortcutRealKeyInput",
        "escapeCloseRequestCompleted = $true",
        "closeCancellationPreservedProcess = $true",
        "savedTimeoutSeconds = 301",
        "$shortcutTimeoutPattern.SetValue(\"300\")",
        "the shortcut-test default-baseline saved message",
        "keyboardShortcuts = $shortcutEvidence",
        "forwardWrapTarget = $firstId",
        "reverseWrapTarget = [int]$TabOrder[$TabOrder.Count - 1]",
        "allFocusedControlsVisible = $true",
        "targetProcessVerifiedAfterEveryChord = $true",
        "foregroundWindowConfirmed = $foregroundWindowConfirmed",
        "schemaVersion = 2",
    ] {
        assert!(
            script.contains(marker),
            "native UI keyboard contract is missing {marker:?}"
        );
    }
    assert!(
        !script.contains("the settings window to become the keyboard foreground window"),
        "hosted Windows runners must verify the focused target rather than require a matching foreground HWND"
    );
    assert!(
        script.contains("$env:GITHUB_ACTIONS -eq \"true\" -and $env:RUNNER_ARCH -eq \"ARM64\""),
        "the non-key-input fallback must remain restricted to GitHub-hosted ARM64"
    );

    let traversal_position = script
        .find("$keyboardTraversal = Test-KeyboardTabOrder")
        .expect("the native UI test must run keyboard traversal");
    let shortcut_position = script
        .find("$shortcutTimeoutPattern =")
        .expect("the native UI test must run the keyboard-shortcut contract");
    let dpi_position = script
        .find("Test-SyntheticDpiTransition -MainWindow")
        .expect("the native UI test must run the DPI-transition contract");
    assert!(
        traversal_position < shortcut_position && shortcut_position < dpi_position,
        "real Enter/Escape input must run immediately after traversal and before dialog-heavy UI paths"
    );
    assert!(
        !script[shortcut_position..dpi_position].contains("Start-Process"),
        "keyboard shortcuts must reuse the foreground traversal process"
    );
    assert!(
        !script.contains("Focus-ControlForRealInput"),
        "keyboard shortcuts must not try to reacquire global foreground input through pointer activation"
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

#[test]
fn transient_pixels_rescale_between_monitor_dpi_spaces() {
    assert_eq!(scale_between_dpi(125, 120, 144), 150);
    assert_eq!(scale_between_dpi(-125, 120, 144), -150);
    assert_eq!(scale_between_dpi(150, 144, 96), 100);
    assert_eq!(scale_between_dpi(i32::MAX, 96, u32::MAX), i32::MAX);
    assert_eq!(scale_between_dpi(i32::MIN, 96, u32::MAX), i32::MIN);
    assert_eq!(scale_between_dpi(100, 0, 0), 100);
}

#[test]
fn settings_manifest_declares_per_monitor_v2_with_legacy_fallback() {
    let manifest = include_str!("../assets/iroha-zip-settings.manifest");
    assert!(manifest.contains(">true/pm</dpiAware>"));
    assert!(manifest.contains(">PerMonitorV2, PerMonitor</dpiAwareness>"));
}
