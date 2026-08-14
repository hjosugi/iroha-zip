# Issue backlog

Updated: 2026-08-14

This backlog records the remaining work discovered during the v0.2.0 refactor. Each stable ID links to its GitHub issue. Priority is based on security and release risk; an item being listed here does not mean its unsafe behavior is currently enabled.

## P0 — release and security evidence

### [SAFE-001: Real Windows end-to-end matrix](https://github.com/hjosugi/iroha-zip/issues/3)

Run extraction and creation on disposable Windows 10/11 x64 machines with a pinned libarchive bundle. Cover every documented format, Japanese filenames, long paths, timeout, memory exhaustion, cleanup after failure, shell invocation, and settings-screen setup.

Acceptance: the matrix is automated where possible; created archives are re-extracted and content-hashed; AppContainer identity and network denial are recorded; no residual profile or temporary tree remains after every tested exit path.

Progress (2026-08-14): the complete fixed-label Windows Server 2022/2025 matrix passed in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176). Its downloaded JSON records a verified MSYS2 backend, zero-capability AppContainer identity, active-loopback denial, deterministic timeout and memory rejection, one effective process-temp path, five explicit profile/root cleanups, ZIP/7z/TAR/TAR.GZ creation-preview-re-extraction, controlled TAR.BZ2/TAR.XZ/TAR.ZST/TAR.Z reads, matching SHA-256 trees, Japanese and >260-character paths, invalid-input non-publication, shell invocation, the generated malicious corpus, and the 26-control English settings save/doctor path. Disposable Windows 10/11 x64 runs, RAR/LHA/CAB/ZIPX/raw-stream fixtures, LPAC and broader denial measurements, crash/loader/reparse races, and long-term evidence retention remain open; see [the evidence contract](WINDOWS_E2E.md).

### [SAFE-002: Malicious archive regression corpus](https://github.com/hjosugi/iroha-zip/issues/4)

Build a legally redistributable local corpus for Zip Slip, absolute/drive/UNC paths, symlinks, hardlinks, junctions, reparse points, ADS, device names, trailing-dot aliases, duplicate file identities, deep paths, sparse expansion, and count/size bombs.

Acceptance: every sample has expected policy output, runs on a disposable worker, cannot publish weaponized payloads as ordinary issue attachments, and fails closed before final publication.

Acceptance met (2026-08-14): the source tree deterministically generates one benign ZIP control plus 18 inert hostile ZIP/ustar/old-GNU sparse inputs without checking binary archives into Git. It covers traversal, absolute/drive/UNC paths, ADS/device/alias/invalid names, duplicates, ZIP/TAR links, depth/count/single/total-size bombs, and a one-byte/2 MiB sparse expansion. A bounded sandboxed preflight (`bsdtar -t` on Unix; manifest-pinned libarchive DLL and its official UTF-8 pathname API on Windows) rejects raw names and case aliases that extraction could otherwise normalize or overwrite before post-extraction audit. Both fixed Server jobs passed the control, all 18 expected rejections, native hardlink/ADS/junction rejection, non-publication, and cleanup in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176); only bounded JSON was uploaded and the archives were deleted. Broader legacy formats, malformed headers, embedded control names, CPU bombs, cancellation/crash/disk-full, races, and long-term evidence are future expansion, not unclaimed coverage. See [the corpus contract](MALICIOUS_CORPUS.md).

### [SAFE-003: Signed release chain](https://github.com/hjosugi/iroha-zip/issues/5)

Add Authenticode signing, documented certificate custody, signature verification, checksummed artifacts, and release provenance/attestations.

Acceptance: all three executables are signed and verified in CI; ZIP/SHA-256/provenance are attached to an immutable release; verification steps are documented independently of GitHub transport security.

Progress (2026-08-14): the active workflow intentionally publishes unsigned Windows x64 binaries with exact tag/version/current-`main` gates, a complete ZIP, three individually downloadable PE executables, SHA-256 inventories, and GitHub artifact attestations. Repository release immutability is enabled for future releases. Publication creates an explicit draft, verifies all six remote assets by exact name/length/SHA-256 before publishing, and requires immutable/latest readback afterward. An administrator must confirm the policy before tagging because its status endpoint requires `Administration: read`, which the standard Actions token cannot request. The already-published `v0.4.0` predates policy enforcement and remains mutable; its assets and attestations were independently checked. The package and website disclose the absence of Authenticode and provide Japanese/English verification guidance. The local build/sign/package split and strict WinVerifyTrust publisher/EKU/timestamp verifier remain available for a future owner-configured signing identity. Authenticode custody, the first signed immutable run, and independent review remain open; no current release is claimed signed. See [the release contract](RELEASE_VERIFICATION.md).

