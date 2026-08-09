use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use crate::policy::Limits;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FileIdentity {
    pub volume: u64,
    pub index: u64,
}

#[derive(Clone, Debug)]
pub struct ProcessSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub current_dir: PathBuf,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
    pub timeout: Duration,
    pub monitor_root: Option<PathBuf>,
    pub limits: Limits,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessResult {
    pub exit_code: i32,
}

#[cfg(windows)]
mod windows_impl;
#[cfg(windows)]
pub use windows_impl::{
    Sandbox, file_identity, open_folder, read_mark_of_the_web, validate_directory_security,
    validate_extracted_entry_security, validate_regular_file_security, write_mark_of_the_web,
};

#[cfg(not(windows))]
mod generic;
#[cfg(not(windows))]
pub use generic::{
    Sandbox, file_identity, open_folder, read_mark_of_the_web, validate_directory_security,
    validate_extracted_entry_security, validate_regular_file_security, write_mark_of_the_web,
};
