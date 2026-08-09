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
- Sandboxed, 64 MiB-bounded archive-member preflight with raw absolute/drive/UNC, Windows-name, depth/path, and case-alias duplicate rejection before extraction
- Deterministic source-generated ZIP/ustar/old-GNU sparse corpus with one control, 18 reject cases, and Windows-only hardlink/ADS/junction policy fixtures
- Bounded backend-manifest parsing with deterministic malformed-input, path, duplicate, and resource-limit regression tests
- Bounded backend provenance/SPDX/license parsers with exact manifest, ownership, relationship, package-verification-code, notice, license-file, and evidence-tree drift tests
- Generated unsupported-source SPDX JSON validated against the official SPDX v2.3 JSON Schema
- Five pinned fuzz targets, a successful initial bounded sanitizer campaign, isolated seeds, and deterministic minimized-regression promotion
- Handle-retaining input/source snapshots and deterministic source-tree race tests for identity replacement, same-size mutation, rename, hardlinks, and symbolic links
- Bounded Windows `GetFileInformationByHandleEx` directory enumeration, before/after directory-handle identity checks, and empty-directory replacement rejection between audit and copy
- Created-archive full-root re-extraction in a second sandbox, before/after source and archive fingerprints, handle-retained create-new publication, eight deterministic root/mutation/mismatch tests, and a local libarchive 3.8.9 ZIP/7z/TAR/TAR.GZ round trip
- Native settings application type-check against `windows` 0.62.2 APIs
- Settings manifest XML and UI Automation PowerShell syntax parsing
- Serialized initial configuration creation plus a Windows-only independent-process, non-ASCII-path save regression
- Windows named-mutex timeout, post-release recovery, and abandoned-owner recovery unit tests
- Normal-AppContainer/LPAC configuration round trips and Windows LPAC process/token API type-check
- Machine-readable AppContainer token/capability, loopback-denial, timeout, memory-limit, staging-source read/write ACL, and explicit four-profile/root-cleanup probe type-check
- Fixed-label Windows Server 2022/2025 archive, shell, settings-setup, and isolation E2E workflow parsing
- Windows Attachment Services COM boundary type-check, three-policy configuration/settings round trips, fail-closed post-handoff fingerprint tests, and partial cleanup tests
- TOML and GitHub Actions workflow parsing
- `cargo-deny` advisory, license, ban, and source policy checks
- `cargo-about` third-party license inventory generation from the locked dependency graph
- All PowerShell scripts parsed with the official PowerShell 7.6.4 parser; unsupported-source evidence generation, explicit-approval failure/rollback, Rust round-trip validation, strict-mode unsupported-source rejection, and notice-tamper rejection executed on Linux PowerShell
- Release inventory policy: official packages do not bundle EXE, DLL, MSI, PDB, or a backend manifest
- Split build/sign/package release boundary, strict three-EXE Authenticode publisher/EKU/timestamp verification, deterministic signature evidence, pinned SLSA provenance generation, and fail-closed immutable-release workflow parsing

The current Linux suite contains 89 passing default-feature tests, plus the feature-gated minimized fuzz-regression gate (90 with all features). Windows CI additionally runs source-file and directory-handle sharing tests that verify open snapshots block writes and renames, plus two independent processes saving one configuration path.

## Performed by GitHub Actions

The CI workflow runs formatting, tests, Clippy, and release builds on both `ubuntu-latest` and `windows-latest`. Windows CI also exports the current signed MSYS2 UCRT64 backend, validates the supported evidence, exercises the explicit unsupported-bundle path, and proves both `--require-supported` and evidence-tamper failures. It then exercises all 26 native settings controls through UI Automation.

A separate fixed-label `windows-2022` / `windows-2025` x64 matrix builds release binaries, exports the verified backend, and produces machine-readable archive/isolation, malicious-corpus, and settings artifacts. It is designed to exercise token/capability inspection, loopback denial, timeout, memory limit, explicit cleanup, four create/read formats, Japanese and long paths, generated hostile ZIP/TAR inputs, native hardlink/ADS/junction fixtures, invalid input, shell invocation, settings save, and settings diagnosis. These new workflow steps have been parsed locally but have not yet produced a GitHub Actions result for the current local branch. The Server matrix is not Windows 10/11 desktop evidence; see the [Windows E2E contract](WINDOWS_E2E.md) and [corpus contract](MALICIOUS_CORPUS.md).

A `v*` tag whose value matches `Cargo.toml` and points to the current `main` commit enters the protected `release-signing` environment. The workflow requires Azure OIDC configuration and release immutability, signs and verifies all three executables, writes ZIP/SHA-256/signature evidence, verifies a local SLSA provenance bundle, checks a complete draft, publishes it, and then requires GitHub to report the release immutable. This workflow has been implemented and parsed locally but has not produced a signed release. The 2026-08-10 read-only audit found no release environment or Actions variables and found release immutability disabled; owner setup remains deliberately outstanding.

## Still requires a real Windows validation machine

- First passing AppContainer/profile/isolated-`bsdtar` evidence artifacts from the new Server 2022/2025 matrix, followed by disposable Windows 10/11 x64 runs
- LPAC `bsdtar --version`, archive-format, registry/file/COM denial, and network-denial measurements
- Settings-screen visual fit at 100–300% and mixed DPI, screen readers, folder pickers, Default Apps, external-state action rollback, and independent-process save contention
- Native archive-preview tree/search/selection UI, progress/cancellation accessibility, and real-format selected-publication matrix
- RAR/RAR5, LHA/LZH, CAB, ZIPX, and raw compressed-stream read fixtures with a pinned libarchive bundle
- First passing ZIP, 7z, TAR, and TAR.GZ creation/re-extraction tree-hash result from both fixed-label Server jobs
- First passing and reviewed malicious-corpus JSON from both Server jobs, then broader legacy-format, malformed-header, control-character, CPU-bomb, cancellation, and race fixtures
- Long-running fuzz campaigns beyond the bounded weekly smoke schedule
- First independent inspection of the Windows-generated MSYS2 provenance, SPDX, and license evidence plus ongoing package-key rotation/archive-availability monitoring
- Real-Windows reparse point race stress tests, native child open relative to a parent handle, and first Windows evidence for handle enumeration, the staging-tree DACL seal, and internal created-archive re-extraction
- Attachment Services with Defender enabled/disabled/unavailable, third-party providers, quarantine/deletion, ADS inventory, and MotW preservation across publication
- First successful reviewed Authenticode/SLSA/immutable release after owner-managed Azure identity validation and repository settings, plus independent security review

Until those Windows integration checks are complete, treat `v0.3.1` as a security-oriented preview rather than an audited security product.
