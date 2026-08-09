use std::path::{Path, PathBuf};

use crate::backend::BackendBundle;
use crate::cli::FilenameEncoding;
use crate::config::Config;
use crate::error::{IrohaZipError, Result};
use crate::{policy, selection, staging, transfer, util};

pub struct ExtractResult {
    pub destination: PathBuf,
    pub attachment_handoff: transfer::AttachmentHandoffOutcome,
}

pub struct ExtractRequest<'a> {
    pub backend: &'a BackendBundle,
    pub config: &'a Config,
    pub archive: &'a Path,
    pub output: Option<&'a Path>,
    pub encoding: FilenameEncoding,
    pub selections: &'a [PathBuf],
    pub open: bool,
    pub allow_unsandboxed: bool,
}

pub fn extract(request: ExtractRequest<'_>) -> Result<ExtractResult> {
    let archive_snapshot = policy::open_input_archive(request.archive, &request.config.limits)?;
    let archive = archive_snapshot.path().to_path_buf();
    let destination = request
        .output
        .map(Path::to_path_buf)
        .map_or_else(|| util::smart_destination(&archive), Ok)?;
    if destination.exists() {
        return Err(IrohaZipError::Usage(format!(
            "refusing to overwrite existing destination: {}",
            destination.display()
        )));
    }

    let motw = if request.config.behavior.preserve_mark_of_the_web {
        crate::platform::read_mark_of_the_web(&archive)?
    } else {
        None
    };

    let staged = staging::stage_archive(
        request.backend,
        request.config,
        archive_snapshot,
        request.encoding,
        request.allow_unsandboxed,
    )?;
    let selected_root = staged.workspace_root().join("selected");
    let (publish_root, summary) = if request.selections.is_empty() {
        (staged.payload_root(), staged.summary().clone())
    } else {
        let summary = selection::materialize_selection(
            staged.payload_root(),
            &selected_root,
            request.selections,
            &request.config.limits,
        )?;
        (selected_root.as_path(), summary)
    };
    let published = transfer::commit_tree(
        publish_root,
        &destination,
        motw.as_deref(),
        request.config.behavior.attachment_handoff,
        &request.config.limits,
    )?;

    if request.open {
        crate::platform::open_folder(&published.destination)?;
    }

    eprintln!(
        "extracted {} files ({} bytes) to {}",
        summary.files,
        summary.total_bytes,
        published.destination.display()
    );
    Ok(ExtractResult {
        destination: published.destination,
        attachment_handoff: published.attachment_handoff,
    })
}
