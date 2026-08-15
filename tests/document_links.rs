use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

fn collect_markdown(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot enumerate {}: {error}", directory.display()))
        .map(|entry| entry.expect("repository entry must be readable"))
        .collect::<Vec<_>>();
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let name = entry.file_name();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", entry.path().display()));
        if file_type.is_dir() {
            if [
                OsStr::new(".git"),
                OsStr::new(".codex"),
                OsStr::new("dist"),
                OsStr::new("node_modules"),
                OsStr::new("target"),
            ]
            .contains(&name.as_os_str())
            {
                continue;
            }
            collect_markdown(&entry.path(), files);
        } else if entry.path().extension() == Some(OsStr::new("md")) {
            assert!(
                file_type.is_file() && !file_type.is_symlink(),
                "Markdown document must be a regular non-link file: {}",
                entry.path().display()
            );
            files.push(entry.path());
        }
    }
}

fn line_destinations(line: &str) -> Vec<&str> {
    let mut destinations = Vec::new();
    let mut remainder = line;
    while let Some(start) = remainder.find("](") {
        remainder = &remainder[start + 2..];
        let Some(end) = remainder.find(')') else {
            break;
        };
        destinations.push(&remainder[..end]);
        remainder = &remainder[end + 1..];
    }

    let trimmed = line.trim_start();
    if trimmed.starts_with('[')
        && let Some(boundary) = trimmed.find("]:")
    {
        let destination = trimmed[boundary + 2..].trim_start();
        if !destination.is_empty() {
            destinations.push(destination);
        }
    }
    destinations
}

fn destination_value(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(value) = trimmed.strip_prefix('<') {
        return value
            .split_once('>')
            .map_or(value, |(destination, _)| destination);
    }
    trimmed.split_ascii_whitespace().next().unwrap_or("")
}

fn exact_local_path(repository: &Path, base: &Path, relative: &str) -> Result<PathBuf, String> {
    let mut resolved = base.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() || !resolved.starts_with(repository) {
                    return Err(format!("local link escapes the repository: {relative}"));
                }
            }
            Component::Normal(expected) => {
                let actual = fs::read_dir(&resolved)
                    .map_err(|error| format!("cannot enumerate {}: {error}", resolved.display()))?
                    .filter_map(Result::ok)
                    .find(|entry| entry.file_name() == expected)
                    .ok_or_else(|| {
                        format!(
                            "path component is missing or has different case under {}: {}",
                            resolved.display(),
                            expected.to_string_lossy()
                        )
                    })?;
                resolved = actual.path();
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(format!("local link must be relative: {relative}"));
            }
        }
    }
    if !resolved.starts_with(repository) {
        return Err(format!("local link escapes the repository: {relative}"));
    }
    Ok(resolved)
}

fn heading_fragments(markdown: &str) -> BTreeSet<String> {
    let mut fragments = BTreeSet::new();
    let mut fenced = false;
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let hashes = trimmed.bytes().take_while(|value| *value == b'#').count();
        if !(1..=6).contains(&hashes) || trimmed.as_bytes().get(hashes) != Some(&b' ') {
            continue;
        }
        let heading = trimmed[hashes + 1..].trim().trim_end_matches('#').trim();
        let mut fragment = String::new();
        let mut previous_hyphen = false;
        for character in heading.chars().flat_map(char::to_lowercase) {
            if character.is_whitespace() {
                if !previous_hyphen {
                    fragment.push('-');
                    previous_hyphen = true;
                }
            } else if character.is_alphanumeric() || character == '-' || character == '_' {
                fragment.push(character);
                previous_hyphen = character == '-';
            }
        }
        fragments.insert(fragment);
    }
    fragments
}

#[test]
fn repository_markdown_has_exact_local_links_and_anchors() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut markdown_files = Vec::new();
    collect_markdown(repository, &mut markdown_files);
    assert!(
        markdown_files.len() >= 33,
        "Markdown discovery unexpectedly shrank: {} files",
        markdown_files.len()
    );

    let mut local_links = 0_usize;
    let mut local_anchors = 0_usize;
    for document in markdown_files {
        let markdown = fs::read_to_string(&document)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", document.display()));
        let mut fenced = false;
        for (line_index, line) in markdown.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                fenced = !fenced;
                continue;
            }
            if fenced {
                continue;
            }

            for raw_destination in line_destinations(line) {
                let destination = destination_value(raw_destination);
                if destination.is_empty()
                    || destination.starts_with("https://")
                    || destination.starts_with("http://")
                    || destination.starts_with("mailto:")
                {
                    continue;
                }
                let (relative, fragment) = destination
                    .split_once('#')
                    .map_or((destination, None), |(path, value)| (path, Some(value)));
                let target = if relative.is_empty() {
                    document.clone()
                } else {
                    exact_local_path(
                        repository,
                        document.parent().expect("document must have a parent"),
                        relative,
                    )
                    .unwrap_or_else(|error| {
                        panic!(
                            "{}:{} has invalid local link {destination:?}: {error}",
                            document.display(),
                            line_index + 1
                        )
                    })
                };
                assert!(
                    target.is_file(),
                    "{}:{} local link is not a file: {destination}",
                    document.display(),
                    line_index + 1
                );
                local_links += 1;

                if let Some(fragment) = fragment.filter(|value| !value.is_empty())
                    && target.extension() == Some(OsStr::new("md"))
                {
                    let target_markdown = fs::read_to_string(&target).unwrap_or_else(|error| {
                        panic!("cannot read linked Markdown {}: {error}", target.display())
                    });
                    assert!(
                        heading_fragments(&target_markdown).contains(&fragment.to_lowercase()),
                        "{}:{} local anchor does not exist: {destination}",
                        document.display(),
                        line_index + 1
                    );
                    local_anchors += 1;
                }
            }
        }
    }

    assert!(
        local_links >= 100,
        "local-link discovery unexpectedly shrank: {local_links}"
    );
    assert!(
        local_anchors >= 6,
        "local-anchor discovery unexpectedly shrank: {local_anchors}"
    );
}
