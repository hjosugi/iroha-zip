use std::fmt;
use std::path::PathBuf;

use crate::config::{
    AttachmentHandoffPolicy, Config, FilenameEncoding, IsolationMode, MAX_MEMORY_LIMIT_MIB,
    MAX_TIMEOUT_SECONDS, MIN_MEMORY_LIMIT_MIB, MIN_TIMEOUT_SECONDS,
};

pub const BASE_DPI: u32 = 96;

pub fn scale_logical(value: i32, dpi: u32) -> i32 {
    let numerator = i64::from(value) * i64::from(dpi.max(BASE_DPI));
    let rounded = if numerator >= 0 {
        numerator + i64::from(BASE_DPI / 2)
    } else {
        numerator - i64::from(BASE_DPI / 2)
    };
    i32::try_from(rounded / i64::from(BASE_DPI)).unwrap_or_else(|_| {
        if rounded.is_negative() {
            i32::MIN
        } else {
            i32::MAX
        }
    })
}

pub mod control_id {
    pub const BACKEND_BROWSE: usize = 1001;
    pub const BACKEND_DOCTOR: usize = 1002;
    pub const BACKEND_IMPORT: usize = 1003;
    pub const BACKEND_MSYS2: usize = 1004;
    pub const REGISTER: usize = 1101;
    pub const UNREGISTER: usize = 1102;
    pub const DEFAULT_APPS: usize = 1103;
    pub const CONFIG_FOLDER: usize = 1104;
    pub const DEFAULTS: usize = 1201;
    // IDOK and IDCANCEL let IsDialogMessageW map Enter and Escape naturally.
    pub const SAVE: usize = 1;
    pub const CANCEL: usize = 2;

    pub const BACKEND_DIRECTORY: usize = 2001;
    pub const ISOLATION: usize = 2002;
    pub const TIMEOUT_SECONDS: usize = 2003;
    pub const MEMORY_LIMIT_MIB: usize = 2004;
    pub const MAX_ARCHIVE_BYTES: usize = 2005;
    pub const MAX_FILES: usize = 2006;
    pub const MAX_DIRECTORIES: usize = 2007;
    pub const MAX_TOTAL_BYTES: usize = 2008;
    pub const MAX_SINGLE_FILE_BYTES: usize = 2009;
    pub const MAX_DEPTH: usize = 2010;
    pub const MAX_PATH_BYTES: usize = 2011;
    pub const PRESERVE_MOTW: usize = 2012;
    pub const OPEN_AFTER_DOUBLE_CLICK: usize = 2013;
    pub const ENCODING: usize = 2014;
    pub const ATTACHMENT_HANDOFF: usize = 2015;

    pub const SETTING_CONTROLS: [usize; 15] = [
        BACKEND_DIRECTORY,
        ISOLATION,
        TIMEOUT_SECONDS,
        MEMORY_LIMIT_MIB,
        MAX_ARCHIVE_BYTES,
        MAX_FILES,
        MAX_DIRECTORIES,
        MAX_TOTAL_BYTES,
        MAX_SINGLE_FILE_BYTES,
        MAX_DEPTH,
        MAX_PATH_BYTES,
        PRESERVE_MOTW,
        OPEN_AFTER_DOUBLE_CLICK,
        ENCODING,
        ATTACHMENT_HANDOFF,
    ];

    pub const ACTION_BUTTONS: [usize; 11] = [
        BACKEND_BROWSE,
        BACKEND_DOCTOR,
        BACKEND_IMPORT,
        BACKEND_MSYS2,
        REGISTER,
        UNREGISTER,
        DEFAULT_APPS,
        CONFIG_FOLDER,
        DEFAULTS,
        SAVE,
        CANCEL,
    ];

    pub fn is_setting(value: usize) -> bool {
        SETTING_CONTROLS.contains(&value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsAction {
    BrowseBackend,
    DiagnoseBackend,
    ImportBackendBundle,
    ImportMsys2Backend,
    RegisterAssociations,
    UnregisterAssociations,
    OpenDefaultApps,
    OpenConfigFolder,
    RestoreDefaults,
    Save,
    Cancel,
}

impl SettingsAction {
    pub const ALL: [Self; 11] = [
        Self::BrowseBackend,
        Self::DiagnoseBackend,
        Self::ImportBackendBundle,
        Self::ImportMsys2Backend,
        Self::RegisterAssociations,
        Self::UnregisterAssociations,
        Self::OpenDefaultApps,
        Self::OpenConfigFolder,
        Self::RestoreDefaults,
        Self::Save,
        Self::Cancel,
    ];

    pub const fn control_id(self) -> usize {
        match self {
            Self::BrowseBackend => control_id::BACKEND_BROWSE,
            Self::DiagnoseBackend => control_id::BACKEND_DOCTOR,
            Self::ImportBackendBundle => control_id::BACKEND_IMPORT,
            Self::ImportMsys2Backend => control_id::BACKEND_MSYS2,
            Self::RegisterAssociations => control_id::REGISTER,
            Self::UnregisterAssociations => control_id::UNREGISTER,
            Self::OpenDefaultApps => control_id::DEFAULT_APPS,
            Self::OpenConfigFolder => control_id::CONFIG_FOLDER,
            Self::RestoreDefaults => control_id::DEFAULTS,
            Self::Save => control_id::SAVE,
            Self::Cancel => control_id::CANCEL,
        }
    }

    pub fn from_control_id(value: usize) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|action| action.control_id() == value)
    }
}

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
    pub isolation: IsolationMode,
    pub max_archive_bytes: String,
    pub max_files: String,
    pub max_directories: String,
    pub max_total_bytes: String,
    pub max_single_file_bytes: String,
    pub max_depth: String,
    pub max_path_bytes: String,
    pub preserve_mark_of_the_web: bool,
    pub attachment_handoff: AttachmentHandoffPolicy,
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
            isolation: config.sandbox.isolation,
            max_archive_bytes: format_byte_count(config.limits.max_archive_bytes),
            max_files: config.limits.max_files.to_string(),
            max_directories: config.limits.max_directories.to_string(),
            max_total_bytes: format_byte_count(config.limits.max_total_bytes),
            max_single_file_bytes: format_byte_count(config.limits.max_single_file_bytes),
            max_depth: config.limits.max_depth.to_string(),
            max_path_bytes: config.limits.max_path_bytes.to_string(),
            preserve_mark_of_the_web: config.behavior.preserve_mark_of_the_web,
            attachment_handoff: config.behavior.attachment_handoff,
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
        config.sandbox.isolation = self.isolation;
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
        config.behavior.attachment_handoff = self.attachment_handoff;
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
