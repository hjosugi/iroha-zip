use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

pub use crate::config::FilenameEncoding;

#[derive(Debug, Parser)]
#[command(
    name = "iroha-zip",
    version,
    about = "Safely extract archives through a pinned bsdtar backend",
    long_about = None
)]
pub struct Cli {
    /// Use a specific configuration file.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Extract an archive to a new directory.
    Extract {
        /// Archive to extract.
        archive: PathBuf,

        /// Destination directory. If omitted, a collision-safe sibling folder is created.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Override the ZIP/LHA filename encoding.
        #[arg(long, value_enum)]
        encoding: Option<FilenameEncoding>,

        /// Open the destination in Explorer after successful extraction.
        #[arg(long)]
        open: bool,

        /// Explicitly permit extraction without `AppContainer` isolation.
        /// This is required on non-Windows platforms and is intentionally noisy.
        #[arg(long)]
        allow_unsandboxed: bool,
    },

    /// Create ZIP, 7z, TAR, or TAR.GZ from one source directory.
    Create {
        #[arg(value_enum)]
        format: CreateFormat,

        /// Directory whose contents will be archived.
        source: PathBuf,

        /// Output archive path.
        output: PathBuf,

        /// Explicitly permit creation without `AppContainer` isolation.
        /// This is required on non-Windows platforms and is intentionally noisy.
        #[arg(long)]
        allow_unsandboxed: bool,
    },

    /// Validate configuration and the pinned backend bundle.
    Doctor,

    /// Write a default configuration file if one does not exist.
    InitConfig,

    /// Print the resolved configuration file path.
    ConfigPath,

    /// Open the graphical settings and setup screen.
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CreateFormat {
    Zip,
    SevenZip,
    Tar,
    TarGz,
}

impl CreateFormat {
    pub fn expected_extension(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::SevenZip => "7z",
            Self::Tar => "tar",
            Self::TarGz => "tar.gz",
        }
    }
}
