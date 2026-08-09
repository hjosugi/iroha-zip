# Changelog

## Unreleased

- Refactored backend-manifest parsing into a bounded, platform-neutral API suitable for deterministic tests and future fuzzing.
- Reject oversized manifests, excessive file records, invalid UTF-8, unsafe Windows names, non-normalized paths, ambiguous executables, duplicate paths, and malformed hashes before backend-tree verification.
- Added a versioned backend-manifest specification and regression coverage for valid and hostile parser inputs.

## 0.3.0 - 2026-08-09

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
