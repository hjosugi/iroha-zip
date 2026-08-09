# Issue backlog

Updated: 2026-08-10

This backlog records the remaining work discovered during the v0.2.0 refactor. Each stable ID links to its GitHub issue. Priority is based on security and release risk; an item being listed here does not mean its unsafe behavior is currently enabled.

## P0 — release and security evidence

### [SAFE-001: Real Windows end-to-end matrix](https://github.com/hjosugi/iroha-zip/issues/3)

Run extraction and creation on disposable Windows 10/11 x64 machines with a pinned libarchive bundle. Cover every documented format, Japanese filenames, long paths, timeout, memory exhaustion, cleanup after failure, shell invocation, and settings-screen setup.

Acceptance: the matrix is automated where possible; created archives are re-extracted and content-hashed; AppContainer identity and network denial are recorded; no residual profile or temporary tree remains after every tested exit path.

Progress (2026-08-10): a fixed-label Windows Server 2022/2025 Actions matrix now exports a verified MSYS2 backend and emits bounded JSON evidence. It covers zero-capability token inspection, active-loopback denial, deterministic timeout and memory-limit probes, explicit profile/root cleanup, ZIP/7z/TAR/TAR.GZ creation-preview-re-extraction, controlled TAR.BZ2/TAR.XZ/TAR.ZST/TAR.Z read fixtures, SHA-256 tree comparison, Japanese and >260-character paths, invalid-input non-publication, shell invocation, and settings-screen save/doctor automation. The workflow is locally parsed and cross-compiled but has not yet produced an Actions result for this local branch. Disposable Windows 10/11 x64 runs, RAR/LHA/CAB/ZIPX/raw-stream fixtures, LPAC and broader denial measurements, crash/loader/reparse races, and long-term evidence retention remain open; see [the evidence contract](WINDOWS_E2E.md).

### [SAFE-002: Malicious archive regression corpus](https://github.com/hjosugi/iroha-zip/issues/4)

Build a legally redistributable local corpus for Zip Slip, absolute/drive/UNC paths, symlinks, hardlinks, junctions, reparse points, ADS, device names, trailing-dot aliases, duplicate file identities, deep paths, sparse expansion, and count/size bombs.

Acceptance: every sample has expected policy output, runs on a disposable worker, cannot publish weaponized payloads as ordinary issue attachments, and fails closed before final publication.

Progress (2026-08-10): the source tree now deterministically generates one benign ZIP control plus 18 inert hostile ZIP/ustar/old-GNU sparse inputs without checking binary archives into Git. It covers traversal, absolute/drive/UNC paths, ADS/device/alias/invalid names, duplicates, ZIP/TAR links, depth/count/single/total-size bombs, and a one-byte/2 MiB sparse expansion. A bounded sandboxed `bsdtar -t` preflight rejects raw names and case aliases that libarchive would otherwise normalize or overwrite before post-extraction audit. The fixed Server 2022/2025 jobs also generate native hardlink, ADS, and junction fixtures, require non-publication and cleanup, upload only bounded JSON evidence, and never upload the archives. The workflow is locally parsed/cross-compiled but has no Actions result for this branch; Windows 10/11, broader legacy formats, malformed headers, embedded control names, CPU bombs, cancellation/crash/disk-full, races, and long-term evidence remain open. See [the corpus contract](MALICIOUS_CORPUS.md).

### [SAFE-003: Signed release chain](https://github.com/hjosugi/iroha-zip/issues/5)

Add Authenticode signing, documented certificate custody, signature verification, checksummed artifacts, and release provenance/attestations.

Acceptance: all three executables are signed and verified in CI; ZIP/SHA-256/provenance are attached to an immutable release; verification steps are documented independently of GitHub transport security.

Progress (2026-08-10): the local release path now separates validated build from packaging so Azure Artifact Signing can sign exactly the three Windows executables through GitHub OIDC. Packaging requires Windows `WinVerifyTrust` success, the exact configured publisher, Code Signing EKU, and RFC3161 timestamps before and after ZIP expansion; deterministic JSON evidence is embedded and attached. Pinned GitHub attestation creates an offline-verifiable SLSA bundle, and the workflow checks tag/current-`main` identity, all four non-empty draft assets, pre-enabled release immutability, and post-publication `isImmutable`. Certificate custody, least-privilege settings, incident response, and independent checksum/Authenticode/offline-bundle verification are documented in [the signed-release contract](RELEASE_VERIFICATION.md). Owner-managed Azure identity validation, OIDC/RBAC/environment configuration, enabling GitHub release immutability, the first real signed run, and independent evidence review remain open; no release is claimed signed yet.

### [SAFE-004: Backend provenance, SBOM, and license evidence](https://github.com/hjosugi/iroha-zip/issues/6)

