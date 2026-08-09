use std::fmt;
use std::path::PathBuf;

use crate::config::{
    Config, FilenameEncoding, MAX_MEMORY_LIMIT_MIB, MAX_TIMEOUT_SECONDS, MIN_MEMORY_LIMIT_MIB,
    MIN_TIMEOUT_SECONDS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsField {
    General,
    BackendDirectory,
    TimeoutSeconds,
    MemoryLimitMib,
    MaxArchiveBytes,
    MaxFiles,
    MaxDirectories,
    MaxTotalBytes,
    MaxSingleFileBytes,
    MaxDepth,
    MaxPathBytes,
}

impl SettingsField {
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "設定",
            Self::BackendDirectory => "バックエンド保存先",
            Self::TimeoutSeconds => "タイムアウト",
            Self::MemoryLimitMib => "メモリ上限",
            Self::MaxArchiveBytes => "入力書庫の上限",
            Self::MaxFiles => "ファイル数",
            Self::MaxDirectories => "ディレクトリ数",
            Self::MaxTotalBytes => "合計容量",
            Self::MaxSingleFileBytes => "単一ファイル容量",
            Self::MaxDepth => "パスの深さ",
            Self::MaxPathBytes => "パス長",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsValidationError {
    pub field: SettingsField,
    pub message: String,
}

impl SettingsValidationError {
    fn new(field: SettingsField, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl fmt::Display for SettingsValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field.label(), self.message)
    }
}

impl std::error::Error for SettingsValidationError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsForm {
    pub backend_directory: String,
    pub timeout_seconds: String,
    pub memory_limit_mib: String,
    pub max_archive_bytes: String,
    pub max_files: String,
    pub max_directories: String,
    pub max_total_bytes: String,
    pub max_single_file_bytes: String,
    pub max_depth: String,
    pub max_path_bytes: String,
    pub preserve_mark_of_the_web: bool,
    pub open_after_double_click: bool,
    pub default_filename_encoding: FilenameEncoding,
}

impl SettingsForm {
    pub fn from_config(config: &Config) -> Self {
        let backend = config
            .backend
            .directory
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new("backend/libarchive"));
        Self {
            backend_directory: backend.to_string_lossy().into_owned(),
            timeout_seconds: config.sandbox.timeout_seconds.to_string(),
            memory_limit_mib: config.sandbox.memory_limit_mib.to_string(),
            max_archive_bytes: format_byte_count(config.limits.max_archive_bytes),
            max_files: config.limits.max_files.to_string(),
            max_directories: config.limits.max_directories.to_string(),
            max_total_bytes: format_byte_count(config.limits.max_total_bytes),
            max_single_file_bytes: format_byte_count(config.limits.max_single_file_bytes),
            max_depth: config.limits.max_depth.to_string(),
            max_path_bytes: config.limits.max_path_bytes.to_string(),
            preserve_mark_of_the_web: config.behavior.preserve_mark_of_the_web,
            open_after_double_click: config.behavior.open_after_double_click,
            default_filename_encoding: config.behavior.default_filename_encoding,
        }
    }

    pub fn into_config(self) -> Result<Config, SettingsValidationError> {
        let mut config = Config::default();
        let backend = self.backend_directory.trim();
        config.backend.directory = if backend.is_empty() {
            None
        } else {
            Some(PathBuf::from(backend))
        };

        config.sandbox.timeout_seconds = parse_bounded_u64(
            &self.timeout_seconds,
            SettingsField::TimeoutSeconds,
            MIN_TIMEOUT_SECONDS,
            MAX_TIMEOUT_SECONDS,
        )?;
        config.sandbox.memory_limit_mib = parse_bounded_u64(
            &self.memory_limit_mib,
            SettingsField::MemoryLimitMib,
            MIN_MEMORY_LIMIT_MIB,
            MAX_MEMORY_LIMIT_MIB,
        )?;
        config.limits.max_archive_bytes =
            parse_byte_count(&self.max_archive_bytes, SettingsField::MaxArchiveBytes)?;
        config.limits.max_files = parse_positive_u64(&self.max_files, SettingsField::MaxFiles)?;
        config.limits.max_directories =
            parse_positive_u64(&self.max_directories, SettingsField::MaxDirectories)?;
        config.limits.max_total_bytes =
            parse_byte_count(&self.max_total_bytes, SettingsField::MaxTotalBytes)?;
        config.limits.max_single_file_bytes = parse_byte_count(
            &self.max_single_file_bytes,
            SettingsField::MaxSingleFileBytes,
        )?;
        config.limits.max_depth = parse_positive_usize(&self.max_depth, SettingsField::MaxDepth)?;
        config.limits.max_path_bytes =
            parse_positive_usize(&self.max_path_bytes, SettingsField::MaxPathBytes)?;
        config.behavior.preserve_mark_of_the_web = self.preserve_mark_of_the_web;
        config.behavior.open_after_double_click = self.open_after_double_click;
        config.behavior.default_filename_encoding = self.default_filename_encoding;

        if config.limits.max_single_file_bytes > config.limits.max_total_bytes {
            return Err(SettingsValidationError::new(
                SettingsField::MaxSingleFileBytes,
                "合計容量以下にしてください。",
            ));
        }
        config.validate().map_err(|error| {
            SettingsValidationError::new(SettingsField::General, error.to_string())
        })?;
        Ok(config)
    }
}

