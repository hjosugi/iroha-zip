# Build and validation status

Updated: 2026-08-15

## Completed locally

- Rust 1.97.1 `cargo fmt --all -- --check`
- Rust 1.97.1 `cargo test --all-targets --locked` on Linux
- Rust 1.97.1 `cargo clippy --all-targets --locked` on Linux
- `cargo clippy --all-targets --target x86_64-pc-windows-msvc --locked -- -D warnings`
- `cargo check --all-targets --target aarch64-pc-windows-msvc --locked`
- Configuration serialization, backward compatibility, validation, rollback-safe replacement, and path-policy tests
- Platform-neutral settings-form round trips, human-readable byte-unit parsing, and field-specific validation tests
- Platform-neutral 100–300% layout scaling, Per-Monitor V2 manifest/scroll-state contracts, stable 26-control IDs, exhaustive 11-action dispatch mapping, and concurrent-save tests
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
- Encrypted-ZIP `preview`/`extract` through a value-free CLI flag, bilingual protected native dialog,
  bounded zeroizing secret storage, a verified suspended AppContainer child, one-use 4 KiB anonymous
  pipe with explicit handle allowlisting, manifest-pinned libarchive password reader, and fail-closed
  entry/path/resource enforcement. Platform-neutral tests cover cancellation, EOF, overflow, timeout,
  abnormal exit, cleanup, inherited-handle, and output-redaction boundaries.
- Japanese and English outcome summaries for every double-click shell warning and error category,
  while preserving the underlying technical diagnostics.
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
- All PowerShell scripts parsed with the official PowerShell 7.6.5 parser; unsupported-source evidence generation, explicit-approval failure/rollback, Rust round-trip validation, strict-mode unsupported-source rejection, notice-tamper rejection, and bounded GitHub release-attestation verification executed on Linux PowerShell
- Every MSYS2 backend-export child, including batched `ldd`, pacman, and the `bsdtar` package/license inventory, has an independently exercised 180-second default timeout with exact argument preservation, bounded termination, stage reporting, temporary-tree cleanup, and no partial destination installation; dependency discovery uses at most 64 paths per process and rejects more than 256 runtime files
- Release inventory policy: official packages include the three iroha-zip executables but no third-party backend EXE/DLL, MSI, PDB, or backend manifest
- Tag/version/main-commit release gates, native x64/ARM64 builds, architecture-separated ZIPs and six standalone PE assets, exact PE-machine checks inside and outside both ZIPs, SHA-256 inventories, pinned GitHub workflow attestations, exact 11-asset pre-publication/post-publication verification, and bounded post-publication verification of the GitHub release attestation plus all 11 assets
- Future split build/sign/package boundary with strict three-EXE Authenticode publisher/EKU/timestamp verification retained for an owner-configured signing identity
- Japanese and English static Pages rendered at desktop and mobile sizes, with all three public routes passing an automated WCAG 2 AA audit
- All repository Markdown is UTF-8-readable; every relative file target and local heading anchor resolves with exact path casing on both case-sensitive and case-insensitive hosts
- Bilingual bug, feature, and pull-request templates point sensitive reports to the private advisory route and keep their displayed release version under an ordinary Rust regression
- Every GitHub Actions job has an explicit timeout, all 28 external action uses are fixed to full 40-hex commits, all 10 checkouts discard persisted credentials, and an ordinary Rust regression fixes the exact write-permission inventory

The current Linux suite contains 119 passing default-feature tests, including password-transport,
package-version/document, and bilingual Pages version/topology contracts, plus one ignored
system-libarchive compatibility test that is invoked explicitly when the dependency is available.
The feature-gated minimized fuzz-regression gate adds one passing deterministic test to the normal
all-target run. Windows CI additionally runs source-file and directory-handle sharing tests that
verify open snapshots block writes and renames, a rejected-child-stays-suspended regression, two
independent processes saving one configuration path, and a real per-user association round trip that
preserves unrelated registry state and each protected `UserChoice` snapshot.

## Performed by GitHub Actions