Define supported libarchive sources and verify package signatures/hashes before import. Produce an SBOM and third-party notices for every included backend file when a private package uses `-IncludeBackend`.

Acceptance: provenance is machine-readable, unsupported sources generate an explicit warning, and the generated manifest/SBOM/license inventory agree exactly with the imported tree.

Progress (2026-08-10): v1 manifest parsing is a bounded, platform-neutral API. The supported MSYS2 UCRT64 exporter now refreshes an isolated package database under `Required TrustedOnly`, requires the installed and freshly signature-verified repository versions to agree, takes source metadata only from that isolated database, downloads and hash-checks each signed package, runs detailed installed-file checks, compares every imported runtime file with the archive-derived byte stream, and captures package license files. Every import generates bounded machine-readable provenance, an SPDX 2.3 file/package SBOM, an exact payload ownership/license inventory, notices, and hashed license evidence. The Rust validator checks all views and evidence-tree contents against the manifest; arbitrary bundles retain an explicit unsupported state and require separate UI/script approval; private backend packaging validates all evidence and rejects unsupported sources by default. Windows CI covers supported, unsupported, fail-closed, and tamper paths. Independent real-Windows review of the first CI evidence artifact and long-term key-rotation/archive-availability monitoring remain ongoing; see [the evidence contract](BACKEND_EVIDENCE.md).

## P1 — hardening and core capability

### [SAFE-005: Reduce source-tree TOCTOU windows](https://github.com/hjosugi/iroha-zip/issues/7)

Hold handles while validating/copying input archives and compression sources, and compare identity, length, timestamps, and content hashes across the audited copy.

Acceptance: replacement, same-size mutation, rename, hardlink, and reparse races have deterministic regression tests and cannot cause unaudited bytes to enter the sandbox or final archive.

Progress (2026-08-10): v0.3.1 keeps each input archive/source-file handle through validation, hashing, and copy; verifies identity, timestamps, length, and SHA-256; blocks write/delete sharing on Windows; uses `O_NOFOLLOW` on Unix; and compares deterministic path/type/content fingerprints before and after source-tree copy. Creation now rechecks the staged source after the backend exits, snapshots the generated archive, passes those exact bytes into a second sandbox for raw-listing preflight and full-root re-extraction, compares the path/type/length/content tree fingerprint, rechecks source and archive identity, and publishes from a retained handle with create-new semantics. Six fake-backend tests cover success, empty input, source mutation, unsafe generated members, content mismatch, and the single-`archive`-directory heuristic boundary; two unit tests cover same-size generated-archive mutation and identical-byte identity replacement. A local libarchive 3.8.9 run passed ZIP, 7z, TAR, and TAR.GZ create/re-extract comparison. Parent-directory handle-relative enumeration, write-denying staging ACL/handles, a real-Windows reparse race stress test, and first Windows evidence remain open.

### [SAFE-006: Evaluate LPAC and explicit capabilities](https://github.com/hjosugi/iroha-zip/issues/8)

Prototype LPAC, document OS-version behavior, and measure whether required backend operations work without widening access.

Acceptance: default isolation is never weakened on fallback; unsupported systems fail closed; the threat model documents verified ACL/capability differences.

Progress (2026-08-09): an opt-in, zero-capability LPAC prototype is selectable from the settings screen. It adds the documented All Application Packages opt-out process attribute and verifies both AppContainer and LPAC token flags after process creation. Attribute, process, or token verification failure is fail closed and never retries as a regular AppContainer. Cross-compilation and platform-neutral configuration tests pass; real Windows backend/capability and ACL measurements remain open.

### [SAFE-007: Secure encrypted-archive input](https://github.com/hjosugi/iroha-zip/issues/9)

Support passwords without command-line, environment, log, crash-report, or persistent-config exposure.

Acceptance: use a protected anonymous channel or equivalent one-use mechanism, zero sensitive buffers where practical, prevent inherited handles, and test cancellation/wrong-password paths.

Progress (2026-08-09): the stock Windows bsdtar boundary is documented. Its safe-looking interactive callback requires a console input handle, while `--passphrase` exposes the secret in process arguments and is explicitly documented upstream as insecure. The implementation contract therefore uses a one-use ConPTY channel with non-inheritable controller ends, zeroizing buffers, concurrent output draining, bounded prompt handling, and fail-closed cancellation. The ConPTY transport, native password dialog, and encrypted corpus tests remain open.

### [SAFE-008: Defender/antimalware handoff](https://github.com/hjosugi/iroha-zip/issues/10)

Evaluate `IAttachmentExecute`, AMSI, or supported Defender interfaces after publication while preserving Mark-of-the-Web.

Acceptance: scanning cannot silently downgrade fail-closed extraction, engine unavailability has an explicit policy, and results are distinguishable from iroha-zip structural validation.

