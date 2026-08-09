use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::error::{IrohaZipError, Result};
use crate::policy::Limits;
use crate::util;

pub const MIN_MEMORY_LIMIT_MIB: u64 = 64;
pub const MAX_MEMORY_LIMIT_MIB: u64 = 1_048_576;
pub const MIN_TIMEOUT_SECONDS: u64 = 1;
pub const MAX_TIMEOUT_SECONDS: u64 = 86_400;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub backend: BackendConfig,
    pub sandbox: SandboxConfig,
    pub limits: Limits,
    pub behavior: BehaviorConfig,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct BackendConfig {
    /// Directory containing backend-manifest.tsv and the pinned bsdtar bundle.
    /// Relative paths are resolved against iroha-zip.exe.
    pub directory: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SandboxConfig {
    pub timeout_seconds: u64,
    pub memory_limit_mib: u64,
    pub isolation: IsolationMode,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 300,
            memory_limit_mib: 768,
            isolation: IsolationMode::AppContainer,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum IsolationMode {
    #[default]
    AppContainer,
    Lpac,
}

impl IsolationMode {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::AppContainer => "AppContainer",
            Self::Lpac => "LPAC",
        }
    }

    pub fn is_lpac(self) -> bool {
        self == Self::Lpac
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct BehaviorConfig {
    pub preserve_mark_of_the_web: bool,
    pub attachment_handoff: AttachmentHandoffPolicy,
    pub open_after_double_click: bool,
    pub default_filename_encoding: FilenameEncoding,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            preserve_mark_of_the_web: true,
            attachment_handoff: AttachmentHandoffPolicy::Disabled,
            open_after_double_click: true,
            default_filename_encoding: FilenameEncoding::Auto,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AttachmentHandoffPolicy {
    #[default]
    Disabled,
    BestEffort,
    Required,
}

impl AttachmentHandoffPolicy {
    pub fn is_enabled(self) -> bool {
        self != Self::Disabled
    }

    pub fn is_required(self) -> bool {
        self == Self::Required
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum FilenameEncoding {
    #[default]
    Auto,
    Utf8,
    Cp932,
    Cp437,
}

impl FilenameEncoding {
    pub fn bsdtar_option(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Utf8 => Some("zip:hdrcharset=UTF-8,lha:hdrcharset=UTF-8"),
            Self::Cp932 => Some("zip:hdrcharset=CP932,lha:hdrcharset=CP932"),
            Self::Cp437 => Some("zip:hdrcharset=CP437,lha:hdrcharset=CP437"),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let input = fs::read(path)
            .map_err(|error| IrohaZipError::io_path("cannot read configuration", path, error))?;
        Self::parse(&input).map_err(|error| {
            IrohaZipError::Config(format!("cannot parse {}: {error}", path.display()))
        })
    }

    pub(crate) fn parse(input: &[u8]) -> Result<Self> {
        let text = std::str::from_utf8(input).map_err(|error| {
            IrohaZipError::Config(format!("configuration is not valid UTF-8: {error}"))
        })?;
        let config: Self = toml::from_str(text)
            .map_err(|error| IrohaZipError::Config(format!("invalid TOML: {error}")))?;
        config.validate()?;
        Ok(config)
    }

    pub fn write_default(path: &Path) -> Result<bool> {
        let _save_guard = crate::platform::lock_config_save()?;
        if path.exists() {
            return Ok(false);
        }
        let parent = configuration_parent(path);
        fs::create_dir_all(parent).map_err(|error| {
            IrohaZipError::io_path("cannot create configuration directory", parent, error)
        })?;
        let config = Self::default();
        let text = config.serialized()?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| IrohaZipError::io_path("cannot create configuration", path, error))?;
        file.write_all(text.as_bytes())
            .map_err(|error| IrohaZipError::io_path("cannot write configuration", path, error))?;
        file.sync_all()
            .map_err(|error| IrohaZipError::io_path("cannot flush configuration", path, error))?;
        Ok(true)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = self.serialized()?;
        let _save_guard = crate::platform::lock_config_save()?;
        let parent = configuration_parent(path);
        fs::create_dir_all(parent).map_err(|error| {
            IrohaZipError::io_path("cannot create configuration directory", parent, error)
        })?;

        let token = util::unique_token();
        let temporary = parent.join(format!(".iroha-zip-config-{token}.tmp"));
        let backup = parent.join(format!(".iroha-zip-config-{token}.bak"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| {
                IrohaZipError::io_path("cannot create temporary configuration", &temporary, error)
            })?;
        if let Err(error) = file
            .write_all(text.as_bytes())
            .and_then(|()| file.sync_all())
        {
            drop(file);
            let _ = fs::remove_file(&temporary);
            return Err(IrohaZipError::io_path(
                "cannot write temporary configuration",
                &temporary,
                error,
            ));
        }
        drop(file);

        let had_previous = path.exists();
        if had_previous {
            fs::rename(path, &backup).map_err(|error| {
                let _ = fs::remove_file(&temporary);
                IrohaZipError::io_path("cannot back up configuration", path, error)
            })?;
        }
        if let Err(error) = fs::rename(&temporary, path) {
            if had_previous {
                let _ = fs::rename(&backup, path);
            }
            let _ = fs::remove_file(&temporary);
            return Err(IrohaZipError::io_path(
                "cannot replace configuration",
                path,
                error,
            ));
        }
        if had_previous {
            let _ = fs::remove_file(backup);
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self
            .backend
            .directory
            .as_ref()
            .is_some_and(|path| path.as_os_str().is_empty())
        {
            return Err(config_error("backend directory must not be empty"));
        }
        if !(MIN_TIMEOUT_SECONDS..=MAX_TIMEOUT_SECONDS).contains(&self.sandbox.timeout_seconds) {
            return Err(config_error(format!(
                "sandbox timeout must be {MIN_TIMEOUT_SECONDS}..={MAX_TIMEOUT_SECONDS} seconds"
            )));
        }
        if !(MIN_MEMORY_LIMIT_MIB..=MAX_MEMORY_LIMIT_MIB).contains(&self.sandbox.memory_limit_mib) {
            return Err(config_error(format!(
                "sandbox memory limit must be {MIN_MEMORY_LIMIT_MIB}..={MAX_MEMORY_LIMIT_MIB} MiB"
            )));
        }

        let limits = &self.limits;
        for (name, value) in [
            ("maximum archive bytes", limits.max_archive_bytes),
            ("maximum files", limits.max_files),
            ("maximum directories", limits.max_directories),
            ("maximum total bytes", limits.max_total_bytes),
            ("maximum single-file bytes", limits.max_single_file_bytes),
        ] {
            if value == 0 {
                return Err(config_error(format!("{name} must be greater than zero")));
            }
        }
        if limits.max_depth == 0 {
            return Err(config_error("maximum path depth must be greater than zero"));
        }
        if limits.max_path_bytes == 0 {
            return Err(config_error(
                "maximum path byte length must be greater than zero",
            ));
        }
        if limits.max_single_file_bytes > limits.max_total_bytes {
            return Err(config_error(
                "maximum single-file bytes must not exceed maximum total bytes",
            ));
        }
        limits
            .max_total_bytes
            .checked_add(limits.max_single_file_bytes)
            .and_then(|value| value.checked_add(2 * 1024 * 1024))
            .ok_or_else(|| config_error("extraction byte limits are too large"))?;
        limits
            .max_files
            .checked_add(18)
            .ok_or_else(|| config_error("maximum file count is too large"))?;
        limits
            .max_directories
            .checked_add(12)
            .ok_or_else(|| config_error("maximum directory count is too large"))?;
        Ok(())
    }

    pub(crate) fn serialized(&self) -> Result<String> {
        self.validate()?;
        toml::to_string_pretty(self).map_err(|error| {
            IrohaZipError::Config(format!("cannot serialize configuration: {error}"))
        })
    }

    pub fn backend_directory(&self) -> Result<PathBuf> {
        let executable = env::current_exe()
            .map_err(|error| IrohaZipError::io("cannot locate iroha-zip executable", error))?;
        let executable_dir = executable.parent().ok_or_else(|| {
            IrohaZipError::Config("iroha-zip executable has no parent directory".to_owned())
        })?;
        let configured = self
            .backend
            .directory
            .clone()
            .unwrap_or_else(|| PathBuf::from("backend/libarchive"));
        let joined = if configured.is_absolute() {
            configured
        } else {
            executable_dir.join(configured)
        };
        Ok(joined)
    }
}

fn config_error(message: impl Into<String>) -> IrohaZipError {
    IrohaZipError::Config(message.into())
}

fn configuration_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

pub fn default_config_path() -> Result<PathBuf> {
    if cfg!(windows) {
        let base = env::var_os("LOCALAPPDATA")
            .ok_or_else(|| IrohaZipError::Config("LOCALAPPDATA is not defined".to_owned()))?;
        return Ok(PathBuf::from(base).join("iroha-zip").join("config.toml"));
    }

    if let Some(base) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(base).join("iroha-zip").join("config.toml"));
    }
    let home = env::var_os("HOME")
        .ok_or_else(|| IrohaZipError::Config("HOME is not defined".to_owned()))?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("iroha-zip")
        .join("config.toml"))
}
