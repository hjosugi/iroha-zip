#![cfg_attr(windows, windows_subsystem = "windows")]
#![cfg_attr(windows, allow(unsafe_code))]
#![cfg_attr(not(windows), deny(unsafe_code))]

use std::path::PathBuf;
use std::process::ExitCode;

use iroha_zip::config::default_config_path;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            show_error(&error.to_string());
            ExitCode::from(2)
        }
    }
}

fn run() -> iroha_zip::error::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let archive = arguments.next().map(PathBuf::from).ok_or_else(|| {
        iroha_zip::error::IrohaZipError::Usage(
            "iroha-zip-shell requires exactly one archive path".to_owned(),
        )
    })?;
    if arguments.next().is_some() {
        return Err(iroha_zip::error::IrohaZipError::Usage(
            "iroha-zip-shell requires exactly one archive path".to_owned(),
        ));
    }
    let config = default_config_path()?;
    let result = iroha_zip::shell_extract_with_report(&archive, &config)?;
    if result.attachment_handoff.is_incomplete() {
        show_warning(&result.attachment_handoff.message());
    }
    Ok(())
}

#[cfg(windows)]
fn show_warning(message: &str) {
    show_message(
        message,
        windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
    );
}

#[cfg(not(windows))]
fn show_warning(message: &str) {
    eprintln!("iroha-zip-shell: warning: {message}");
}

#[cfg(windows)]
fn show_error(message: &str) {
    show_message(
        message,
        windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
    );
}

#[cfg(windows)]
fn show_message(message: &str, icon: windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::WindowsAndMessaging::{MB_OK, MessageBoxW};
    use windows::core::PCWSTR;

    let body: Vec<u16> = OsStr::new(message)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let title: Vec<u16> = OsStr::new("iroha-zip")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(body.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | icon,
        );
    }
}

#[cfg(not(windows))]
fn show_error(message: &str) {
    eprintln!("iroha-zip-shell: {message}");
}
