use std::fmt::{self, Debug, Display, Formatter};

use zeroize::{Zeroize, Zeroizing};

use crate::error::{IrohaZipError, Result};

/// The stock Windows bsdtar callback owns a 1,024-byte buffer. Reserve one
/// byte for its terminator and one for the line-ending sent through `ConPTY`.
pub const MAX_PASSWORD_UTF8_BYTES: usize = 1_022;
pub const MAX_PASSWORD_UTF16_UNITS: usize = 1_022;
#[cfg(any(windows, test))]
pub(crate) const BACKEND_PASSWORD_PROMPT: &[u8] = b"Enter passphrase:";

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(any(windows, test))]
enum EscapeState {
    Plain,
    Escape,
    ControlSequence,
    OperatingSystemCommand,
    OperatingSystemCommandEscape,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(any(windows, test))]
pub(crate) enum PasswordOutputEvent {
    Prompt,
    AdditionalPrompt,
    OutputLimitExceeded,
}

/// Incrementally strips terminal control sequences, bounds raw pseudoconsole
/// output, and recognizes only the pinned bsdtar password prompt.
#[cfg(any(windows, test))]
pub(crate) struct PasswordOutputMonitor {
    raw_bytes: usize,
    max_raw_bytes: usize,
    prompt_count: u8,
    prompt_match: usize,
    line_start: bool,
    escape_state: EscapeState,
    overflow_reported: bool,
    suppress_log_after_prompt: bool,
}

#[cfg(any(windows, test))]
impl PasswordOutputMonitor {
    pub(crate) fn new(max_raw_bytes: usize) -> Self {
        Self {
            raw_bytes: 0,
            max_raw_bytes,
            prompt_count: 0,
            prompt_match: 0,
            line_start: true,
            escape_state: EscapeState::Plain,
            overflow_reported: false,
            suppress_log_after_prompt: false,
        }
    }

    pub(crate) fn consume(
        &mut self,
        input: &[u8],
        sanitized: &mut Vec<u8>,
        events: &mut Vec<PasswordOutputEvent>,
    ) {
        self.raw_bytes = self.raw_bytes.saturating_add(input.len());
        if self.raw_bytes > self.max_raw_bytes {
            if !self.overflow_reported {
                events.push(PasswordOutputEvent::OutputLimitExceeded);
                self.overflow_reported = true;
            }
            return;
        }

        for &byte in input {
            match self.escape_state {
                EscapeState::Plain => {
                    if byte == 0x1b {
                        self.escape_state = EscapeState::Escape;
                    } else if matches!(byte, b'\r' | b'\n' | b'\t') || byte >= 0x20 {
                        if !self.suppress_log_after_prompt {
                            sanitized.push(byte);
                        }
                        self.observe_prompt_byte(byte, events);
                    } else if !self.suppress_log_after_prompt {
                        let escaped = format!("\\x{byte:02X}");
                        sanitized.extend_from_slice(escaped.as_bytes());
                    }
                }
                EscapeState::Escape => match byte {
                    b'[' => self.escape_state = EscapeState::ControlSequence,
                    b']' => self.escape_state = EscapeState::OperatingSystemCommand,
                    _ => self.escape_state = EscapeState::Plain,
                },
                EscapeState::ControlSequence => {
                    if (0x40..=0x7e).contains(&byte) {
                        self.escape_state = EscapeState::Plain;
                    }
                }
                EscapeState::OperatingSystemCommand => {
                    if byte == 0x07 {
                        self.escape_state = EscapeState::Plain;
                    } else if byte == 0x1b {
                        self.escape_state = EscapeState::OperatingSystemCommandEscape;
                    }
                }
                EscapeState::OperatingSystemCommandEscape => {
                    self.escape_state = if byte == b'\\' {
                        EscapeState::Plain
                    } else if byte == 0x1b {
                        EscapeState::OperatingSystemCommandEscape
                    } else {
                        EscapeState::OperatingSystemCommand
                    };
                }
            }
        }
    }

