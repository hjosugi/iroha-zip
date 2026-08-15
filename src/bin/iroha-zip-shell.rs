#![cfg_attr(windows, windows_subsystem = "windows")]
#![cfg_attr(windows, allow(unsafe_code))]
#![cfg_attr(not(windows), deny(unsafe_code))]

use std::path::PathBuf;
use std::process::ExitCode;

#[cfg(windows)]
use clap::Parser as _;
use iroha_zip::config::default_config_path;
use iroha_zip::error::IrohaZipError;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if is_internal_archive_reader_invocation() {
                eprintln!("iroha-zip: {error}");
            } else {
                show_error(&user_facing_error(&error));
            }
            ExitCode::from(2)
        }
    }
}

fn run() -> iroha_zip::error::Result<()> {
    #[cfg(windows)]
    if let Some(result) = run_internal_archive_reader() {
        return result;
    }

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
        show_warning(&user_facing_handoff_warning(
            &result.attachment_handoff.message(),
        ));
    }
    Ok(())
}

fn user_facing_error(error: &IrohaZipError) -> String {
    let (english, japanese) = match error {
        IrohaZipError::Io { .. } => (
            "A file or system operation failed.",
            "ファイルまたはシステムの処理に失敗しました。",
        ),
        IrohaZipError::Config(_) => (
            "The iroha-zip configuration is invalid.",
            "iroha-zipの設定が正しくありません。",
        ),
        IrohaZipError::Backend(_) => (
            "The archive backend could not be verified or used.",
            "書庫バックエンドを検証または使用できませんでした。",
        ),
        IrohaZipError::Policy(_) => (
            "The archive was rejected by the safety policy.",
            "安全ポリシーにより書庫を拒否しました。",
        ),
        IrohaZipError::TrustHandoff(_) => (
            "Windows trust handoff failed.",
            "Windowsの信頼連携に失敗しました。",
        ),
        IrohaZipError::Sandbox(_) => (
            "The isolated archive process failed.",
            "分離された書庫処理に失敗しました。",
        ),
        IrohaZipError::Unsupported(_) => (
            "This archive operation is not supported.",
            "この書庫操作には対応していません。",
        ),
        IrohaZipError::Usage(_) => (
            "The archive request is invalid.",
            "書庫の操作指定が正しくありません。",
        ),
    };
    bilingual_details(english, japanese, &error.to_string())
}

fn user_facing_handoff_warning(details: &str) -> String {
    bilingual_details(
        "Extraction completed, but Windows trust handoff was incomplete.",
        "展開は完了しましたが、Windowsの信頼連携は完了しませんでした。",
        details,
    )
}

fn bilingual_details(english: &str, japanese: &str, details: &str) -> String {
    format!("{english}\r\n{japanese}\r\n\r\nTechnical details / 技術情報:\r\n{details}")
}

#[cfg(windows)]
fn run_internal_archive_reader() -> Option<iroha_zip::error::Result<()>> {
    use iroha_zip::cli::{Cli, Command};

    if !is_internal_archive_reader_invocation() {
        return None;
    }
    let operation = (|| {
        let cli = Cli::try_parse_from(std::env::args_os()).map_err(|error| {
            iroha_zip::error::IrohaZipError::Usage(format!(
                "invalid internal archive reader arguments: {error}"
            ))
        })?;
        match cli.command {
            Command::InternalArchiveListing {
                backend_root,
                candidates,
                archive,
                encoding,
                max_entries,
                max_path_bytes,
                allow_unsandboxed,
            } => iroha_zip::platform::write_utf8_archive_listing(
                &backend_root,
                &candidates,
                &archive,
                encoding,
                max_entries,
                max_path_bytes,
                allow_unsandboxed,
            ),
            Command::InternalPasswordArchiveExtraction {
                backend_root,
                candidates,
                archive,
                output,
                encoding,
                max_files,
                max_directories,
                max_total_bytes,
                max_single_file_bytes,
                max_depth,
                max_path_bytes,
            } => iroha_zip::platform::extract_password_archive(
                &backend_root,
                &candidates,
                &archive,
                &output,
                encoding,
                &iroha_zip::policy::Limits {
                    max_archive_bytes: u64::MAX,
                    max_files,
                    max_directories,
                    max_total_bytes,
                    max_single_file_bytes,
                    max_depth,
                    max_path_bytes,
                },
            ),
            Command::InternalRawArchive {
                backend_root,
                candidates,
                archive,
                filter,
                output_name,
                max_bytes,
                output,
                allow_unsandboxed,
            } => iroha_zip::platform::process_raw_archive(
                &backend_root,
                &candidates,
                &archive,
                filter,
                &output_name,
                max_bytes,
                output.as_deref(),
                allow_unsandboxed,
            ),
            _ => Err(iroha_zip::error::IrohaZipError::Usage(
                "internal archive reader dispatch mismatch".to_owned(),
            )),
        }
    })();
    Some(operation)
}

