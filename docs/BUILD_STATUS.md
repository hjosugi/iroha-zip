# Build and validation status

Updated: 2026-08-14

## Completed locally

- Rust 1.97.1 `cargo fmt --all -- --check`
- Rust 1.97.1 `cargo test --all-targets --locked` on Linux
- Rust 1.97.1 `cargo clippy --all-targets --locked` on Linux
- `cargo check --all-targets --target x86_64-pc-windows-msvc --locked`
- Configuration serialization, backward compatibility, validation, rollback-safe replacement, and path-policy tests
- Platform-neutral settings-form round trips, human-readable byte-unit parsing, and field-specific validation tests
- Platform-neutral 100–300% layout scaling, stable 26-control IDs, exhaustive 11-action dispatch mapping, and concurrent-save tests
- Shared archive staging, typed/sorted policy-safe preview inventory, fail-closed selective-tree materialization, and Unix fake-backend end-to-end orchestration tests
- Sandboxed, 64 MiB-bounded archive-member preflight with raw absolute/drive/UNC, Windows-name, depth/path, and case-alias duplicate rejection before extraction
- Deterministic source-generated ZIP/ustar/old-GNU sparse corpus with one control, 18 reject cases, and Windows-only hardlink/ADS/junction policy fixtures
- Bounded backend-manifest parsing with deterministic malformed-input, path, duplicate, and resource-limit regression tests
- Bounded backend provenance/SPDX/license parsers with exact manifest, ownership, relationship, package-verification-code, notice, license-file, and evidence-tree drift tests
- Generated unsupported-source SPDX JSON validated against the official SPDX v2.3 JSON Schema
- Five pinned fuzz targets, a successful initial bounded sanitizer campaign, isolated seeds, and deterministic minimized-regression promotion
- Handle-retaining input/source snapshots and deterministic source-tree race tests for identity replacement, same-size mutation, rename, hardlinks, and symbolic links
- Bounded Windows `GetFileInformationByHandleEx` directory enumeration, before/after directory-handle identity checks, and empty-directory replacement rejection between audit and copy
- Audited regular-object copy into a unique external creation tree; exact source-tree fingerprint comparison; bounded recursive per-object Package-SID read/execute-only sealing; trusted bounded PAX serialization; fixed `@source.pax.tar` conversion without Windows volume-root access; created-archive full-root re-extraction in a second sandbox; handle-retained create-new publication; deterministic root/mutation/mismatch/budget tests; and a local libarchive ZIP/7z/TAR/TAR.GZ/empty/Japanese-name round trip through the production path
- Recursive Package-SID read/execute-only sealing of each sandbox backend tree after UTF-8 manifest transformation and before launch
- Handle-pinned, recursively sealed sandbox archive copies; post-listing/post-extraction fingerprint checks; and extraction-directory create-new only after the listing child has exited and its output passed policy
- Windows raw-name listing through libarchive's UTF-8 pathname API in a dedicated child that loads only manifest-pinned DLL candidates after rechecking AppContainer/zero-capability isolation; fixed `asInvoker`/long-path/UTF-8 process manifests with byte-for-byte resource readback remain on disposable backend copies
- Native settings application type-check against `windows` 0.62.2 APIs
- Settings manifest XML and UI Automation PowerShell syntax parsing
- Safe UI Automation button paths for three folder-picker cancellations, Restore Defaults, and unsaved-change Cancel
- Serialized initial configuration creation plus a Windows-only independent-process, non-ASCII-path save regression
- Windows named-mutex timeout, post-release recovery, and abandoned-owner recovery unit tests
- Deterministic configuration replacement/restore failure injection with preserved recovery evidence
- Normal-AppContainer/LPAC configuration round trips and Windows LPAC process/token API type-check
- Machine-readable AppContainer token/capability, loopback-denial, timeout, memory-limit, process-temp, staging-source read/write ACL, and explicit five-profile/root-cleanup probe type-check
- Fixed-label Windows Server 2022/2025 archive, shell, settings-setup, and isolation E2E workflow parsing
- Windows Attachment Services COM boundary type-check, three-policy configuration/settings round trips, fail-closed post-handoff fingerprint tests, and partial cleanup tests
- TOML and GitHub Actions workflow parsing
- `cargo-deny` advisory, license, ban, and source policy checks
- `cargo-about` third-party license inventory generation from the locked dependency graph
- All PowerShell scripts parsed with the official PowerShell 7.6.4 parser; unsupported-source evidence generation, explicit-approval failure/rollback, Rust round-trip validation, strict-mode unsupported-source rejection, and notice-tamper rejection executed on Linux PowerShell
- Release inventory policy: official packages include the three iroha-zip executables but no third-party backend EXE/DLL, MSI, PDB, or backend manifest
- Tag/version/main-commit release gates, three standalone PE assets plus a complete ZIP, SHA-256 inventories, pinned GitHub artifact attestations, and exact pre-publication/post-publication asset verification
- Future split build/sign/package boundary with strict three-EXE Authenticode publisher/EKU/timestamp verification retained for an owner-configured signing identity
- Japanese and English static Pages rendered at desktop and mobile sizes, with all three public routes passing an automated WCAG 2 AA audit

