use std::ffi::{OsStr, OsString};
use std::path::{Component, Path};

use crate::backend::{BackendManifest, MAX_BACKEND_MANIFEST_FILES};
use crate::config::Config;
use crate::policy::{self, Limits};
use crate::{util, windows_command_line};

pub fn backend_manifest(input: &[u8]) {
    if let Ok(manifest) = BackendManifest::parse(input) {
        assert!((1..=MAX_BACKEND_MANIFEST_FILES).contains(&manifest.file_count()));
        assert!(manifest.file_hash(manifest.executable()).is_some());
    }
}

pub fn windows_paths(input: &[u8]) {
    let name = os_string_from_fuzz_bytes(input);
    let path = Path::new(&name);
    let limits = Limits::default();
    let _ = policy::validate_component(&name);
    if policy::validate_relative_path(path, &limits).is_ok() {
        assert!(!path.as_os_str().is_empty());
        assert!(!path.is_absolute());
        let components: Vec<_> = path.components().collect();
        assert!(components.len() <= limits.max_depth);
        for component in components {
            let Component::Normal(component) = component else {
                panic!("validated path contains a non-normal component");
            };
            policy::validate_component(component)
                .expect("every component of a validated path must be valid");
        }
    }
}

pub fn archive_name(input: &[u8]) {
    let Ok(text) = std::str::from_utf8(input) else {
        return;
    };
    let base = util::archive_base_name(text);
    assert!(!base.is_empty());
    policy::validate_component(OsStr::new(&base)).expect("normalized archive base must be safe");
}

pub fn command_line(input: &[u8]) {
    let units: Vec<u16> = if let Some((&0xff, bytes)) = input.split_first() {
        bytes
            .chunks(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk.get(1).copied().unwrap_or(0)]))
            .collect()
    } else {
        input.iter().map(|byte| u16::from(*byte)).collect()
    };
    if let Ok(encoded) = windows_command_line::quote(&units) {
        assert_eq!(
            windows_command_line::decode_single(&encoded).as_deref(),
            Some(units.as_slice())
        );
    }

    if let Ok(encoded) =
        windows_command_line::encode(&[u16::from(b'p')], std::slice::from_ref(&units))
    {
        assert_eq!(encoded.last(), Some(&0));
        assert!(!encoded[..encoded.len() - 1].contains(&0));
        assert!(encoded.len() <= 32_767);
        assert_eq!(
            windows_command_line::decode_single(&encoded[2..encoded.len() - 1]).as_deref(),
            Some(units.as_slice())
        );
    }
}

pub fn config_round_trip(input: &[u8]) {
    let Ok(config) = Config::parse(input) else {
        return;
    };
    let encoded = config
        .serialized()
        .expect("a validated configuration must serialize");
    let decoded = Config::parse(encoded.as_bytes())
        .expect("a serialized configuration must parse and validate");
    assert_eq!(decoded, config);
}

#[cfg(unix)]
fn os_string_from_fuzz_bytes(input: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt as _;

    OsString::from_vec(input.to_vec())
}

#[cfg(windows)]
fn os_string_from_fuzz_bytes(input: &[u8]) -> OsString {
    use std::os::windows::ffi::OsStringExt as _;

    let units: Vec<u16> = input
        .chunks(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk.get(1).copied().unwrap_or(0)]))
        .collect();
    OsString::from_wide(&units)
}
