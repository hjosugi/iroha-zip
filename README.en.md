# iroha-zip

[日本語](README.md) | [English](README.en.md) | [Website](https://hjosugi.github.io/iroha-zip/en/)

iroha-zip is a Rust wrapper for extracting untrusted archives on Windows with minimal privileges and creating archives only from inspected regular files.

It does not attempt to reimplement every archive format in Rust. It runs a current libarchive / `bsdtar.exe` backend as a separate process inside an ephemeral AppContainer, then inspects inputs and outputs in Rust. Neither extraction nor creation runs `bsdtar.exe` directly with the user's normal privileges.

This is not a security-audited product. Version `v0.4.0` is a practical preview whose design and real-world behavior are still being validated.

## Download

Download the Windows x64 ZIP or individual executables from [GitHub Releases](https://github.com/hjosugi/iroha-zip/releases/latest). The current official binaries are unsigned. Verify their origin with `SHA256SUMS.txt` and the GitHub artifact attestation; see [About unsigned releases](docs/UNSIGNED_RELEASE.md).

The package does not bundle libarchive / `bsdtar.exe`. After the first launch, import a backend you trust in Settings.

## How extraction works

```text
Double-click archive.zip
    ↓
Create an ephemeral AppContainer with no capabilities or network access
    ↓
Copy the bsdtar bundle pinned by its SHA-256 manifest
    ↓
Extract into temporary AppContainer storage
    ↓
Monitor file count, directory count, and expanded size while extracting
    ↓
Recheck reparse points, links, ADS, reserved names, and resource limits
    ↓
Propagate Mark-of-the-Web to every published file
    ↓
If configured, hand off to Windows Attachment Services and recheck content and MotW
    ↓
Publish a same-named folder beside the archive
```

### How creation works

```text
Select a source directory
    ↓
Audit links, reparse points, ADS, hard links, and size limits
    ↓
Copy only audited regular files to isolated staging
    ↓
Have the parent create a bounded PAX snapshot, then remove the original staging copy
    ↓
Materialize the PAX tree in AppContainer and match it to the source fingerprint
    ↓
Discard and recopy the backend, then seal the materialized tree read-only to its Package SID
    ↓
Archive only relative `.` with the freshly SHA-256-verified bsdtar
    ↓
Monitor the output archive size
    ↓
Recheck the sealed materialized-source fingerprint
    ↓
List and re-extract the result in a second AppContainer and compare the complete tree
    ↓
Copy from the same verified handle into a new output file
```

Creation needs additional temporary disk space of roughly twice the source tree plus the output archive. This deliberate cost separates normal source files from a compromised backend and detects damaged output before publication.

## Supported formats

### Extraction

libarchive/bsdtar detects any format enabled by the selected build. Important examples include:

- ZIP / ZIPX
- 7z
- LHA / LZH
- RAR / RAR5
- TAR
- GZ / BZ2 / XZ / Zstandard
- UNIX compress `.Z`
- CAB and other formats enabled by the backend build

### Creation

- ZIP
- 7z
- TAR
- TAR.GZ

RAR and LZH creation are not implemented. RAR is proprietary and libarchive mainly supports reading it; this backend also does not target LZH creation.

## Japanese filename encoding

Archive flags and libarchive auto-detection are used by default. If an older Japanese ZIP or LZH archive is garbled, explicitly select CP932:

```powershell
iroha-zip.exe extract .\old-japanese.zip --encoding cp932
```

Available values are:

```text
auto
utf8
cp932
cp437
```

Perfect automatic detection is impossible when a ZIP does not record its filename encoding correctly. The Settings application therefore retains an explicit default encoding option for double-click extraction.

## Security design

iroha-zip fails closed on:

- implicit extraction or creation after AppContainer setup fails;
- `..`, absolute, drive-prefixed, or UNC names found by sandboxed preflight listing;
- symbolic links, junctions, and other reparse points;
- hard links and duplicate file identities;
- duplicate archive members and path aliases that differ only by case or separators;
- NTFS Alternate Data Streams;
- Windows reserved names such as `CON`, `NUL`, and `COM1`;
- trailing dots/spaces, colons, and other Windows-invalid characters;
- excessive file count, directory count, single-file size, total size, depth, or path length;
- backend executables or DLLs that do not match the SHA-256 manifest;
- unexpected, missing, or linked files in the backend bundle; and
- overwriting an existing extraction destination or output archive.

A Job Object restricts the backend to one process, enforces a memory limit, and terminates timeouts. Extracted files are never used directly from temporary storage: only inspected regular files are copied into a new partial folder, which is then renamed atomically. Windows tree audits enumerate names through parent directory handles opened without rename/delete sharing and compare directory identity during audit and copy. Creation similarly audits and duplicates the source, denies Package SID writes, then re-extracts the produced archive in a separate sandbox and requires a matching tree fingerprint. The explicit unsandboxed diagnostic path keeps before/after fingerprint detection but cannot apply the Windows DACL seal.

See the [threat model](docs/THREAT_MODEL.md). The differences between AppContainer and experimental LPAC, fail-closed rules, and unfinished validation are tracked in the [LPAC evaluation](docs/LPAC_EVALUATION.md). Automated Windows evidence and its limits are specified in [Windows E2E](docs/WINDOWS_E2E.md), and generated hostile fixtures are described in the [malicious corpus](docs/MALICIOUS_CORPUS.md).

## Runtime requirements

- Windows 10 or later; normal use targets Windows 11 x64
- A libarchive 3.8.9-series `bsdtar.exe` and its required DLLs
- PowerShell 5.1 or later

Building from source additionally requires Rust 1.97.1 and the MSVC C++ workload from Visual Studio Build Tools. `rust-toolchain.toml` pins Rust 1.97.1.

## Why the ZIP does not bundle bsdtar

The project avoids redistributing third-party executables without independently established provenance. Neither the source tree nor the official release package contains a backend binary.

Provide a libarchive build you trust and use **Import bundle** or **Import from MSYS2** in Settings. Import creates a SHA-256 manifest for the executable and every DLL, then verifies the complete installed tree. The included scripts expose the same flow for automation.

Example using MSYS2 UCRT64:

```powershell
# Run in an MSYS2 UCRT64 shell
pacman -S mingw-w64-ucrt-x86_64-libarchive
```

Then choose **Import from MSYS2** in Settings and select `C:\msys64`. The PowerShell equivalent is:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\export-msys2-backend.ps1 -Msys2Root C:\msys64
```

If you already have a minimal bsdtar bundle, choose **Import bundle**, or use:

```powershell
.\scripts\install-backend.ps1 `
  -SourceDirectory C:\path\to\minimal-bsdtar-bundle `
  -AllowUnsupportedSource
```

An arbitrary local bundle is an unsupported source whose publisher signature cannot be established by the importer. Settings requires a dedicated confirmation, and the CLI requires `-AllowUnsupportedSource`. Every payload directly under `SourceDirectory` or its descendants is pinned, so do not include unrelated executables or DLLs.

The [backend manifest specification](docs/BACKEND_MANIFEST.md) defines format, input limits, path rules, and verification coverage. The [backend evidence specification](docs/BACKEND_EVIDENCE.md) covers required MSYS2 signatures, unsupported-source warnings, machine-readable provenance, SPDX 2.3 SBOMs, license inventories, and fail-closed private packaging.

## Build

Run from a Developer PowerShell for VS 2022 or another PowerShell with the MSVC environment:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\build-release.ps1
```

The script runs:

```text
   cargo fmt --all -- --check
cargo test --all-targets
cargo test --features fuzzing --test fuzz_regressions
cargo clippy --all-targets
cargo build --release
```

Successful output is written to:

```text
dist\iroha-zip\
dist\iroha-zip-0.4.0-windows-x64.zip
```

Both the normal build and tag-driven release workflow produce unsigned binaries. Official releases attach a ZIP, three individual EXEs, and SHA-256 checksums, and publish a GitHub artifact attestation. See [About unsigned releases](docs/UNSIGNED_RELEASE.md) for SmartScreen and independent verification guidance. The strict verification path required for future Authenticode-signed releases remains documented in the [release verification specification](docs/RELEASE_VERIFICATION.md).

If `Cargo.lock` is initially absent, the script creates it. Review and commit it; all subsequent builds use `--locked`.

Official packages do not include third-party backends. To create a private package with an independently trusted backend, explicitly use:

```powershell
.\scripts\build-release.ps1 -IncludeBackend
```

`-IncludeBackend` requires supported-source evidence by default. Add `-AllowUnsupportedBackendSource` only when intentionally packaging an independently reviewed unsupported bundle.

## Settings and initial setup

Open Settings from the extracted package:

```powershell
.\iroha-zip.exe settings
# or .\iroha-zip-settings.exe
```

The application manages:

| Area | Controls |
|---|---|
| Backend | Destination, explicitly warned local import, signed MSYS2 collection, SHA-256/provenance/SPDX/license validation, AppContainer diagnosis |
| AppContainer | Normal AppContainer or experimental LPAC, timeout, Job Object memory limit |
| Resource limits | Input archive size, files, directories, total and single-file size, path depth and length |
| Extraction | Mark-of-the-Web propagation, open after double-click, default filename encoding |
| Windows trust handoff | Disabled (default), best-effort, or required Attachment Services handoff; never described as a clean verdict |
| Windows integration | Register/unregister association candidates, Default Apps, configuration folder |
| Configuration | Field validation, safe defaults, rollback-safe save |

`--allow-unsandboxed` is a noisy, per-command diagnostic exception and cannot be persisted.

Size fields accept readable binary units such as `16 GiB` and `512 MiB`. Validation focuses the invalid field. The title's `*` and a close confirmation expose unsaved changes. Tab/Shift+Tab, access keys, Enter-to-save, and Escape are supported. High-DPI scaling adds scrolling and focus tracking when necessary. Backend replacement, association removal, and restoring defaults require confirmation; long-running imports and diagnostics report status. Configuration writes are serialized and rollback-safe. The implemented contract and remaining physical-device matrix are in [Settings accessibility](docs/SETTINGS_ACCESSIBILITY.md).

The default configuration path is:

```text
%LOCALAPPDATA%\iroha-zip\config.toml
```

See [`config.example.toml`](config.example.toml) for every option.

When Windows trust handoff is enabled, iroha-zip calls `IAttachmentExecute::Save` for the partial tree, then rechecks SHA-256 content, tree structure, links/reparse points/ADS, and Mark-of-the-Web. `best-effort` reports service unavailability and continues; `required` refuses publication. Content changes or deletion always fail closed. See [Antimalware handoff](docs/ANTIMALWARE_HANDOFF.md).

CLI-only setup and diagnosis remain available:

```powershell
.\iroha-zip.exe init-config
.\iroha-zip.exe doctor
```

Windows requires the user to confirm default-app changes. Register iroha-zip as an association candidate in Settings, open **Default apps**, and assign ZIP, 7z, RAR, or other desired formats. Double-click then extracts beside the archive.

## CLI

### Preview and selective extraction

`preview` performs the same sandboxed extraction, timeout, resource, and path audit as normal extraction, but publishes nothing and prints only the audited tree. The main process does not parse an archive listing string.

```powershell
iroha-zip.exe preview .\archive.zip
iroha-zip.exe extract .\archive.zip --select "docs\readme.txt"
iroha-zip.exe extract .\archive.zip --select "photos" --select "docs\index.txt"
```

Repeat `--select` for preview-relative files or directories. Selection is never sent to the backend: iroha-zip audits the entire archive first, materializes the selection, re-audits that tree, and uses the normal partial/atomic publication path. An unsafe unselected entry still rejects the entire archive. See [Archive preview](docs/ARCHIVE_PREVIEW.md).

### Extract

```powershell
iroha-zip.exe extract .\archive.zip
iroha-zip.exe extract .\archive.zip --encoding cp932
iroha-zip.exe extract .\archive.7z --output D:\Extracted\archive
iroha-zip.exe extract .\archive.tar.gz --open
```

Existing destinations are never overwritten. Without `--output`, a collision-safe sibling directory is selected.

### Create

```powershell
iroha-zip.exe create zip .\folder .\folder.zip
iroha-zip.exe create seven-zip .\folder .\folder.7z
iroha-zip.exe create tar .\folder .\folder.tar
iroha-zip.exe create tar-gz .\folder .\folder.tar.gz
```

iroha-zip rejects output inside the source tree, source links or ADS, and overwrite attempts.

### Diagnose

```powershell
iroha-zip.exe doctor
```

This validates configuration, every backend hash, `bsdtar --version`, and AppContainer creation.

## Current limitations

- Password-protected archives are not supported. The stock Windows bsdtar constraints and a ConPTY design that avoids command-line secrets are tracked in [Encrypted archives](docs/ENCRYPTED_ARCHIVES.md).
- The CLI has policy-safe preview and selective extraction, but there is no native archive browsing/search/selection GUI.
- iroha-zip is not an antivirus engine and cannot promise that extracted executables are safe.
- It cannot guarantee protection from unknown vulnerabilities in AppContainer, the Windows kernel, or libarchive.
- Normal AppContainer is the default. Experimental LPAC must be selected explicitly and used only after `doctor` succeeds with the chosen backend. There is no silent compatibility downgrade.
- It cannot fully eliminate races against an attacker who already controls the same user account.
- The Linux suite, Clippy, Windows MSVC type checking, and five bounded fuzz targets cover manifests, Windows paths, archive names, Windows command lines, and configuration round trips. Server 2022/2025 E2E and generated malicious-corpus workflows exist, but they are not a substitute for Windows 10/11 device validation. See [Fuzzing](docs/FUZZING.md), [Windows E2E](docs/WINDOWS_E2E.md), [Malicious corpus](docs/MALICIOUS_CORPUS.md), and [Build status](docs/BUILD_STATUS.md).

Remaining work and acceptance criteria are tracked in [`docs/ISSUE_BACKLOG.md`](docs/ISSUE_BACKLOG.md). Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before proposing a change.

## License

iroha-zip is dual-licensed under MIT or Apache-2.0.

Imported libarchive binaries and DLLs remain subject to their distributors' licenses. If you redistribute a backend, independently verify the required notices for every dependency DLL.
