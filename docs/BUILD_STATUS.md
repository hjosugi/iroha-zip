# Build and validation status

Updated: 2026-08-09

## Completed locally

- Rust 1.97.1 `cargo fmt --all -- --check`
- Rust 1.97.1 `cargo test --all-targets --locked` on Linux
- Rust 1.97.1 `cargo clippy --all-targets --locked` on Linux
- `cargo check --all-targets --target x86_64-pc-windows-msvc --locked`
- Configuration serialization, backward compatibility, validation, rollback-safe replacement, and path-policy tests
- Native settings application type-check against `windows` 0.62.2 APIs
- TOML and GitHub Actions workflow parsing
- `cargo-deny` advisory, license, ban, and source policy checks
- `cargo-about` third-party license inventory generation from the locked dependency graph
- PowerShell syntax review for backend import, association, and release scripts
- Release inventory policy: official packages do not bundle EXE, DLL, MSI, PDB, or a backend manifest

## Performed by GitHub Actions

The CI workflow runs formatting, tests, Clippy, and release builds on both `ubuntu-latest` and `windows-latest`. A `v*` tag whose value matches `Cargo.toml` builds the backend-free Windows x64 ZIP, writes its SHA-256 sidecar, and creates the GitHub release.

## Still requires a real Windows validation machine

- AppContainer profile creation and isolated `bsdtar` execution
- Settings-screen interaction, DPI, keyboard navigation, folder pickers, and Default Apps flow
- ZIP, 7z, RAR, LZH, TAR.GZ, and `.Z` extraction with a pinned libarchive bundle
- ZIP, 7z, TAR, and TAR.GZ creation and byte-for-byte content comparison after re-extraction
- Malicious archive regression corpus for traversal, links, junctions, ADS, hardlinks, and archive bombs
- Authenticode signing and independent security review

Until those Windows integration checks are complete, treat `v0.2.0` as a security-oriented preview rather than an audited security product.
