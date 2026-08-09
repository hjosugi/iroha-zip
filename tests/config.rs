use std::fs;
#[cfg(windows)]
use std::process::{Child, Command, ExitStatus};
use std::sync::{Arc, Barrier};
#[cfg(windows)]
use std::time::{Duration, Instant};

use iroha_zip::config::{AttachmentHandoffPolicy, Config, FilenameEncoding, IsolationMode};
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
        decoded.behavior.attachment_handoff,
        original.behavior.attachment_handoff
    );
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
    assert_eq!(
        config.behavior.attachment_handoff,
        AttachmentHandoffPolicy::Disabled
    );
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
    assert_eq!(
        config.behavior.attachment_handoff,
        AttachmentHandoffPolicy::Disabled
    );
}

#[test]
fn attachment_handoff_policy_is_explicit_and_unknown_values_are_rejected() {
    let best_effort: Config = toml::from_str(
        r#"
[behavior]
attachment_handoff = "best-effort"
"#,
    )
    .unwrap();
    assert_eq!(
        best_effort.behavior.attachment_handoff,
        AttachmentHandoffPolicy::BestEffort
    );

    let required: Config = toml::from_str(
        r#"
[behavior]
attachment_handoff = "required"
"#,
    )
    .unwrap();
    assert_eq!(
        required.behavior.attachment_handoff,
        AttachmentHandoffPolicy::Required
    );

    let invalid = r#"
[behavior]
attachment_handoff = "automatic"
"#;
    assert!(toml::from_str::<Config>(invalid).is_err());
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
    config.behavior.attachment_handoff = AttachmentHandoffPolicy::Required;
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

#[test]
fn concurrent_saves_leave_one_complete_configuration_and_no_staging_files() {
    let directory = std::env::temp_dir().join(format!(
        "iroha-zip-concurrent-config-{}",
        util::unique_token()
    ));
    let path = directory.join("config.toml");
    let mut first = Config::default();
    first.sandbox.timeout_seconds = 101;
    let mut second = Config::default();
    second.sandbox.timeout_seconds = 202;
    let barrier = Arc::new(Barrier::new(3));

    let handles = [first.clone(), second.clone()].map(|config| {
        let path = path.clone();
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            config.save(&path)
        })
    });
    barrier.wait();
    for handle in handles {
        handle.join().unwrap().unwrap();
    }

    let saved = Config::load(&path).unwrap();
    assert!(saved == first || saved == second);
    assert!(
        fs::read_dir(&directory)
            .unwrap()
            .all(|entry| entry.unwrap().file_name() == "config.toml")
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn concurrent_default_creation_reports_exactly_one_creator() {
    let directory =
        std::env::temp_dir().join(format!("iroha-zip-default-config-{}", util::unique_token()));
    let path = directory.join("config.toml");
    let barrier = Arc::new(Barrier::new(9));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                Config::write_default(&path)
            })
        })
        .collect();
    barrier.wait();

    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap().unwrap())
        .collect();
    assert_eq!(outcomes.iter().filter(|created| **created).count(), 1);
    assert_eq!(Config::load(&path).unwrap(), Config::default());
    assert!(
        fs::read_dir(&directory)
            .unwrap()
            .all(|entry| entry.unwrap().file_name() == "config.toml")
    );
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(windows)]
#[test]
fn independent_process_saves_leave_one_complete_configuration() {
    let directory = std::env::temp_dir().join(format!(
        "iroha-zip-process-config-日本語-{}",
        util::unique_token()
    ));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("設定.toml");
    let start = directory.join("start");
    let ready = [directory.join("ready-101"), directory.join("ready-202")];
    let mut children = [
        spawn_config_save_helper(&path, &start, &ready[0], 101),
        spawn_config_save_helper(&path, &start, &ready[1], 202),
    ];

    wait_for_paths(&ready, &mut children);
    fs::write(&start, b"start").unwrap();
    let statuses = children
        .iter_mut()
        .map(|child| wait_for_child(child, Duration::from_secs(30)))
        .collect::<Vec<_>>();
    assert!(
        statuses.iter().all(ExitStatus::success),
        "one or more config save helpers failed: {statuses:?}"
    );

    let saved = Config::load(&path).unwrap();
    assert!([101, 202].contains(&saved.sandbox.timeout_seconds));
    for coordination_file in ready.into_iter().chain([start]) {
        fs::remove_file(coordination_file).unwrap();
    }
    assert!(
        fs::read_dir(&directory)
            .unwrap()
            .all(|entry| entry.unwrap().file_name() == "設定.toml")
    );
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(windows)]
#[test]
fn config_save_subprocess_helper() {
    let Some(path) = std::env::var_os("IROHA_ZIP_TEST_CONFIG_PATH") else {
        return;
    };
    let start = std::path::PathBuf::from(
        std::env::var_os("IROHA_ZIP_TEST_CONFIG_START").expect("missing helper start path"),
    );
    let ready = std::path::PathBuf::from(
        std::env::var_os("IROHA_ZIP_TEST_CONFIG_READY").expect("missing helper ready path"),
    );
    let timeout_seconds = std::env::var("IROHA_ZIP_TEST_CONFIG_TIMEOUT")
        .expect("missing helper timeout")
        .parse()
        .expect("invalid helper timeout");

    fs::write(&ready, b"ready").unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !start.is_file() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for parent start signal"
        );
        std::thread::sleep(Duration::from_millis(10));
    }

    let mut config = Config::default();
    config.sandbox.timeout_seconds = timeout_seconds;
    config.save(std::path::Path::new(&path)).unwrap();
}

#[cfg(windows)]
fn spawn_config_save_helper(
    path: &std::path::Path,
    start: &std::path::Path,
    ready: &std::path::Path,
    timeout_seconds: u64,
) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("config_save_subprocess_helper")
        .arg("--nocapture")
        .env("IROHA_ZIP_TEST_CONFIG_PATH", path)
        .env("IROHA_ZIP_TEST_CONFIG_START", start)
        .env("IROHA_ZIP_TEST_CONFIG_READY", ready)
        .env("IROHA_ZIP_TEST_CONFIG_TIMEOUT", timeout_seconds.to_string())
        .spawn()
        .unwrap()
}

#[cfg(windows)]
fn wait_for_paths(paths: &[std::path::PathBuf], children: &mut [Child]) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while paths.iter().any(|path| !path.is_file()) {
        let mut early_exit = None;
        for child in &mut *children {
            if let Some(status) = child.try_wait().unwrap() {
                early_exit = Some(status);
                break;
            }
        }
        if let Some(status) = early_exit {
            terminate_children(children);
            panic!("config save helper exited before the start signal: {status}");
        }
        if Instant::now() >= deadline {
            terminate_children(children);
            panic!("timed out waiting for config save helpers");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(windows)]
fn wait_for_child(child: &mut Child, timeout: Duration) -> ExitStatus {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("timed out waiting for config save helper");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(windows)]
fn terminate_children(children: &mut [Child]) {
    for child in children {
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
        }
        let _ = child.wait();
    }
}
