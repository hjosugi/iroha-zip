# Build and validation status

Updated: 2026-08-09

## Completed locally

- Rust 1.97.1 `cargo fmt --all -- --check`
- Rust 1.97.1 `cargo test --all-targets --locked` on Linux
- Rust 1.97.1 `cargo clippy --all-targets --locked` on Linux
- `cargo check --all-targets --target x86_64-pc-windows-msvc --locked`
- Configuration serialization, backward compatibility, validation, rollback-safe replacement, and path-policy tests
- Platform-neutral settings-form round trips, human-readable byte-unit parsing, and field-specific validation tests
- Bounded backend-manifest parsing with deterministic malformed-input, path, duplicate, and resource-limit regression tests
- Handle-retaining input/source snapshots and deterministic source-tree race tests for identity replacement, same-size mutation, rename, hardlinks, and symbolic links
- Native settings application type-check against `windows` 0.62.2 APIs
- Normal-AppContainer/LPAC configuration round trips and Windows LPAC process/token API type-check
- TOML and GitHub Actions workflow parsing
- `cargo-deny` advisory, license, ban, and source policy checks
- `cargo-about` third-party license inventory generation from the locked dependency graph
- PowerShell syntax review for backend import, association, and release scripts
- Release inventory policy: official packages do not bundle EXE, DLL, MSI, PDB, or a backend manifest

The current Linux suite contains 40 passing tests. Windows CI additionally runs the source-handle sharing test that verifies open snapshots block writes and renames.

## Performed by GitHub Actions

The CI workflow runs formatting, tests, Clippy, and release builds on both `ubuntu-latest` and `windows-latest`. A `v*` tag whose value matches `Cargo.toml` builds the backend-free Windows x64 ZIP, writes its SHA-256 sidecar, and creates the GitHub release.

## Still requires a real Windows validation machine

- AppContainer profile creation and isolated `bsdtar` execution
- LPAC `bsdtar --version`, archive-format, registry/file/COM denial, and network-denial measurements
- Settings-screen interaction, DPI, keyboard navigation, folder pickers, and Default Apps flow
- ZIP, 7z, RAR, LZH, TAR.GZ, and `.Z` extraction with a pinned libarchive bundle
- ZIP, 7z, TAR, and TAR.GZ creation and byte-for-byte content comparison after re-extraction
- Malicious archive regression corpus for traversal, links, junctions, ADS, hardlinks, and archive bombs
- Real-Windows reparse point race stress tests, parent-directory handle-relative source enumeration, staging-tree sealing, and created-archive re-extraction comparison
- Authenticode signing and independent security review

Until those Windows integration checks are complete, treat `v0.3.1` as a security-oriented preview rather than an audited security product.
