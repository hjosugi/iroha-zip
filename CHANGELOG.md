# Changelog

## Unreleased

- Added fixed-label Windows Server 2022/2025 E2E jobs with machine-readable evidence for verified-backend setup, zero-capability AppContainer tokens, loopback denial, timeout, memory limits, cleanup, archive creation/preview/re-extraction, invalid input, shell invocation, and settings-screen setup.
- Require explicit AppContainer profile/root cleanup on successful create, extract, preview, shell, and doctor operations and on backend launch, timeout, resource, or nonzero-exit failures; cleanup failures are no longer silently hidden.
- Extended `doctor` and the dedicated isolation report with measured token flags and capability counts instead of reporting only the requested isolation mode.
- Added an opt-in zero-capability LPAC prototype, exposed in the settings screen and configuration file.
- Verify the created child token is an AppContainer and, when requested, a less-privileged AppContainer; unsupported or downgraded launches fail closed.
- Documented the fail-closed ConPTY design required for future encrypted-archive input without command-line, environment, log, or configuration exposure.
- Added opt-in Windows Attachment Services handoff policies (`disabled`, `best-effort`, and `required`) to configuration and the settings screen.
- Run enabled handoff on the staged tree before publication, then re-audit file identities, links, reparse points, ADS, content hashes, tree structure, and Mark-of-the-Web before the final rename.
- Report Attachment Services acceptance separately from structural validation and never describe it as a clean malware verdict.
- Added a System-DPI-aware, resizable settings layout with 100–300% logical scaling, horizontal/vertical scrolling, and automatic focus visibility.
- Assigned stable IDs and unique access keys to every setting and action, with exhaustive action dispatch tests and a native Windows UI Automation smoke test.
- Serialize concurrent configuration saves before rollback-safe replacement, using a named Windows mutex across processes in the current session.
- Include every document linked from the packaged README in release ZIPs.
- Refactored extraction into a shared staging boundary used by normal extraction and policy-safe preview.
- Added a `preview` CLI that inventories only a fully extracted, audited, fingerprint-stable temporary tree and publishes nothing.
- Added repeatable `extract --select PATH` filtering after the complete archive audit, followed by selected-tree and source-tree revalidation before the existing atomic publication boundary.
- Make verified backend copies owner-readable/executable on Unix so the explicit unsandboxed integration-test path can exercise real process orchestration.
- Added five pinned `cargo-fuzz` targets, a bounded weekly sanitizer workflow, seed isolation, failure artifacts, and SHA-256-addressed deterministic regression promotion.
- Refactored Windows command-line quoting into a platform-neutral, property-tested encoder that rejects interior NUL, invalid program names, and the `CreateProcessW` length overflow before launch.
- Archive-derived destination names now fall back to `archive` when extension removal would produce a Windows-invalid or reserved component.
- Added atomic backend provenance bundles containing a strict source record, exact payload ownership, SPDX 2.3 SBOM, license inventory, notices, and copied license evidence.
- The supported MSYS2 UCRT64 exporter now refreshes isolated signed databases under `Required TrustedOnly`, records package signatures/archive hashes, verifies current versions, and compares imported bytes with signed package archives.
- Arbitrary backend bundles now require explicit unsupported-source approval; diagnostics warn permanently, and private backend packaging rejects unsupported or inconsistent evidence by default.

## 0.3.1 - 2026-08-09

- Added handle-retaining SHA-256 snapshots for input archives and compression source files.
- On Windows, deny write/delete sharing while source handles are retained and reject reparse points and multi-link files through the open handle.
- On Unix, use `O_NOFOLLOW` for source snapshots and reject multi-link files through handle metadata.
- Replace size-only source-tree comparisons with deterministic path, type, length, and content fingerprints.
- Reject identity replacement, timestamp/length changes, same-size content mutation, links, and source/target root escapes during audited copy, removing partial output on failure.
- Added deterministic full-copy race injection tests and snapshot regression tests for replacement, same-size mutation, rename, hardlinks, and symbolic links.

## 0.3.0 - 2026-08-09

- Refactored backend-manifest parsing into a bounded, platform-neutral API suitable for deterministic tests and future fuzzing.
- Reject oversized manifests, excessive file records, invalid UTF-8, unsafe Windows names, non-normalized paths, ambiguous executables, duplicate paths, and malformed hashes before backend-tree verification.
- Added a versioned backend-manifest specification and regression coverage for valid and hostile parser inputs.
- Unified the package, application, service, executable, configuration, Windows integration, sandbox, backend manifest, and release artifact names under `iroha-zip`.
- Renamed the Rust library crate to `iroha_zip` and its public error type to `IrohaZipError`.
- Moved the default configuration directory to `%LOCALAPPDATA%\iroha-zip` on Windows and the corresponding `iroha-zip` directory on other platforms.
- Renamed the CLI, shell integration, and settings executables to `iroha-zip.exe`, `iroha-zip-shell.exe`, and `iroha-zip-settings.exe`.

## 0.2.1 - 2026-08-09

- Refactored settings form conversion and validation into a platform-neutral, unit-tested module.
- Added human-readable binary size units, field-specific validation focus, and lossless config round trips.
- Added unsaved-change tracking, close confirmation, progress states, destructive-action confirmation, and keyboard dialog navigation to the settings screen.
- Linked the documented security, QA, platform, operations, and UX backlog to GitHub issues.

## 0.2.0 - 2026-08-09

- Added a native Windows settings application covering every configuration field.
- Added backend folder selection, verified bundle import, MSYS2 import, and full diagnostics to the settings screen.
- Added file-association registration/removal and Windows Default Apps shortcuts to the settings screen.
- Added a configurable default filename encoding for shell and CLI extraction, with per-command override.
- Added validated, rollback-safe configuration replacement and expanded configuration tests.
- Added reproducible backend-free Windows release packages and tag-driven GitHub releases.
- Added a checked dependency policy and generated third-party license inventory to release packages.
- Added `iroha-zip settings` as the supported settings entry point.

## 0.1.0 - 2026-08-08

- Initial Rust implementation.
- Ephemeral Windows AppContainer execution for both extraction and archive creation.
- Job Object limits for memory, process count, and timeout.
- Live full-sandbox filesystem monitoring with baseline-aware file, directory, and byte budgets.
- SHA-256-pinned libarchive backend bundle.
- ZIP/LHA filename encoding override for UTF-8, CP932, and CP437.
- Path, link, reparse point, ADS, hardlink, Windows device-name, and archive-bomb checks.
- Mark-of-the-Web propagation.
- ZIP, 7z, TAR, and TAR.GZ creation.
- Per-user Windows file-association scripts.