### [SAFE-004: Backend provenance, SBOM, and license evidence](https://github.com/hjosugi/iroha-zip/issues/6)

Define supported libarchive sources and verify package signatures/hashes before import. Produce an SBOM and third-party notices for every included backend file when a private package uses `-IncludeBackend`.

Acceptance: provenance is machine-readable, unsupported sources generate an explicit warning, and the generated manifest/SBOM/license inventory agree exactly with the imported tree.

Acceptance met (2026-08-14): v1 manifest parsing is a bounded, platform-neutral API. The supported MSYS2 UCRT64 exporter refreshes an isolated package database under `Required TrustedOnly`, requires the installed and freshly signature-verified repository versions to agree, takes source metadata only from that isolated database, downloads and hash-checks each signed package, runs detailed installed-file checks, compares every imported runtime file with the archive-derived byte stream, and captures package license files. Every import generates bounded machine-readable provenance, an SPDX 2.3 file/package SBOM, an exact payload ownership/license inventory, notices, and hashed license evidence. The Rust validator checks all views and evidence-tree contents against the manifest; arbitrary bundles retain an explicit unsupported state and require separate UI/script approval; private backend packaging validates all evidence and rejects unsupported sources by default. The Server 2025 job passed supported, unsupported, strict-source, rollback, and evidence-tamper gates in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176). Long-term key-rotation/archive-availability monitoring remains operational follow-up; see [the evidence contract](BACKEND_EVIDENCE.md).

## P1 — hardening and core capability

### [SAFE-005: Reduce source-tree TOCTOU windows](https://github.com/hjosugi/iroha-zip/issues/7)

Hold handles while validating/copying input archives and compression sources, and compare identity, length, timestamps, and content hashes across the audited copy.

Acceptance: replacement, same-size mutation, rename, hardlink, and reparse races have deterministic regression tests and cannot cause unaudited bytes to enter the sandbox or final archive.

Progress (2026-08-14): v0.4.0 keeps each input archive/source-file handle through validation, hashing, and copy; verifies identity, timestamps, length, and SHA-256; blocks write/delete sharing on Windows; uses `O_NOFOLLOW` on Unix; and compares deterministic path/type/content fingerprints before and after source-tree copy. Windows tree membership comes from bounded `GetFileInformationByHandleEx` enumeration on reparse-aware, rename/delete-blocking directory handles. For creation, the trusted parent copies only audited regular objects into a unique external staging tree and requires the complete fingerprint to match. It protects every file/directory DACL from inheritance, grants the Package SID read/execute only, and serializes the sealed tree into a deterministic bounded PAX stream. The backend receives only fixed sandbox-local `@source.pax.tar`, avoiding both the normal source path and libarchive's AppContainer-denied volume-root query. The PAX handle and staging fingerprint are retained/rechecked; the result is re-extracted in a second sandbox, compared by full path/type/length/content, and published from a retained handle with create-new semantics. Each sandbox backend tree is also recursively read/execute-only so a listing pass cannot persist self-modification into extraction. On Windows, a dedicated child loads only manifest-pinned DLL candidates, rechecks zero-capability AppContainer isolation, and uses libarchive's official UTF-8 pathname API; ZIP/PAX writers explicitly request UTF-8 names, and disposable backend EXEs retain a byte-verified UTF-8 process manifest. Fake-backend regressions, a local real-libarchive round trip, and both fixed Server jobs in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176) cover ZIP/7z/TAR/TAR.GZ, internal re-extraction, Japanese/long paths, directory enumeration, and the staging-tree DACL seal. Native child open relative to the parent handle and a real-Windows reparse race stress test remain open.

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

Acceptance met (2026-08-14): an opt-in `IAttachmentExecute::Save` handoff runs on the staged tree under explicit disabled, best-effort, or required policy. Enabled handoff is followed by deterministic primary-stream/tree fingerprint comparison, link/reparse/ADS revalidation, and MotW verification before atomic publication. Service acceptance is reported as a handoff rather than a clean verdict, and the default remains disabled. Platform-neutral policy/failure/publication tests pass in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176). A real Windows Defender/unavailable/third-party-provider/quarantine interoperability matrix remains follow-up and is not claimed.

### [UX-001: Settings accessibility and integration automation](https://github.com/hjosugi/iroha-zip/issues/11)

Exercise the native settings application at 100–300% DPI, keyboard-only navigation, Japanese/English Windows, screen readers, long/non-ASCII paths, import failure rollback, association changes, and concurrent saves.

