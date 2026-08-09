#![deny(unsafe_code)]

pub mod backend;
pub mod cli;
pub mod config;
pub mod create;
pub mod error;
pub mod extract;
pub mod monitor;
pub mod platform;
pub mod policy;
pub mod settings;
pub mod transfer;
pub mod util;

use std::path::{Path, PathBuf};

use backend::BackendBundle;
use config::Config;
use error::Result;
use extract::ExtractRequest;

pub fn load_config(path: &Path) -> Result<Config> {
    Config::load(path)
}

pub fn verify_backend(config: &Config) -> Result<BackendBundle> {
    let directory = config.backend_directory()?;
    BackendBundle::verify(&directory)
}

pub fn shell_extract(archive: &Path, config_path: &Path) -> Result<PathBuf> {
    let config = load_config(config_path)?;
    let backend = verify_backend(&config)?;
    extract::extract(ExtractRequest {
        backend: &backend,
        config: &config,
        archive,
        output: None,
        encoding: config.behavior.default_filename_encoding,
        open: config.behavior.open_after_double_click,
        allow_unsandboxed: false,
    })
}
