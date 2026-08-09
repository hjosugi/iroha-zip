# Changelog

## Unreleased

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
- Added `safearc settings` as the supported settings entry point.

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