GitHub CodeQL default setup now analyzes Rust, GitHub Actions, and JavaScript/TypeScript with the
`extended` suite and `remote_and_local` threat model on standard hosted runners. The initial
[setup run 31788227953](https://github.com/hjosugi/iroha-zip/actions/runs/31788227953) succeeded for all
three languages; Actions and JavaScript/TypeScript reported zero alerts. The Rust baseline reported
233 instances of `rust/path-injection`. Review of every sink and reported source classified 188 as
test-only and 45 as expected same-user, validated-relative, unique-root, create-new, or exact-cleanup
filesystem boundaries. The rule remains enabled for new flows. See the bilingual
[CodeQL baseline and triage](CODEQL.md).

A later Pages behavior regression produced one `js/code-injection` alert (#234) at the test-only
`vm.runInNewContext` boundary. Only checked-in `site/assets/site.js` reaches that VM, so the alert was
dismissed as `used in tests` after review. The current three-language
[CodeQL run 31887104856](https://github.com/hjosugi/iroha-zip/actions/runs/31887104856) passed for
Rust, Actions, and JavaScript/TypeScript on exact `main` commit
`c3984c578f4a49f8bea37e0d22df33b7d0483621`. The repository had zero open CodeQL, Dependabot, and
secret-scanning alerts on 2026-08-15.

The fast CI matrix runs formatting, tests, and Clippy on both `ubuntu-latest` and `windows-latest`,
then builds the Windows settings binary in the debug profile for its 26-control UI Automation
contract and disposable association-state round trip. The matrix also injects backend-import failure
immediately after the prior tree is renamed to backup and requires byte-identical restoration, zero
transaction residue, and a subsequent successful import. These gates, the native ARM64 job, and the
Windows PowerShell 5.1 launcher regression passed for exact `main` commit
`c3984c578f4a49f8bea37e0d22df33b7d0483621` in
[Actions run 31887105016](https://github.com/hjosugi/iroha-zip/actions/runs/31887105016). A transient
signed-database mirror timeout failed the first native ARM64 attempt; rerun attempt 2 completed the
unchanged exact commit and every required job. The same tag commit passed again in
[Actions run 31887844038](https://github.com/hjosugi/iroha-zip/actions/runs/31887844038). All external
action uses are pinned to exact 40-hex commits, every job has an explicit timeout, every checkout
discards persisted credentials, and the repository Actions policy rejects non-SHA references while
retaining read-only default workflow permissions. Production-profile compilation
and signed-MSYS2 backend work are intentionally kept out of the duplicate x64 matrix path.

The native GitHub `windows-11-arm` job requires OS/process architecture `Arm64` and Rust host
`aarch64-pc-windows-msvc`, verifies all three application PEs and all backend EXE/DLL payloads as
machine `0xAA64`, and exports a signature-verified MSYS2 CLANGARM64 backend. It runs the complete
create/read matrix, malicious corpus, shell, Japanese and English Settings, normal-AppContainer
isolation, and exact LPAC fail-closed branch. The schema-v5 password expansion passed at exact `main`
commit `9debd02e819899f8dbdfdd5281d3d0b2a68a89db` in
[Actions run 31875638650](https://github.com/hjosugi/iroha-zip/actions/runs/31875638650). All five reports
were downloaded by exact filename and independently checked: ZipCrypto, WinZip AES-128, and AES-256
each used the bilingual protected native control and one-use channel, exposed no password in output,
and produced preview/extraction trees matching the controlled source. Wrong-password and cancel paths
published no destination. The reports also record normal AppContainer with zero capabilities, seven
successful profile/root cleanups, four create formats, 14 exact read fixtures, three raw-stream
rejection cases, one control plus 18 hostile rejects, standalone-raw shell dispatch, both Settings
languages, and complete temporary-root removal. The two Settings reports also record the exact
26-control forward/reverse order and successful save/dirty-close actions while explicitly marking
the hosted image's non-key fallback. The LPAC query returned the exact
`ERROR_INVALID_PARAMETER` class and did not fall back. See [the ARM64 boundary and setup](ARM64.md).

A separate fixed-label `windows-2022` / `windows-2025` x64 matrix builds the production release
binaries once per supported Server image, exports the verified backend once per image, and produces
machine-readable archive/isolation, malicious-corpus, and settings artifacts. The Server 2025 path
reuses that backend to validate supported evidence, the explicit unsupported-bundle path,
`--require-supported`, rollback, and evidence-tamper failures. The schema-v5 matrix passed on both
images at exact `main` commit `9debd02e819899f8dbdfdd5281d3d0b2a68a89db` in
[Actions run 31875638650](https://github.com/hjosugi/iroha-zip/actions/runs/31875638650). All six reports
were downloaded by exact artifact name and independently checked. Each OS passed ZipCrypto,
AES-128, and AES-256 with the protected bilingual dialog, one-use channel, password output absence,
source-identical preview/extraction trees, wrong-password/cancel non-publication, and complete
cleanup. The same reports cover four created formats and 14 exact read fixtures: four filtered TARs;
standalone GZ/BZ2/XZ/Zstandard/compress streams; validly Microsoft-signed LZX CAB; and pinned official
libarchive 3.8.9 RAR, RAR5, LHA level 3, and BZIP2-compressed ZIPX. They also record all three
raw-stream rejection cases with non-publication, Japanese and long paths, normal and standalone-raw
shell extraction, one benign plus 18 rejected generated archives, native hardlink/ADS/junction
rejection, the 26-control English settings save/diagnosis path, and seven successful profile/root
cleanups per image. The Settings reports require real `SendInput` for both Tab directions and wrap,
Enter save/message dismissal, and Escape close-request/cancellation. LPAC requests on both images
produced the exact classified unsupported token-query
result and failed closed without backend-success output or residue. The Server matrix is not Windows
10/11 desktop evidence; see the [Windows E2E contract](WINDOWS_E2E.md),
[LPAC evidence](LPAC_EVALUATION.md), and [corpus contract](MALICIOUS_CORPUS.md).

A `vX.Y.Z` tag whose value matches `Cargo.toml` and points to current `main` builds unsigned x64 and native ARM64 packages on separate native runners. It requires x64 `0x8664` and ARM64 `0xAA64` across build output, standalone assets, and expanded ZIPs; attests both ZIPs, all six EXEs, and the combined checksum inventory; and permits exactly 11 architecture-separated assets. Publication uses the immutable-release policy, creates a draft without overwriting an existing version, verifies exact name/length/digest before publishing, marks it latest, and requires immutable exact readback. The current independently verified stable [v0.6.3 release](https://github.com/hjosugi/iroha-zip/releases/tag/v0.6.3) passed that entire path in [Actions run 31887844084](https://github.com/hjosugi/iroha-zip/actions/runs/31887844084), after [non-publishing run 31887126357](https://github.com/hjosugi/iroha-zip/actions/runs/31887126357) passed the same package path; exact-source [tag CI run 31887844038](https://github.com/hjosugi/iroha-zip/actions/runs/31887844038) also passed. An independent public re-download matched all 11 API digests and byte lengths, eight checksum subjects, two sidecars, every direct and ZIP-contained PE identity, all ZIP-to-standalone bytes, both bilingual package trees and checked-in source documents, backend non-inclusion, release-body equality, annotated tag object `c99000335f2b64df815f029ff1d7b7d25e31a2c0`, exact commit `c3984c578f4a49f8bea37e0d22df33b7d0483621`, and all nine hosted-runner-only tag-ref workflow attestations; all six distinct executables had empty Authenticode Certificate Tables as disclosed. The separate GitHub release attestation bound the annotated tag object to all 11 assets, and every downloaded asset passed `gh release verify-asset`. The public project-root, Japanese, English, JavaScript, CSS, favicon, robots, sitemap, and custom 404 resources matched the source deployed by [Pages run 31887105044](https://github.com/hjosugi/iroha-zip/actions/runs/31887105044) byte-for-byte. Fixed release metadata, hashes, workflow artifact digests, and check results are preserved in the [v0.6.3 release snapshot](https://github.com/hjosugi/iroha-zip/tree/main/evidence/releases/v0.6.3). `v0.6.2` was the previous complete-contract immutable release, `v0.4.1` was the first post-policy immutable release, and `v0.4.0` predates enforcement and remains mutable. The future Authenticode verification path remains available locally but is inactive until the owner configures and independently reviews a signing identity. See [release verification](RELEASE_VERIFICATION.md).

## Still requires a real Windows validation machine

- Disposable Windows 10/11 x64 runs of the now-passing fixed-Server evidence contract
- LPAC `bsdtar --version`, archive-format, registry/file/COM denial, and network-denial measurements beyond the fixed-Server fail-closed token-query result
- Settings-screen visual fit at 100–300% and on physical mixed-DPI monitors beyond the synthetic 96→144→96 PMv2 relayout test, screen readers, completed folder-picker actions, Default Apps, UI-driven backend replacement, and other external-state rollback paths beyond the passing post-backup import rollback
- Native archive-preview tree/search/selection UI, progress/cancellation accessibility, and real-format selected-publication matrix
- Broader legacy-format, malformed-header, control-character, CPU-bomb, cancellation, and race fixtures beyond the passing pinned and generated corpora
- Long-running fuzz campaigns beyond the bounded weekly smoke schedule
- Periodic promotion of reviewed Windows evidence beyond the two checked-in canonical snapshots and
  rolling 90-day public-repository artifact window
- First independent inspection of the Windows-generated MSYS2 provenance, SPDX, and license evidence plus ongoing package-key rotation/archive-availability monitoring
- Real-Windows reparse point race stress tests and native child open relative to a parent handle
- Attachment Services with Defender enabled/disabled/unavailable, third-party providers, quarantine/deletion, ADS inventory, and MotW preservation across publication
- First successful reviewed Authenticode release after owner-managed signing identity validation, plus independent security review
- Windows 10 ARM64 and multiple retail Windows 11 ARM devices beyond the passing hosted native runner

Until those Windows integration checks are complete, treat the unsigned `v0.6.x` line as security-oriented software rather than an audited security product.
