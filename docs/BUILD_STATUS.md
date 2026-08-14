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
- Windows standalone GZ/BZ2/XZ/Zstandard/compress handling through libarchive's official raw reader in two fresh AppContainer passes; the outer extension fixes both the expected filter and safe output name, preflight drains the complete stream, extraction is create-new and bounded by the single-file limit, and mismatch/error cleanup removes partial output. Platform-neutral naming/CLI tests, x64/ARM64 cross-target checks, and a direct libarchive 3.8.9 five-filter API proof pass locally.
- Native settings application type-check against `windows` 0.62.2 APIs
- Settings manifest XML and UI Automation PowerShell syntax parsing
- Safe UI Automation button paths for three folder-picker cancellations, Restore Defaults, and unsaved-change Cancel
- Serialized initial configuration creation plus a Windows-only independent-process, non-ASCII-path save regression
- Windows named-mutex timeout, post-release recovery, and abandoned-owner recovery unit tests
- Deterministic configuration replacement/restore failure injection with preserved recovery evidence
- Normal-AppContainer/LPAC configuration round trips and Windows LPAC process/token API type-check
- Suspended Windows child creation with positive token/mode/capability verification before the single resume; a forced two-second verification failure requires zero child stdout
- Machine-readable AppContainer token/capability, loopback-denial, timeout, memory-limit, abnormal-exit, corrupt-loader, process-temp, staging-source read/write ACL, and explicit seven-profile/root-cleanup probe type-check
- Fixed-label Windows Server 2022/2025 archive, shell, settings-setup, and isolation E2E workflow parsing
- Windows Attachment Services COM boundary type-check, three-policy configuration/settings round trips, fail-closed post-handoff fingerprint tests, and partial cleanup tests
- TOML and GitHub Actions workflow parsing
- `cargo-deny` advisory, license, ban, and source policy checks
- `cargo-about` third-party license inventory generation from the locked dependency graph
- All PowerShell scripts parsed with the official PowerShell 7.6.4 parser; unsupported-source evidence generation, explicit-approval failure/rollback, Rust round-trip validation, strict-mode unsupported-source rejection, and notice-tamper rejection executed on Linux PowerShell
- Release inventory policy: official packages include the three iroha-zip executables but no third-party backend EXE/DLL, MSI, PDB, or backend manifest
- Tag/version/main-commit release gates, native x64/ARM64 builds, architecture-separated ZIPs and six standalone PE assets, exact PE-machine checks inside and outside both ZIPs, SHA-256 inventories, pinned GitHub artifact attestations, and exact 11-asset pre-publication/post-publication verification
- Future split build/sign/package boundary with strict three-EXE Authenticode publisher/EKU/timestamp verification retained for an owner-configured signing identity
- Japanese and English static Pages rendered at desktop and mobile sizes, with all three public routes passing an automated WCAG 2 AA audit

The current Linux suite contains 108 passing default-feature tests, including the package-version/document and bilingual Pages version/topology contracts, plus one explicitly invoked system-libarchive compatibility test. The feature-gated minimized fuzz-regression gate adds one passing deterministic test to the normal all-target run. Windows CI additionally runs source-file and directory-handle sharing tests that verify open snapshots block writes and renames, a rejected-child-stays-suspended regression, two independent processes saving one configuration path, and a real per-user association round trip that preserves unrelated registry state and each protected `UserChoice` snapshot.

## Performed by GitHub Actions

