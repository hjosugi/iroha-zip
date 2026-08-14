use std::net::SocketAddr;
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
    /// Inspect the policy-safe tree that would be published, without publishing it.
    Preview {
        /// Archive to inspect.
        archive: PathBuf,

        /// Override the ZIP/LHA filename encoding.
        #[arg(long, value_enum)]
        encoding: Option<FilenameEncoding>,

        /// Explicitly permit preview without `AppContainer` isolation.
        /// This is required on non-Windows platforms and is intentionally noisy.
        #[arg(long)]
        allow_unsandboxed: bool,
    },

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

        /// Publish only this preview-relative file or directory. Repeat for multiple paths.
        #[arg(long = "select", value_name = "PATH")]
        select: Vec<PathBuf>,

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

    /// Validate backend provenance, SPDX SBOM, and license evidence.
    VerifyBackendEvidence {
        /// Backend directory containing backend-manifest.tsv.
        backend: PathBuf,

        /// Reject evidence from an explicitly unsupported source.
        #[arg(long)]
        require_supported: bool,
    },

    /// Measure the selected AppContainer/LPAC token, capabilities, network denial, and cleanup.
    IsolationReport,

    #[command(hide = true)]
    InternalNetworkProbe { endpoint: SocketAddr },

    #[command(hide = true)]
    InternalSleepProbe { milliseconds: u64 },

    #[command(hide = true)]
    InternalMemoryProbe { bytes: u64 },

    #[command(hide = true)]
    InternalProcessTempProbe,

    #[command(hide = true)]
    InternalStagingWriteProbe { root: PathBuf },

    #[command(hide = true)]
    InternalArchiveListing {
        backend_root: PathBuf,
        candidates: PathBuf,
        archive: PathBuf,

        #[arg(long, value_enum)]
        encoding: FilenameEncoding,

        #[arg(long)]
        max_entries: u64,

        #[arg(long)]
        max_path_bytes: usize,

        #[arg(long)]
        allow_unsandboxed: bool,
    },

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