    fn observe_prompt_byte(&mut self, byte: u8, events: &mut Vec<PasswordOutputEvent>) {
        if self.prompt_count > 0 {
            if matches!(byte, b'\r' | b'\n') {
                self.prompt_match = 0;
                return;
            }
            if BACKEND_PASSWORD_PROMPT.get(self.prompt_match) == Some(&byte) {
                self.prompt_match += 1;
                if self.prompt_match == BACKEND_PASSWORD_PROMPT.len() {
                    self.prompt_count = self.prompt_count.saturating_add(1);
                    events.push(PasswordOutputEvent::AdditionalPrompt);
                    self.prompt_match = 0;
                }
            } else {
                self.prompt_match = usize::from(BACKEND_PASSWORD_PROMPT.first() == Some(&byte));
            }
            return;
        }
        if matches!(byte, b'\r' | b'\n') {
            self.prompt_match = 0;
            self.line_start = true;
            return;
        }
        if !self.line_start {
            return;
        }
        if BACKEND_PASSWORD_PROMPT.get(self.prompt_match) != Some(&byte) {
            self.prompt_match = 0;
            self.line_start = false;
            return;
        }
        self.prompt_match += 1;
        if self.prompt_match == BACKEND_PASSWORD_PROMPT.len() {
            self.prompt_count = self.prompt_count.saturating_add(1);
            events.push(if self.prompt_count == 1 {
                self.suppress_log_after_prompt = true;
                PasswordOutputEvent::Prompt
            } else {
                PasswordOutputEvent::AdditionalPrompt
            });
            self.prompt_match = 0;
            self.line_start = false;
        }
    }

    pub(crate) fn prompt_count(&self) -> u8 {
        self.prompt_count
    }

    pub(crate) fn output_limit_exceeded(&self) -> bool {
        self.overflow_reported
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

    #[test]
    fn output_monitor_recognizes_one_fragmented_prompt_and_then_rejects_a_retry() {
        let mut monitor = PasswordOutputMonitor::new(4096);
        let mut sanitized = Vec::new();
        let mut events = Vec::new();
        monitor.consume(b"Enter pass", &mut sanitized, &mut events);
        monitor.consume(b"phrase:", &mut sanitized, &mut events);
        monitor.consume(b"Enter passphrase:", &mut sanitized, &mut events);
        assert_eq!(
            events,
            [
                PasswordOutputEvent::Prompt,
                PasswordOutputEvent::AdditionalPrompt
            ]
        );
        assert_eq!(monitor.prompt_count(), 2);
    }

    #[test]
    fn output_monitor_strips_terminal_sequences_before_logging_and_matching() {
        let mut monitor = PasswordOutputMonitor::new(4096);
        let mut sanitized = Vec::new();
        let mut events = Vec::new();
        monitor.consume(
            b"\x1b[31mEnter passphrase:\x1b[0m\x01",
            &mut sanitized,
            &mut events,
        );
        assert_eq!(sanitized, b"Enter passphrase:");
        assert_eq!(events, [PasswordOutputEvent::Prompt]);
    }

    #[test]
    fn output_monitor_never_logs_bytes_after_the_password_prompt() {
        let mut monitor = PasswordOutputMonitor::new(4096);
        let mut sanitized = Vec::new();
        let mut events = Vec::new();
        monitor.consume(
            b"status\r\nEnter passphrase:must-never-reach-the-log\r\n",
            &mut sanitized,
            &mut events,
        );
        assert_eq!(sanitized, b"status\r\nEnter passphrase:");
        assert_eq!(events, [PasswordOutputEvent::Prompt]);
    }

    #[test]
    fn output_monitor_does_not_treat_untrusted_inline_text_as_a_prompt() {
        let mut monitor = PasswordOutputMonitor::new(4096);
        let mut sanitized = Vec::new();
        let mut events = Vec::new();
        monitor.consume(
            b"entry named Enter passphrase: is invalid\r\n",
            &mut sanitized,
            &mut events,
        );
        assert!(events.is_empty());
        assert_eq!(monitor.prompt_count(), 0);
    }

    #[test]
    fn output_monitor_reports_overflow_once() {
        let mut monitor = PasswordOutputMonitor::new(3);
        let mut sanitized = Vec::new();
        let mut events = Vec::new();
        monitor.consume(b"abcd", &mut sanitized, &mut events);
        monitor.consume(b"efgh", &mut sanitized, &mut events);
        assert_eq!(events, [PasswordOutputEvent::OutputLimitExceeded]);
        assert!(sanitized.is_empty());
        assert!(monitor.output_limit_exceeded());
    }
}