fn is_internal_archive_reader_invocation() -> bool {
    use std::ffi::OsStr;

    std::env::args_os().nth(1).is_some_and(|argument| {
        argument == OsStr::new("internal-archive-listing")
            || argument == OsStr::new("internal-password-archive-extraction")
            || argument == OsStr::new("internal-raw-archive")
    })
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

#[cfg(test)]
mod tests {
    use std::io;

    use super::{user_facing_error, user_facing_handoff_warning};
    use iroha_zip::error::IrohaZipError;

    #[test]
    fn every_shell_error_category_has_english_and_japanese_context() {
        let cases = [
            (
                IrohaZipError::io("read archive", io::Error::other("test detail")),
                "A file or system operation failed.",
                "ファイルまたはシステムの処理に失敗しました。",
                "read archive: test detail",
            ),
            (
                IrohaZipError::Config("test detail".to_owned()),
                "The iroha-zip configuration is invalid.",
                "iroha-zipの設定が正しくありません。",
                "configuration error: test detail",
            ),
            (
                IrohaZipError::Backend("test detail".to_owned()),
                "The archive backend could not be verified or used.",
                "書庫バックエンドを検証または使用できませんでした。",
                "backend error: test detail",
            ),
            (
                IrohaZipError::Policy("test detail".to_owned()),
                "The archive was rejected by the safety policy.",
                "安全ポリシーにより書庫を拒否しました。",
                "archive rejected: test detail",
            ),
            (
                IrohaZipError::TrustHandoff("test detail".to_owned()),
                "Windows trust handoff failed.",
                "Windowsの信頼連携に失敗しました。",
                "Windows trust handoff failed: test detail",
            ),
            (
                IrohaZipError::Sandbox("test detail".to_owned()),
                "The isolated archive process failed.",
                "分離された書庫処理に失敗しました。",
                "sandbox error: test detail",
            ),
            (
                IrohaZipError::Unsupported("test detail".to_owned()),
                "This archive operation is not supported.",
                "この書庫操作には対応していません。",
                "unsupported operation: test detail",
            ),
            (
                IrohaZipError::Usage("test detail".to_owned()),
                "The archive request is invalid.",
                "書庫の操作指定が正しくありません。",
                "usage error: test detail",
            ),
        ];

        for (error, english, japanese, detail) in cases {
            let message = user_facing_error(&error);
            assert!(message.starts_with(&format!("{english}\r\n{japanese}\r\n")));
            assert!(message.contains("Technical details / 技術情報:\r\n"));
            assert!(message.ends_with(detail));
        }
    }

    #[test]
    fn trust_handoff_warning_is_bilingual_and_keeps_technical_details() {
        let message = user_facing_handoff_warning("test handoff detail");
        assert!(message.starts_with(
            "Extraction completed, but Windows trust handoff was incomplete.\r\n\
             展開は完了しましたが、Windowsの信頼連携は完了しませんでした。\r\n"
        ));
        assert!(message.contains("Technical details / 技術情報:\r\n"));
        assert!(message.ends_with("test handoff detail"));
    }
}
