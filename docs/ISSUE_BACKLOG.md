# Issue backlog

Updated: 2026-08-15

This backlog records the remaining work discovered during the v0.2.0 refactor. Each stable ID links to its GitHub issue. Priority is based on security and release risk; an item being listed here does not mean its unsafe behavior is currently enabled.

## P0 — release and security evidence

### [SAFE-001: Real Windows end-to-end matrix](https://github.com/hjosugi/iroha-zip/issues/3)

Run extraction and creation on disposable Windows 10/11 x64 machines with a pinned libarchive bundle. Cover every documented format, Japanese filenames, long paths, timeout, memory exhaustion, cleanup after failure, shell invocation, and settings-screen setup.

Acceptance: the matrix is automated where possible; created archives are re-extracted and content-hashed; AppContainer identity and network denial are recorded; no residual profile or temporary tree remains after every tested exit path.

Progress (2026-08-14): the expanded schema-v4 matrix passed on Windows Server 2022, Windows Server 2025, and native Windows 11 ARM from commit `27610e69f21bf85709f70a68695acc1113d22dca` in [Actions run 31778764604](https://github.com/hjosugi/iroha-zip/actions/runs/31778764604); native ARM64 independently repeated it in [run 31778405711](https://github.com/hjosugi/iroha-zip/actions/runs/31778405711). Independently downloaded JSON for every environment records a verified MSYS2 backend, zero-capability AppContainer identity, active-loopback denial, deterministic timeout and memory rejection, abnormal child termination, corrupt-loader process-creation rejection, one effective process-temp path, and seven explicit profile/root cleanups. Each report covers ZIP/7z/TAR/TAR.GZ creation-preview-re-extraction plus 14 exact reads: four filtered TARs; standalone GZ/BZ2/XZ/Zstandard/compress; validly Microsoft-signed LZX CAB; and exact official libarchive 3.8.9 RAR/RAR5/LHA-level-3/ZIPX fixtures. All three raw-stream mismatch/limit/corruption cases reject without publication; normal ZIP and standalone-raw shell dispatch, matching SHA-256 trees, Japanese and >260-character paths, invalid-input non-publication, the generated malicious corpus, x64 English Settings, and native ARM64 Japanese/English Settings also pass. A Windows regression holds the child suspended through a forced two-second token-verification failure and requires zero child stdout. Disposable Windows 10/11 x64 desktop runs, broader LPAC denial measurements, concurrent race stress, further legacy/malformed fixtures, and long-term evidence retention remain open; see [the evidence contract](WINDOWS_E2E.md).

Current extension (2026-08-15): schema-v5 passed on both fixed Windows Server x64 images and native
Windows 11 ARM from exact `main` commit `3ec61f665a3d50a046d3a28c178a8ced7f4276ed` in
[Actions run 31867159915](https://github.com/hjosugi/iroha-zip/actions/runs/31867159915). Exact-name local
checks of all 11 downloaded reports confirmed ZipCrypto/AES-128/AES-256
native-dialog extraction, one-use secret transport, wrong-password/cancel non-publication, the 19-sample
corpus, Settings diagnosis, capability 0, and complete cleanup. Disposable Windows 10/11 x64 desktop
execution remains the acceptance gap; hosted Server/ARM evidence is not relabelled as desktop proof.
New native ARM64 and fixed-Server evidence artifacts are retained for the public-repository maximum
of 90 days. Preservation of reviewed evidence beyond that rolling window remains open.

### [SAFE-002: Malicious archive regression corpus](https://github.com/hjosugi/iroha-zip/issues/4)

Build a legally redistributable local corpus for Zip Slip, absolute/drive/UNC paths, symlinks, hardlinks, junctions, reparse points, ADS, device names, trailing-dot aliases, duplicate file identities, deep paths, sparse expansion, and count/size bombs.

Acceptance: every sample has expected policy output, runs on a disposable worker, cannot publish weaponized payloads as ordinary issue attachments, and fails closed before final publication.

Acceptance met (2026-08-14): the source tree deterministically generates one benign ZIP control plus 18 inert hostile ZIP/ustar/old-GNU sparse inputs without checking binary archives into Git. It covers traversal, absolute/drive/UNC paths, ADS/device/alias/invalid names, duplicates, ZIP/TAR links, depth/count/single/total-size bombs, and a one-byte/2 MiB sparse expansion. A bounded sandboxed preflight (`bsdtar -t` on Unix; manifest-pinned libarchive DLL and its official UTF-8 pathname API on Windows) rejects raw names and case aliases that extraction could otherwise normalize or overwrite before post-extraction audit. Both fixed Server jobs passed the control, all 18 expected rejections, native hardlink/ADS/junction rejection, non-publication, and cleanup in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176); only bounded JSON was uploaded and the archives were deleted. Broader legacy formats, malformed headers, embedded control names, CPU bombs, cancellation/crash/disk-full, races, and long-term evidence are future expansion, not unclaimed coverage. See [the corpus contract](MALICIOUS_CORPUS.md).

### [SAFE-003: Signed release chain](https://github.com/hjosugi/iroha-zip/issues/5)

Add Authenticode signing, documented certificate custody, signature verification, checksummed artifacts, and release provenance/attestations.

Acceptance: all three executables are signed and verified in CI; ZIP/SHA-256/provenance are attached to an immutable release; verification steps are documented independently of GitHub transport security.

Progress (2026-08-15): the active workflow intentionally publishes unsigned Windows x64 and native ARM64 binaries with exact tag/version/current-`main` gates, two backend-free ZIPs, six individually downloadable PE executables, SHA-256 inventories, and GitHub artifact attestations. Publication creates an explicit draft, verifies all 11 remote assets by exact name/length/SHA-256 before publishing, and requires immutable/latest readback afterward. The unsigned [v0.6.0 release](https://github.com/hjosugi/iroha-zip/releases/tag/v0.6.0) passed that complete path in [Actions run 31864491738, attempt 2](https://github.com/hjosugi/iroha-zip/actions/runs/31864491738), after [dry run 31864073772](https://github.com/hjosugi/iroha-zip/actions/runs/31864073772) passed without publication. All public assets were independently downloaded; API digests and byte lengths, eight checksum subjects, two sidecars, x64/ARM64 PE identities inside and outside both ZIPs, empty Authenticode Certificate Tables, bilingual package content and source-document equality, backend non-inclusion, annotated tag object `4464c6b61ee809d9079a45b29c1626df5188303d`, exact commit `4464e4fb7ef36e1e24c54969df57817dd4202b25`, and nine hosted-runner-only tag-ref attestations all matched. The package and website disclose the absence of Authenticode and provide Japanese/English verification guidance. The local build/sign/package split and strict WinVerifyTrust publisher/EKU/timestamp verifier remain available for a future owner-configured signing identity. Authenticode custody, the first signed run, and independent review remain open; no current release is claimed signed. See [the release contract](RELEASE_VERIFICATION.md).

### [SAFE-004: Backend provenance, SBOM, and license evidence](https://github.com/hjosugi/iroha-zip/issues/6)

Define supported libarchive sources and verify package signatures/hashes before import. Produce an SBOM and third-party notices for every included backend file when a private package uses `-IncludeBackend`.

Acceptance: provenance is machine-readable, unsupported sources generate an explicit warning, and the generated manifest/SBOM/license inventory agree exactly with the imported tree.

Acceptance met (2026-08-14): v1 manifest parsing is a bounded, platform-neutral API. The supported MSYS2 UCRT64 exporter refreshes an isolated package database under `Required TrustedOnly`, requires the installed and freshly signature-verified repository versions to agree, takes source metadata only from that isolated database, downloads and hash-checks each signed package, runs detailed installed-file checks, compares every imported runtime file with the archive-derived byte stream, and captures package license files. Every import generates bounded machine-readable provenance, an SPDX 2.3 file/package SBOM, an exact payload ownership/license inventory, notices, and hashed license evidence. The Rust validator checks all views and evidence-tree contents against the manifest; arbitrary bundles retain an explicit unsupported state and require separate UI/script approval; private backend packaging validates all evidence and rejects unsupported sources by default. The Server 2025 job passed supported, unsupported, strict-source, rollback, and evidence-tamper gates in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176). Long-term key-rotation/archive-availability monitoring remains operational follow-up; see [the evidence contract](BACKEND_EVIDENCE.md).

## P1 — hardening and core capability

### [SAFE-005: Reduce source-tree TOCTOU windows](https://github.com/hjosugi/iroha-zip/issues/7)

Hold handles while validating/copying input archives and compression sources, and compare identity, length, timestamps, and content hashes across the audited copy.

Acceptance: replacement, same-size mutation, rename, hardlink, and reparse races have deterministic regression tests and cannot cause unaudited bytes to enter the sandbox or final archive.

Acceptance met (2026-08-14): v0.5.0 keeps each input archive/source-file handle through validation, hashing, and copy; verifies identity, timestamps, length, and SHA-256; blocks write/delete sharing on Windows; uses `O_NOFOLLOW` on Unix; and compares deterministic path/type/content fingerprints before and after source-tree copy. Windows tree membership comes from bounded `GetFileInformationByHandleEx` enumeration on reparse-aware, rename/delete-blocking directory handles. For creation, the trusted parent copies only audited regular objects into a unique external staging tree and requires the complete fingerprint to match. It protects every file/directory DACL from inheritance, grants the Package SID read/execute only, and serializes the sealed tree into a deterministic bounded PAX stream. The backend receives only fixed sandbox-local `@source.pax.tar`, avoiding both the normal source path and libarchive's AppContainer-denied volume-root query. The PAX handle and staging fingerprint are retained/rechecked; the result is re-extracted in a second sandbox, compared by full path/type/length/content, and published from a retained handle with create-new semantics. Each sandbox backend tree is also recursively read/execute-only so a listing pass cannot persist self-modification into extraction. Every Windows child is created suspended, assigned to its Job, and resumed exactly once only after token mode and zero-capability verification; a forced two-second verification failure requires empty child stdout. On Windows, a dedicated child loads only manifest-pinned DLL candidates, rechecks zero-capability AppContainer isolation, and uses libarchive's official UTF-8 pathname API; ZIP/PAX writers explicitly request UTF-8 names, and disposable backend EXEs retain a byte-verified UTF-8 process manifest. Deterministic tests cover same-size mutation, identical-byte identity replacement, rename, hardlinks, symbolic links, empty-directory replacement, final-archive mutation/replacement, and non-publication of unaudited bytes. A Windows test replaces an audited directory with a real `mklink /J` reparse point, confirms the reparse identity, requires the specific post-audit link rejection, preserves the outside hostile file, and publishes no target; it passed on x64 and native ARM64 in [Actions run 31770292261](https://github.com/hjosugi/iroha-zip/actions/runs/31770292261), and the complete suite passed again in [Actions run 31774280671](https://github.com/hjosugi/iroha-zip/actions/runs/31774280671). Native child opens relative to retained parent handles and concurrent race stress remain documented defense-in-depth follow-ups beyond this issue's acceptance criteria.

### [SAFE-006: Evaluate LPAC and explicit capabilities](https://github.com/hjosugi/iroha-zip/issues/8)

Prototype LPAC, document OS-version behavior, and measure whether required backend operations work without widening access.

Acceptance: default isolation is never weakened on fallback; unsupported systems fail closed; the threat model documents verified ACL/capability differences.

Progress (2026-08-14): an opt-in, zero-capability LPAC prototype is selectable from the settings screen. It adds the documented All Application Packages opt-out process attribute, creates the child suspended, and requires positive AppContainer/LPAC token flags plus zero capabilities before the only resume. Attribute, process, token verification, or resume failure terminates the Job without a regular-AppContainer retry. On fixed Windows Server 2022 build 20348 and Server 2025 build 26100, `TokenIsLessPrivilegedAppContainer` returned `ERROR_INVALID_PARAMETER`; the harness accepted only this exact classified unsupported result, exit code 2, empty report stdout, no backend-success output, and complete cleanup. Normal AppContainer passed on both. This is fail-closed runtime evidence, not proof that the kernels cannot create any LPAC. Real LPAC archive, ACL, registry/file/COM, and wider network measurements remain open; see [the LPAC evaluation](LPAC_EVALUATION.md).

### [SAFE-007: Secure encrypted-archive input](https://github.com/hjosugi/iroha-zip/issues/9)

Support passwords without command-line, environment, log, crash-report, or persistent-config exposure.

Acceptance: use a protected anonymous channel or equivalent one-use mechanism, zero sensitive buffers where practical, prevent inherited handles, and test cancellation/wrong-password paths.

Acceptance met (2026-08-15): the product path uses a one-use anonymous pipe without `--passphrase`,
inheritable controller handles, or environment/config/file/log storage. A byte-identical sealed
internal child remains suspended until its AppContainer/LPAC token and zero capabilities are
verified; only its stdin read handle is admitted through the explicit handle list. The parent writes
the bounded value into a dedicated 4 KiB pipe and closes it while the verified child is still
suspended, then performs the sole resume without a synchronous pipe flush. The child loads only
manifest-pinned libarchive DLL candidates, registers the one bounded value through the public
libarchive password API, rejects non-file/directory entries before creation, and re-enforces path and
resource limits while reading. A non-`Clone`, always-redacted secret owns zeroizing UTF-16/UTF-8
buffers, and the bilingual password control is cleared before destruction. Platform-neutral tests
and a real-AppContainer probe cover Japanese input, cancellation, EOF after one value, timeout,
large output, crash, cleanup, and log absence. The schema-v5 matrix generates
ZipCrypto/AES-128/AES-256 ZIPs, drives the native UI, compares preview/extracted trees, and requires
wrong-password/cancel non-publication. PR [#16](https://github.com/hjosugi/iroha-zip/pull/16)
merged after all hosted checks passed. Exact-main
[Actions run 31867159915](https://github.com/hjosugi/iroha-zip/actions/runs/31867159915) exercised the
real encrypted backend for ZipCrypto, AES-128, and AES-256 on Windows Server 2022/2025 x64 and native
Windows 11 ARM. All 11 downloaded evidence JSON files were independently checked for schema-v5
success, protected bilingual controls, secret output absence, one-use delivery,
wrong-password/cancel non-publication, AppContainer capability 0, malicious-corpus rejection,
Japanese/English Settings diagnosis, correct ARM64 PE identity, and complete temporary-root cleanup.
Issue #9 closed with the reviewed merge.

### [SAFE-008: Defender/antimalware handoff](https://github.com/hjosugi/iroha-zip/issues/10)

Evaluate `IAttachmentExecute`, AMSI, or supported Defender interfaces after publication while preserving Mark-of-the-Web.

Acceptance: scanning cannot silently downgrade fail-closed extraction, engine unavailability has an explicit policy, and results are distinguishable from iroha-zip structural validation.

Acceptance met (2026-08-14): an opt-in `IAttachmentExecute::Save` handoff runs on the staged tree under explicit disabled, best-effort, or required policy. Enabled handoff is followed by deterministic primary-stream/tree fingerprint comparison, link/reparse/ADS revalidation, and MotW verification before atomic publication. Service acceptance is reported as a handoff rather than a clean verdict, and the default remains disabled. Platform-neutral policy/failure/publication tests pass in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176). A real Windows Defender/unavailable/third-party-provider/quarantine interoperability matrix remains follow-up and is not claimed.

### [UX-001: Settings accessibility and integration automation](https://github.com/hjosugi/iroha-zip/issues/11)

Exercise the native settings application at 100–300% DPI, keyboard-only navigation, Japanese/English Windows, screen readers, long/non-ASCII paths, import failure rollback, association changes, and concurrent saves.

Acceptance: all controls are reachable and labelled, content fits supported displays, destructive/long-running actions show progress or confirmation, and UI automation covers every button and setting.

Progress (2026-08-15): the settings executable declares Per-Monitor V2 awareness with Per-Monitor and legacy fallbacks. Its `WM_DPICHANGED` handler applies the suggested window rectangle, rebuilds every child layout from the 96-DPI baseline, rescales scroll state, recreates the DPI-specific system font, and recalculates both scrollbars. Windows UI Automation now requires the effective PMv2 HWND context and a drift-free synthetic 96→144→96 relayout in addition to Japanese and English names, types, focus, bounds, long/non-ASCII input, dirty state, and confirmations for all 26 controls. The script opens and cancels all three safe folder-picker paths, verifies both Restore Defaults decisions, drives both unsaved-change Cancel decisions, and uses accessible dialog patterns across desktop/Server UIA providers. Configuration replacement and initial creation are serialized by a named Windows mutex across processes in the current session. Thread regressions cover simultaneous replacement and first creation; a Windows-only integration test releases two independent processes against the same non-ASCII path and rejects partial or leftover staging state. Windows unit tests also require bounded timeout failure, successful acquisition after release, and abandoned-owner recovery. Injected configuration rename failures require byte-identical restoration or preservation and reporting of the exact recovery-backup path. Backend import CI separately injects failure immediately after the prior backend is renamed to its unique backup, requires byte-identical restoration with no stage/backup residue, and completes a normal import successfully. A disposable per-user registry test registers all 18 archive types twice, checks exact commands/icons/capabilities, and unregisters while preserving unrelated values and an exact snapshot of every protected `UserChoice`. Real physical-monitor visual/screen-reader evidence, completed external folder/default-app actions, UI-driven backend replacement, and broader external-state failure paths remain open. See [the detailed accessibility contract](SETTINGS_ACCESSIBILITY.md).

## P2 — usability and platform breadth

### [UX-002: Archive preview without backend privilege expansion](https://github.com/hjosugi/iroha-zip/issues/12)

Design a read-only file listing and selective extraction flow while treating metadata as untrusted.

Acceptance: listing has the same timeout/resource/path policy, does not parse archives in the main process, and selection cannot bypass post-extraction tree audit.

Acceptance met (2026-08-14): extraction has one shared staging boundary. The `preview` CLI performs a complete temporary extraction under the same AppContainer/LPAC, timeout, Job Object, live resource, and path/security audit, then builds a typed inventory between two full tree fingerprints and publishes nothing. Repeated `extract --select` paths are applied only to the audited payload, are never forwarded to bsdtar, reject unsafe/ambiguous selectors, and are copied through handle-retaining/audited APIs into a tree that is fingerprinted again before the existing partial/handoff/atomic publication path. Deterministic selection regressions and the real-Windows preview/extract matrix pass in [Actions run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176). A native graphical browser and cancellation/progress UX are separate future features, not part of this CLI security-boundary acceptance. See [the preview security contract](ARCHIVE_PREVIEW.md).

### [OPS-001: Signed updater](https://github.com/hjosugi/iroha-zip/issues/13)

Design an opt-in updater with rollback, channel selection, signature verification, and no implicit backend replacement.

Acceptance: packages are verified before execution, downgrade policy is explicit, and backend trust remains independently controlled by the user.

Progress (2026-08-15): iroha-zip still ships no updater and performs no background update check because the `0.6.x` line, including the `v0.6.0` package, is intentionally unsigned. The bilingual [signed-updater design](UPDATER.md) fixes the disabled-by-default stable/preview channels, Authenticode publisher/EKU/timestamp and exact digest/inventory gates, default downgrade denial plus explicit signed recovery downgrade, same-volume replacement and byte-identical rollback behavior, and the invariant that updater code never modifies backend trust/evidence. Tamper, interruption, AV, disk-full, concurrent-update, launch, and rollback tests remain prerequisites. Implementation remains gated on SAFE-003's owner-controlled signing identity and first reviewed signed immutable release.

### [QA-001: Fuzzing and property tests](https://github.com/hjosugi/iroha-zip/issues/14)

Fuzz manifest parsing, Windows path validation, archive-name normalization, command-line quoting, and configuration round-trips.

Acceptance: reproducible fuzz targets run in CI on a bounded schedule and all minimized regressions become deterministic tests.

Acceptance met (2026-08-14): backend manifest parsing is separated from filesystem I/O and covered by malformed UTF-8, structural, hash, duplicate, Windows-path, size, depth, and record-count regression tests. Five reproducible `cargo-fuzz` targets cover the manifest, Windows path validation, archive destination names, Windows command-line quoting, and validated configuration round-trips. The pinned, read-only, weekly workflow bounds each campaign and uploads failures; minimized artifacts are SHA-256-named inputs executed by ordinary CI. The scheduled campaign passed in [Actions run 31354299685](https://github.com/hjosugi/iroha-zip/actions/runs/31354299685), and the normal minimized-regression gate passed again in [run 31763927176](https://github.com/hjosugi/iroha-zip/actions/runs/31763927176). Longer campaigns and review of future promoted regressions remain ongoing maintenance; see [the operating guide](FUZZING.md).

### [PORT-001: Windows ARM64 package](https://github.com/hjosugi/iroha-zip/issues/15)

Validate dependency/toolchain support and add a separately checksummed ARM64 artifact.

Acceptance: native ARM64 AppContainer and archive matrix pass; assets cannot be confused with x64 packages; setup and backend sourcing are documented.

Acceptance met (2026-08-15): native GitHub `windows-11-arm` CI requires OS/process architecture `Arm64` and Rust host `aarch64-pc-windows-msvc`, runs all-target tests and Clippy, and requires PE machine `0xAA64` for all three application EXEs and every backend EXE/DLL. The signature-enforcing MSYS2 exporter and strict source/repository/package-pair validator support CLANGARM64 with exact package versions, signed database/archive hashes, payload byte comparison, provenance, SPDX, and license inventory. The current native run covers ZIP/7z/TAR/TAR.GZ creation, 14 exact read fixtures including standalone streams, Microsoft LZX CAB, and official libarchive RAR/RAR5/LHA/ZIPX fixtures, all three raw-stream negative cases, the 18-reject/one-control malicious corpus plus native policy fixtures, normal and standalone-raw shell dispatch, Japanese and English Settings, normal schema-v5 AppContainer, and the exact LPAC fail-closed branch. All five downloaded JSONs from [Actions run 31867159915](https://github.com/hjosugi/iroha-zip/actions/runs/31867159915), independently repeated on exact release source in [tag CI run 31864491729](https://github.com/hjosugi/iroha-zip/actions/runs/31864491729), record those matrices as executed and every temporary root removed. The ARM64 Settings import path selects CLANGARM64 automatically and the exporter rejects non-ARM64 backend payloads. The immutable [v0.6.0 release](https://github.com/hjosugi/iroha-zip/releases/tag/v0.6.0), published by [Actions run 31864491738, attempt 2](https://github.com/hjosugi/iroha-zip/actions/runs/31864491738), separates x64 and ARM64 names, sidecars, ZIPs, and six standalone EXEs; requires `0x8664`/`0xAA64` both outside and inside each ZIP; and admits only the exact 11-asset inventory. Independent public re-download verified every API digest and byte length, checksum, PE identity, ZIP-to-standalone byte match, bilingual package content and source-document equality, backend non-inclusion, annotated tag identity, and all nine hosted-runner-only tag-ref attestations. Additional retail-device breadth remains ongoing compatibility work, not an unmet package acceptance condition; see [the ARM64 boundary](ARM64.md).

## Dependency order

1. SAFE-001 and SAFE-002 establish behavioral evidence.
2. SAFE-003 and SAFE-004 establish release and supply-chain evidence.
3. SAFE-005 through SAFE-008 harden or expand trust boundaries only after regression coverage exists.
4. OPS-001 remains gated on Authenticode identity; platform breadth continues only after its native matrix and architecture-specific release checks pass.