Acceptance: all controls are reachable and labelled, content fits supported displays, destructive/long-running actions show progress or confirmation, and UI automation covers every button and setting.

Progress (2026-08-14): the settings executable embeds System-DPI awareness, scales its logical layout at 100–300%, remains resizable, scrolls both axes, and follows keyboard focus into the viewport. All 26 setting/action controls have stable IDs and access keys; every action dispatch is exhaustively mapped; Windows CI has a native UI Automation smoke test for Japanese and English names, types, focus, bounds, long/non-ASCII input, dirty state, and confirmations. The script opens and cancels all three safe folder-picker paths, verifies both Restore Defaults decisions, drives both unsaved-change Cancel decisions, and uses accessible dialog patterns across desktop/Server UIA providers. Configuration replacement and initial creation are serialized by a named Windows mutex across processes in the current session. Thread regressions cover simultaneous replacement and first creation; a Windows-only integration test releases two independent processes against the same non-ASCII path and rejects partial or leftover staging state. Windows unit tests also require bounded timeout failure, successful acquisition after release, and abandoned-owner recovery. Injected rename failures require byte-identical restoration or preservation and reporting of the exact recovery-backup path. Both fixed Server jobs saved a real verified backend and completed settings-screen AppContainer diagnosis in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176). Real visual/screen-reader/mixed-DPI evidence and actual external-state/import-rollback automation remain open. See [the detailed accessibility contract](SETTINGS_ACCESSIBILITY.md).

## P2 — usability and platform breadth

### [UX-002: Archive preview without backend privilege expansion](https://github.com/hjosugi/iroha-zip/issues/12)

Design a read-only file listing and selective extraction flow while treating metadata as untrusted.

Acceptance: listing has the same timeout/resource/path policy, does not parse archives in the main process, and selection cannot bypass post-extraction tree audit.

Acceptance met (2026-08-14): extraction has one shared staging boundary. The `preview` CLI performs a complete temporary extraction under the same AppContainer/LPAC, timeout, Job Object, live resource, and path/security audit, then builds a typed inventory between two full tree fingerprints and publishes nothing. Repeated `extract --select` paths are applied only to the audited payload, are never forwarded to bsdtar, reject unsafe/ambiguous selectors, and are copied through handle-retaining/audited APIs into a tree that is fingerprinted again before the existing partial/handoff/atomic publication path. Deterministic selection regressions and the real-Windows preview/extract matrix pass in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176). A native graphical browser and cancellation/progress UX are separate future features, not part of this CLI security-boundary acceptance. See [the preview security contract](ARCHIVE_PREVIEW.md).

### [OPS-001: Signed updater](https://github.com/hjosugi/iroha-zip/issues/13)

Design an opt-in updater with rollback, channel selection, signature verification, and no implicit backend replacement.

Acceptance: packages are verified before execution, downgrade policy is explicit, and backend trust remains independently controlled by the user.

### [QA-001: Fuzzing and property tests](https://github.com/hjosugi/iroha-zip/issues/14)

Fuzz manifest parsing, Windows path validation, archive-name normalization, command-line quoting, and configuration round-trips.

Acceptance: reproducible fuzz targets run in CI on a bounded schedule and all minimized regressions become deterministic tests.

Acceptance met (2026-08-14): backend manifest parsing is separated from filesystem I/O and covered by malformed UTF-8, structural, hash, duplicate, Windows-path, size, depth, and record-count regression tests. Five reproducible `cargo-fuzz` targets cover the manifest, Windows path validation, archive destination names, Windows command-line quoting, and validated configuration round-trips. The pinned, read-only, weekly workflow bounds each campaign and uploads failures; minimized artifacts are SHA-256-named inputs executed by ordinary CI. The scheduled campaign passed in [Actions run 31354299685](https://github.com/hjosugi/iroha-zip/actions/runs/31354299685), and the normal minimized-regression gate passed again in [run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176). Longer campaigns and review of future promoted regressions remain ongoing maintenance; see [the operating guide](FUZZING.md).

### [PORT-001: Windows ARM64 package](https://github.com/hjosugi/iroha-zip/issues/15)

Validate dependency/toolchain support and add a separately checksummed ARM64 artifact.

Acceptance: native ARM64 AppContainer and archive matrix pass; assets cannot be confused with x64 packages; setup and backend sourcing are documented.

## Dependency order

1. SAFE-001 and SAFE-002 establish behavioral evidence.
2. SAFE-003 and SAFE-004 establish release and supply-chain evidence.
3. SAFE-005 through SAFE-008 harden or expand trust boundaries only after regression coverage exists.
4. UX-002, OPS-001, and PORT-001 remain gated by the same signed-release and Windows matrix requirements.