Progress (2026-08-09): an opt-in `IAttachmentExecute::Save` handoff now runs on the staged tree under explicit disabled, best-effort, or required policy. Enabled handoff is followed by deterministic primary-stream/tree fingerprint comparison, link/reparse/ADS revalidation, and MotW verification before atomic publication. Service acceptance is reported as a handoff rather than a clean verdict. The default remains disabled; a real Windows Defender/unavailable/third-party-provider/quarantine matrix remains open.

### [UX-001: Settings accessibility and integration automation](https://github.com/hjosugi/iroha-zip/issues/11)

Exercise the native settings application at 100–300% DPI, keyboard-only navigation, Japanese/English Windows, screen readers, long/non-ASCII paths, import failure rollback, association changes, and concurrent saves.

Acceptance: all controls are reachable and labelled, content fits supported displays, destructive/long-running actions show progress or confirmation, and UI automation covers every button and setting.

Progress (2026-08-09): the settings executable now embeds System-DPI awareness, scales its logical layout at 100–300%, remains resizable, scrolls both axes, and follows keyboard focus into the viewport. All 26 setting/action controls have stable IDs and access keys; every action dispatch is exhaustively mapped; Windows CI has a native UI Automation smoke test for names, types, focus, bounds, long/non-ASCII input, dirty state, and close confirmation. Configuration saves are serialized, with a named Windows mutex across processes in the current session and deterministic same-process concurrency coverage. Real visual/screen-reader/mixed-DPI evidence, Windows CI results for this change, independent-process contention, and external-state button/rollback automation remain open. See [the detailed accessibility contract](SETTINGS_ACCESSIBILITY.md).

## P2 — usability and platform breadth

### [UX-002: Archive preview without backend privilege expansion](https://github.com/hjosugi/iroha-zip/issues/12)

Design a read-only file listing and selective extraction flow while treating metadata as untrusted.

Acceptance: listing has the same timeout/resource/path policy, does not parse archives in the main process, and selection cannot bypass post-extraction tree audit.

Progress (2026-08-09): extraction now has one shared staging boundary. The `preview` CLI performs a complete temporary extraction under the same AppContainer/LPAC, timeout, Job Object, live resource, and path/security audit, then builds a typed inventory between two full tree fingerprints and publishes nothing. Repeated `extract --select` paths are applied only to the audited payload, are never forwarded to bsdtar, reject unsafe/ambiguous selectors, and are copied through handle-retaining/audited APIs into a tree that is fingerprinted again before the existing partial/handoff/atomic publication path. The native graphical preview, cancellation/progress UX, prerequisite real-Windows/corpus/signed-release evidence, and end-to-end selected-format matrix remain open. See [the preview security contract](ARCHIVE_PREVIEW.md).

### [OPS-001: Signed updater](https://github.com/hjosugi/iroha-zip/issues/13)

Design an opt-in updater with rollback, channel selection, signature verification, and no implicit backend replacement.

Acceptance: packages are verified before execution, downgrade policy is explicit, and backend trust remains independently controlled by the user.

### [QA-001: Fuzzing and property tests](https://github.com/hjosugi/iroha-zip/issues/14)

Fuzz manifest parsing, Windows path validation, archive-name normalization, command-line quoting, and configuration round-trips.

Acceptance: reproducible fuzz targets run in CI on a bounded schedule and all minimized regressions become deterministic tests.

Progress (2026-08-10): backend manifest parsing is separated from filesystem I/O and covered by malformed UTF-8, structural, hash, duplicate, Windows-path, size, depth, and record-count regression tests. Five reproducible `cargo-fuzz` targets now cover the manifest, Windows path validation, archive destination names, Windows command-line quoting, and validated configuration round-trips. A pinned, read-only, weekly workflow bounds each campaign and uploads failures; minimized artifacts are SHA-256-named inputs executed by ordinary CI. The initial local sanitizer campaign completed without a crash. Long-running campaigns and review of future promoted regressions remain ongoing; see [the operating guide](FUZZING.md).

### [PORT-001: Windows ARM64 package](https://github.com/hjosugi/iroha-zip/issues/15)

Validate dependency/toolchain support and add a separately checksummed ARM64 artifact.

Acceptance: native ARM64 AppContainer and archive matrix pass; assets cannot be confused with x64 packages; setup and backend sourcing are documented.

## Dependency order

1. SAFE-001 and SAFE-002 establish behavioral evidence.
2. SAFE-003 and SAFE-004 establish release and supply-chain evidence.
3. SAFE-005 through SAFE-008 harden or expand trust boundaries only after regression coverage exists.
4. UX-002, OPS-001, and PORT-001 remain gated by the same signed-release and Windows matrix requirements.
