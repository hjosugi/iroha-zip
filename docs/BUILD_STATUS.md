# Build and validation status

Updated: 2026-08-10

## Completed locally

- Rust 1.97.1 `cargo fmt --all -- --check`
- Rust 1.97.1 `cargo test --all-targets --locked` on Linux
- Rust 1.97.1 `cargo clippy --all-targets --locked` on Linux
- `cargo check --all-targets --target x86_64-pc-windows-msvc --locked`
- Configuration serialization, backward compatibility, validation, rollback-safe replacement, and path-policy tests
- Platform-neutral settings-form round trips, human-readable byte-unit parsing, and field-specific validation tests
- Platform-neutral 100–300% layout scaling, stable 26-control IDs, exhaustive 11-action dispatch mapping, and concurrent-save tests
- Shared archive staging, typed/sorted policy-safe preview inventory, fail-closed selective-tree materialization, and Unix fake-backend end-to-end orchestration tests
- Bounded backend-manifest parsing with deterministic malformed-input, path, duplicate, and resource-limit regression tests
- Bounded backend provenance/SPDX/license parsers with exact manifest, ownership, relationship, package-verification-code, notice, license-file, and evidence-tree drift tests
- Generated unsupported-source SPDX JSON validated against the official SPDX v2.3 JSON Schema
- Five pinned fuzz targets, a successful initial bounded sanitizer campaign, isolated seeds, and deterministic minimized-regression promotion
- Handle-retaining input/source snapshots and deterministic source-tree race tests for identity replacement, same-size mutation, rename, hardlinks, and symbolic links
- Native settings application type-check against `windows` 0.62.2 APIs
- Settings manifest XML and UI Automation PowerShell syntax parsing
- Normal-AppContainer/LPAC configuration round trips and Windows LPAC process/token API type-check
- Windows Attachment Services COM boundary type-check, three-policy configuration/settings round trips, fail-closed post-handoff fingerprint tests, and partial cleanup tests
- TOML and GitHub Actions workflow parsing
- `cargo-deny` advisory, license, ban, and source policy checks
- `cargo-about` third-party license inventory generation from the locked dependency graph
- All PowerShell scripts parsed with the official PowerShell 7.6.4 parser; unsupported-source evidence generation, explicit-approval failure/rollback, Rust round-trip validation, strict-mode unsupported-source rejection, and notice-tamper rejection executed on Linux PowerShell
- Release inventory policy: official packages do not bundle EXE, DLL, MSI, PDB, or a backend manifest

The current Linux suite contains 66 passing tests, plus the feature-gated minimized fuzz-regression gate. Windows CI additionally runs the source-handle sharing test that verifies open snapshots block writes and renames.

## Performed by GitHub Actions

The CI workflow runs formatting, tests, Clippy, and release builds on both `ubuntu-latest` and `windows-latest`. Windows CI also exports the current signed MSYS2 UCRT64 backend, validates the supported evidence, exercises the explicit unsupported-bundle path, and proves both `--require-supported` and evidence-tamper failures. It then exercises all 26 native settings controls through UI Automation. These new Windows evidence/UI steps have been parsed locally but have not yet produced a GitHub Actions result for the current local branch. A `v*` tag whose value matches `Cargo.toml` builds the backend-free Windows x64 ZIP, writes its SHA-256 sidecar, and creates the GitHub release.

## Still requires a real Windows validation machine

- AppContainer profile creation and isolated `bsdtar` execution
- LPAC `bsdtar --version`, archive-format, registry/file/COM denial, and network-denial measurements
- Settings-screen visual fit at 100–300% and mixed DPI, screen readers, folder pickers, Default Apps, external-state action rollback, and independent-process save contention
- Native archive-preview tree/search/selection UI, progress/cancellation accessibility, and real-format selected-publication matrix
- ZIP, 7z, RAR, LZH, TAR.GZ, and `.Z` extraction with a pinned libarchive bundle
- ZIP, 7z, TAR, and TAR.GZ creation and byte-for-byte content comparison after re-extraction
- Malicious archive regression corpus for traversal, links, junctions, ADS, hardlinks, and archive bombs
- Long-running fuzz campaigns beyond the bounded weekly smoke schedule
- First independent inspection of the Windows-generated MSYS2 provenance, SPDX, and license evidence plus ongoing package-key rotation/archive-availability monitoring
- Real-Windows reparse point race stress tests, parent-directory handle-relative source enumeration, staging-tree sealing, and created-archive re-extraction comparison
- Attachment Services with Defender enabled/disabled/unavailable, third-party providers, quarantine/deletion, ADS inventory, and MotW preservation across publication
- Authenticode signing and independent security review

Until those Windows integration checks are complete, treat `v0.3.1` as a security-oriented preview rather than an audited security product.