The fast CI matrix runs formatting, tests, and Clippy on both `ubuntu-latest` and `windows-latest`, then builds the Windows settings binary in the debug profile for its 26-control UI Automation contract and disposable association-state round trip. The matrix also injects backend-import failure immediately after the prior tree is renamed to backup and requires byte-identical restoration, zero transaction residue, and a subsequent successful import. These gates passed for the exact `v0.5.2` tag commit `a8ff7e0f30c33131e67e957226f26ba8faf4b214` in [Actions run 31784888606](https://github.com/hjosugi/iroha-zip/actions/runs/31784888606), after the same commit passed on `main` in [run 31783823846](https://github.com/hjosugi/iroha-zip/actions/runs/31783823846). All 26 external action uses are pinned to exact 40-hex commits, and the repository Actions policy now rejects non-SHA references while retaining read-only default workflow permissions. Production-profile compilation and signed-MSYS2 backend work are intentionally kept out of that duplicate matrix path.

The native GitHub `windows-11-arm` job requires OS/process architecture `Arm64` and Rust host `aarch64-pc-windows-msvc`, verifies all three application PEs and all backend EXE/DLL payloads as machine `0xAA64`, and exports a signature-verified MSYS2 CLANGARM64 backend. It runs the complete create/read matrix, malicious corpus, shell, Japanese and English Settings, normal-AppContainer schema-v4 isolation, and exact LPAC fail-closed branch. All five downloaded JSON reports from commit `27610e69f21bf85709f70a68695acc1113d22dca` in [Actions run 31778764604](https://github.com/hjosugi/iroha-zip/actions/runs/31778764604), independently repeated on push in [run 31778405711](https://github.com/hjosugi/iroha-zip/actions/runs/31778405711), record the matrices as executed, normal AppContainer with zero capabilities, seven successful profile/root cleanups, four create formats, 14 exact read fixtures, three raw-stream rejection cases, one control plus 18 hostile rejects, standalone-raw shell dispatch, and complete temporary-root removal. The read set covers four filtered TARs, five standalone streams, Microsoft LZX CAB, and pinned official libarchive 3.8.9 RAR/RAR5/LHA/ZIPX fixtures. The LPAC query returned the exact `ERROR_INVALID_PARAMETER` class and did not fall back. See [the ARM64 boundary and setup](ARM64.md).

A separate fixed-label `windows-2022` / `windows-2025` x64 matrix builds the production release binaries once per supported Server image, exports the verified backend once per image, and produces machine-readable archive/isolation, malicious-corpus, and settings artifacts. The Server 2025 path reuses that backend to validate supported evidence, the explicit unsupported-bundle path, `--require-supported`, rollback, and evidence-tamper failures. The expanded schema-v4 matrix passed on both images from commit `27610e69f21bf85709f70a68695acc1113d22dca` in [Actions run 31778764604](https://github.com/hjosugi/iroha-zip/actions/runs/31778764604). Independently downloaded reports cover four created formats and the same 14 exact read fixtures as ARM64: four filtered TARs; standalone GZ/BZ2/XZ/Zstandard/compress streams; validly Microsoft-signed LZX CAB; and pinned official libarchive 3.8.9 RAR, RAR5, LHA level 3, and BZIP2-compressed ZIPX. They also record all three raw-stream rejection cases with non-publication, Japanese and long paths, normal and standalone-raw shell extraction, one benign plus 18 rejected generated archives, native hardlink/ADS/junction rejection, the 26-control English settings save/diagnosis path, and seven successful profile/root cleanups per image. LPAC requests on both images produced the exact classified unsupported token-query result and failed closed without backend-success output or residue. The Server matrix is not Windows 10/11 desktop evidence; see the [Windows E2E contract](WINDOWS_E2E.md), [LPAC evidence](LPAC_EVALUATION.md), and [corpus contract](MALICIOUS_CORPUS.md).

A `vX.Y.Z` tag whose value matches `Cargo.toml` and points to current `main` builds unsigned x64 and native ARM64 packages on separate native runners. It requires x64 `0x8664` and ARM64 `0xAA64` across build output, standalone assets, and expanded ZIPs; attests both ZIPs, all six EXEs, and the combined checksum inventory; and permits exactly 11 architecture-separated assets. Publication uses the immutable-release policy, creates a draft without overwriting an existing version, verifies exact name/length/digest before publishing, marks it latest, and requires immutable exact readback. The immutable stable [v0.5.2 release](https://github.com/hjosugi/iroha-zip/releases/tag/v0.5.2) passed that entire path in [Actions run 31784888614](https://github.com/hjosugi/iroha-zip/actions/runs/31784888614), after [non-publishing run 31784269152](https://github.com/hjosugi/iroha-zip/actions/runs/31784269152) passed the same package path. An independent public re-download matched all 11 API digests and byte lengths, eight checksum subjects, two sidecars, every direct and ZIP-contained PE identity, all ZIP-to-standalone bytes, bilingual package content, backend non-inclusion, annotated tag object `b54c8ae9dfb898258a61074cb8445f4ef20bb14a`, exact commit `a8ff7e0f30c33131e67e957226f26ba8faf4b214`, all 174 packaged local Markdown links, and all nine hosted-runner-only tag-ref attestations; all six distinct executables had empty Authenticode Certificate Tables as disclosed. `v0.5.1` was the previous complete-contract release, `v0.4.1` was the first post-policy immutable release, and `v0.4.0` predates enforcement and remains mutable. The future Authenticode verification path remains available locally but is inactive until the owner configures and independently reviews a signing identity. See [release verification](RELEASE_VERIFICATION.md).

## Still requires a real Windows validation machine

- Disposable Windows 10/11 x64 runs of the now-passing fixed-Server evidence contract
- LPAC `bsdtar --version`, archive-format, registry/file/COM denial, and network-denial measurements beyond the fixed-Server fail-closed token-query result
- Settings-screen visual fit at 100–300% and mixed DPI, screen readers, completed folder-picker actions, Default Apps, UI-driven backend replacement, and other external-state rollback paths beyond the passing post-backup import rollback
- Native archive-preview tree/search/selection UI, progress/cancellation accessibility, and real-format selected-publication matrix
- Broader legacy-format, malformed-header, control-character, CPU-bomb, cancellation, and race fixtures beyond the passing pinned and generated corpora
- Long-running fuzz campaigns beyond the bounded weekly smoke schedule
- First independent inspection of the Windows-generated MSYS2 provenance, SPDX, and license evidence plus ongoing package-key rotation/archive-availability monitoring
- Real-Windows reparse point race stress tests and native child open relative to a parent handle
- Attachment Services with Defender enabled/disabled/unavailable, third-party providers, quarantine/deletion, ADS inventory, and MotW preservation across publication
- First successful reviewed Authenticode release after owner-managed signing identity validation, plus independent security review
- Windows 10 ARM64 and multiple retail Windows 11 ARM devices beyond the passing hosted native runner

Until those Windows integration checks are complete, treat the unsigned `v0.5.x` line as security-oriented software rather than an audited security product.