The current Linux suite contains 95 passing default-feature tests plus one explicitly invoked system-libarchive compatibility test. The feature-gated minimized fuzz-regression gate brings the normal all-feature count to 96. Windows CI additionally runs source-file and directory-handle sharing tests that verify open snapshots block writes and renames, plus two independent processes saving one configuration path.

## Performed by GitHub Actions

The fast CI matrix runs formatting, tests, and Clippy on both `ubuntu-latest` and `windows-latest`, then builds the Windows settings binary in the debug profile for its 26-control UI Automation contract. Production-profile compilation and signed-MSYS2 backend work are intentionally kept out of that duplicate matrix path.

A separate fixed-label `windows-2022` / `windows-2025` x64 matrix builds the production release binaries once per supported Server image, exports the verified backend once per image, and produces machine-readable archive/isolation, malicious-corpus, and settings artifacts. The Server 2025 path reuses that already-exported backend to validate supported evidence, the explicit unsupported-bundle path, `--require-supported`, rollback, and evidence-tamper failures. This removes a redundant release build and signed-package export from the fast matrix without dropping a gate. The complete matrix passed on both images in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176) at commit `2d410f5f3eac3166b54808af83bcdc385470819b`. The downloaded reports were checked for AppContainer/zero-capability identity, loopback denial, timeout and memory rejection, one effective process-temp path, five explicit profile/root cleanups, four created formats, four additional read filters, Japanese and long paths, invalid-input non-publication, shell extraction, one benign plus 18 rejected generated archives, native hardlink/ADS/junction rejection, and the 26-control English settings save/diagnosis path. The Server matrix is not Windows 10/11 desktop evidence; see the [Windows E2E contract](WINDOWS_E2E.md) and [corpus contract](MALICIOUS_CORPUS.md).

A `vX.Y.Z` tag whose value matches `Cargo.toml` and points to the current `main` commit builds an unsigned Windows x64 package and three standalone executables and produces SHA-256 inventories and GitHub artifact attestations. Publication uses the repository immutable-release policy, creates a draft without overwriting an existing version, verifies all six remote assets against the local name/length/digest inventory, publishes it as latest, and requires immutable readback. An administrator must confirm the policy before tagging because GitHub does not expose its admin-only setting status to the standard Actions token. The policy applies to releases published after 2026-08-14; `v0.4.0` predates enforcement and remains mutable. The future Authenticode verification path remains available locally but is not active until the owner configures and independently reviews a signing identity. See [release verification](RELEASE_VERIFICATION.md).

## Still requires a real Windows validation machine

- Disposable Windows 10/11 x64 runs of the now-passing fixed-Server evidence contract
- LPAC `bsdtar --version`, archive-format, registry/file/COM denial, and network-denial measurements
- Settings-screen visual fit at 100–300% and mixed DPI, screen readers, folder pickers, Default Apps, external-state action rollback, and independent-process save contention
- Native archive-preview tree/search/selection UI, progress/cancellation accessibility, and real-format selected-publication matrix
- RAR/RAR5, LHA/LZH, CAB, ZIPX, and raw compressed-stream read fixtures with a pinned libarchive bundle
- Broader legacy-format, malformed-header, control-character, CPU-bomb, cancellation, and race fixtures beyond the passing generated corpus
- Long-running fuzz campaigns beyond the bounded weekly smoke schedule
- First independent inspection of the Windows-generated MSYS2 provenance, SPDX, and license evidence plus ongoing package-key rotation/archive-availability monitoring
- Real-Windows reparse point race stress tests and native child open relative to a parent handle
- Attachment Services with Defender enabled/disabled/unavailable, third-party providers, quarantine/deletion, ADS inventory, and MotW preservation across publication
- First successful reviewed Authenticode release after owner-managed signing identity validation, plus independent security review

Until those Windows integration checks are complete, treat `v0.4.0` as a security-oriented preview rather than an audited security product.