pub fn format_byte_count(bytes: u64) -> String {
    for (unit, multiplier) in [
        ("TiB", 1024_u64.pow(4)),
        ("GiB", 1024_u64.pow(3)),
        ("MiB", 1024_u64.pow(2)),
        ("KiB", 1024_u64),
    ] {
        if bytes >= multiplier && bytes.is_multiple_of(multiplier) {
            return format!("{} {unit}", bytes / multiplier);
        }
    }
    format!("{bytes} B")
}

fn parse_byte_count(input: &str, field: SettingsField) -> Result<u64, SettingsValidationError> {
    let normalized = input.trim().replace('_', "");
    let digit_end = normalized
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(normalized.len());
    let (digits, suffix) = normalized.split_at(digit_end);
    let value = digits.parse::<u64>().map_err(|_| {
        SettingsValidationError::new(field, "0より大きい整数と単位を入力してください。")
    })?;
    if value == 0 {
        return Err(SettingsValidationError::new(
            field,
            "0より大きい値を入力してください。",
        ));
    }
    let multiplier = match suffix.trim().to_ascii_lowercase().as_str() {
        "" | "b" | "byte" | "bytes" => 1,
        "kib" => 1024_u64,
        "mib" => 1024_u64.pow(2),
        "gib" => 1024_u64.pow(3),
        "tib" => 1024_u64.pow(4),
        _ => {
            return Err(SettingsValidationError::new(
                field,
                "単位は B、KiB、MiB、GiB、TiB のいずれかを使用してください。",
            ));
        }
    };
    value
        .checked_mul(multiplier)
        .ok_or_else(|| SettingsValidationError::new(field, "値が大きすぎます。"))
}

fn parse_positive_u64(input: &str, field: SettingsField) -> Result<u64, SettingsValidationError> {
    let value =
        input.trim().replace('_', "").parse::<u64>().map_err(|_| {
            SettingsValidationError::new(field, "0より大きい整数を入力してください。")
        })?;
    if value == 0 {
        return Err(SettingsValidationError::new(
            field,
            "0より大きい整数を入力してください。",
        ));
    }
    Ok(value)
}

fn parse_positive_usize(
    input: &str,
    field: SettingsField,
) -> Result<usize, SettingsValidationError> {
    let value = parse_positive_u64(input, field)?;
    usize::try_from(value).map_err(|_| SettingsValidationError::new(field, "値が大きすぎます。"))
}

fn parse_bounded_u64(
    input: &str,
    field: SettingsField,
    minimum: u64,
    maximum: u64,
) -> Result<u64, SettingsValidationError> {
    let value = input.trim().replace('_', "").parse::<u64>().map_err(|_| {
        SettingsValidationError::new(
            field,
            format!("{minimum}から{maximum}の整数を入力してください。"),
        )
    })?;
    if !(minimum..=maximum).contains(&value) {
        return Err(SettingsValidationError::new(
            field,
            format!("{minimum}から{maximum}の範囲で入力してください。"),
        ));
    }
    Ok(value)
}
