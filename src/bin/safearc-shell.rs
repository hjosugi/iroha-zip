#![cfg_attr(windows, windows_subsystem = "windows")]
#![cfg_attr(windows, allow(unsafe_code))]
#![cfg_attr(not(windows), deny(unsafe_code))]

use std::path::PathBuf;
use std::process::ExitCode;

use safearc::config::default_config_path;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            show_error(&error.to_string());
            ExitCode::from(2)
        }
    }
}

fn run() -> safearc::error::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let archive = arguments.next().map(PathBuf::from).ok_or_else(|| {
        safearc::error::SafeArcError::Usage(
            "safearc-shell requires exactly one archive path".to_owned(),
        )
    })?;
    if arguments.next().is_some() {
        return Err(safearc::error::SafeArcError::Usage(
            "safearc-shell requires exactly one archive path".to_owned(),
        ));
    }
    let config = default_config_path()?;
    let _ = safearc::shell_extract(&archive, &config)?;
    Ok(())
}

#[cfg(windows)]
fn show_error(message: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    use windows::core::PCWSTR;

    let body: Vec<u16> = OsStr::new(message)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let title: Vec<u16> = OsStr::new("SafeArc")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(body.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn show_error(message: &str) {
    eprintln!("safearc-shell: {message}");
}
