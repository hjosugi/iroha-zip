use std::fmt::{self, Debug, Display, Formatter};

use zeroize::{Zeroize, Zeroizing};

use crate::error::{IrohaZipError, Result};

/// Keep the one-use UTF-8 pipe value within a small, auditable bound while
/// leaving room for its terminator and transport delimiter.
pub const MAX_PASSWORD_UTF8_BYTES: usize = 1_022;
pub const MAX_PASSWORD_UTF16_UNITS: usize = 1_022;

/// A bounded archive password whose formatting is always redacted.
///
/// This type deliberately does not implement `Clone`. Its UTF-16 storage is
/// zeroized on every drop path, including failures before process creation.
pub struct ArchivePassword {
    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    utf16: Zeroizing<Vec<u16>>,
}

#[derive(Debug)]
pub enum PasswordPreparation {
    Cancelled,
    Ready(Option<ArchivePassword>),
}

pub fn prepare_password<F>(requested: bool, prompt: F) -> Result<PasswordPreparation>
where
    F: FnOnce() -> Result<Option<ArchivePassword>>,
{
    if !requested {
        return Ok(PasswordPreparation::Ready(None));
    }
    Ok(match prompt()? {
        Some(password) => PasswordPreparation::Ready(Some(password)),
        None => PasswordPreparation::Cancelled,
    })
}

impl ArchivePassword {
    pub fn from_utf16(units: Vec<u16>) -> Result<Self> {
        let mut units = Zeroizing::new(units);
        if units.is_empty() {
            return Err(IrohaZipError::Usage(
                "archive password must not be empty".to_owned(),
            ));
        }
        if units.len() > MAX_PASSWORD_UTF16_UNITS {
            return Err(IrohaZipError::Usage(format!(
                "archive password exceeds {MAX_PASSWORD_UTF16_UNITS} UTF-16 units"
            )));
        }
        if units.iter().any(|unit| matches!(*unit, 0 | 10 | 13)) {
            return Err(IrohaZipError::Usage(
                "archive password contains NUL or a line break".to_owned(),
            ));
        }
        if std::char::decode_utf16(units.iter().copied()).any(|unit| unit.is_err()) {
            units.zeroize();
            return Err(IrohaZipError::Usage(
                "archive password is not valid UTF-16".to_owned(),
            ));
        }
        let utf8_bytes = std::char::decode_utf16(units.iter().copied())
            .map(|unit| unit.map_or(0, char::len_utf8))
            .sum::<usize>();
        if utf8_bytes > MAX_PASSWORD_UTF8_BYTES {
            return Err(IrohaZipError::Usage(format!(
                "UTF-8 archive password exceeds {MAX_PASSWORD_UTF8_BYTES} bytes"
            )));
        }
        Ok(Self { utf16: units })
    }

    #[cfg(any(windows, test))]
    pub(crate) fn into_transport(self) -> Result<PasswordTransport> {
        let mut text = Zeroizing::new(String::from_utf16(&self.utf16).map_err(|_| {
            IrohaZipError::Usage("archive password is not valid UTF-16".to_owned())
        })?);
        if text.len() > MAX_PASSWORD_UTF8_BYTES {
            return Err(IrohaZipError::Usage(format!(
                "UTF-8 archive password exceeds {MAX_PASSWORD_UTF8_BYTES} bytes"
            )));
        }
        let bytes = std::mem::take(&mut *text).into_bytes();
        Ok(PasswordTransport {
            bytes: Zeroizing::new(bytes),
        })
    }
}

impl Debug for ArchivePassword {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ArchivePassword([REDACTED])")
    }
}

impl Display for ArchivePassword {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[cfg(any(windows, test))]
pub(crate) struct PasswordTransport {
    bytes: Zeroizing<Vec<u8>>,
}

#[cfg(any(windows, test))]
impl PasswordTransport {
    pub(crate) fn line(&mut self) -> &[u8] {
        self.bytes.push(b'\r');
        &self.bytes
    }
}

#[cfg(any(windows, test))]
impl Debug for PasswordTransport {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PasswordTransport([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_formatting_is_always_redacted() {
        let password = ArchivePassword::from_utf16("内緒secret".encode_utf16().collect()).unwrap();
        assert_eq!(password.to_string(), "[REDACTED]");
        assert_eq!(format!("{password:?}"), "ArchivePassword([REDACTED])");
    }

    #[test]
    fn process_spec_debug_never_contains_the_secret() {
        let secret = "sentinel-secret-value";
        let spec = crate::platform::ProcessSpec {
            program: "backend.exe".into(),
            args: vec!["-x".into()],
            current_dir: ".".into(),
            temp_dir: None,
            stdin_file: None,
            interactive_password: Some(
                ArchivePassword::from_utf16(secret.encode_utf16().collect()).unwrap(),
            ),
            stdout_log: "stdout.log".into(),
            stderr_log: "stderr.log".into(),
            timeout: std::time::Duration::from_secs(1),
            monitor_root: None,
            limits: crate::policy::Limits::default(),
        };
        let debug = format!("{spec:?}");
        assert!(!debug.contains(secret));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn cancellation_returns_before_any_password_is_prepared() {
        let cancelled = prepare_password(true, || Ok(None)).unwrap();
        assert!(matches!(cancelled, PasswordPreparation::Cancelled));

        let mut prompt_called = false;
        let disabled = prepare_password(false, || {
            prompt_called = true;
            Ok(None)
        })
        .unwrap();
        assert!(matches!(disabled, PasswordPreparation::Ready(None)));
        assert!(!prompt_called);
    }

    #[test]
    fn secret_rejects_empty_invalid_and_oversized_values_without_echoing_them() {
        assert!(ArchivePassword::from_utf16(Vec::new()).is_err());
        assert!(ArchivePassword::from_utf16(vec![u16::from(b'x'), 0]).is_err());
        assert!(ArchivePassword::from_utf16(vec![0xd800]).is_err());
        assert!(
            ArchivePassword::from_utf16(vec![u16::from(b'x'); MAX_PASSWORD_UTF16_UNITS + 1])
                .is_err()
        );
        assert!(ArchivePassword::from_utf16("秘".repeat(400).encode_utf16().collect()).is_err());
    }

    #[test]
    fn utf8_transport_is_bounded_and_has_one_line_ending() {
        let password = ArchivePassword::from_utf16("日本語".encode_utf16().collect()).unwrap();
        let mut transport = password.into_transport().unwrap();
        assert_eq!(transport.line(), "日本語\r".as_bytes());
    }
}
