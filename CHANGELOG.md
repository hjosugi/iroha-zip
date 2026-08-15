# Changelog

## Unreleased

- Preserve the independently verified `v0.6.2` immutable Release as an exact 11-asset snapshot with
  API digests and lengths, checksum subjects, x64/ARM64 PE identities, ZIP-to-standalone equality,
  unsigned Certificate Tables, annotated-tag identity, release/workflow attestations, and ordinary
  Rust regressions for the retained evidence.
- Require future publication runs to verify the GitHub Release attestation and all 11 local assets
  after immutable/latest readback, with bounded retries and deterministic cross-platform tests for
  delayed availability, mismatch, exhaustion, incomplete inventory, and duplicate names. Make the
  bilingual Pages updater reject Release API assets without exact positive lengths and SHA-256
  digests.
- Add a repository-wide Markdown contract that discovers exact-case relative links and local heading
  anchors while rejecting links, non-files, traversal outside the repository, and generated trees.
- Make the bug, feature, and pull-request templates fully bilingual and private-report aware; fix the
  project-root and custom-404 language metadata, heading association, and destination-language links;
  and lock those contribution and Pages contracts into ordinary tests.
- Give every repository-defined GitHub Actions job an explicit timeout, disable persisted checkout
  credentials, and add a regression for the exact regular workflow inventory, all 28 full-SHA action
  references, all 10 checkout settings, forbidden privileged triggers, and write permissions. Record
  a successful current-main five-target bounded fuzz campaign.
- Refresh the fuzz-only `cc` and `find-msvc-tools` patch releases while leaving production
  dependencies and the pinned `libfuzzer-sys` unchanged; pass the locked workspace, dependency
  policy, deterministic regression, native x64/ARM64 CI, CodeQL, and a five-target sanitizer campaign.
- Give double-click shell warnings and failures Japanese and English outcome summaries while
  retaining the underlying technical details, with regression coverage for every error category.

## 0.6.2 - 2026-08-15

- Decouple completed immutable-release evidence from the crate's not-yet-published version so release
  preparation validates an explicit bounded snapshot list; new release evidence is added only after
  its public assets, attestations, tag, workflows, and Pages result have been independently checked.
- Preserve a second independently reviewed exact-`main` Windows evidence run as 11 canonical JSON
  reports with original/canonical hashes and source artifact digests. Extend the ordinary snapshot
  regression to reject extra JSON and validate schema-v2 Settings keyboard evidence: real
  forward/reverse Tab, Enter, and Escape on Server 2022/2025, plus explicitly non-key hosted-ARM64
  fallback results without overstating them as physical-keyboard proof.
- Correct the detailed preview and Attachment Services documents to mark their completed GitHub
  issues closed while keeping graphical-browser and real-provider interoperability work explicitly
  unclaimed; add a regression that rejects those stale open-state statements.
- Exercise the native Settings keyboard path with real Win32 input in Japanese and English: require
  exact forward Tab and reverse Shift+Tab traversal across all 26 controls, both wrap boundaries,
  expected process/AutomationId focus, and automatic scrolling that keeps every focused control
  visible. In the same foreground process, require real Enter-to-save, real Enter to dismiss the
  saved message, and real Escape to request a dirty close whose confirmation is cancelled, while
  requiring the exact saved value. Lock the PowerShell order to a platform-neutral Rust contract and
  record schema-v2 machine-readable traversal and shortcut evidence. Keep real key input mandatory
  on x64 and non-hosted ARM64;
  record an explicit non-key `AttachThreadInput` focus fallback only for the GitHub-hosted ARM64
  runner, whose spawned window exposes no foreground/global UIA focus.
- Preserve the independently verified `v0.6.1` publication as a bilingual, machine-readable release
  snapshot with the exact tag and commit, all 11 public asset hashes and API identities, dry/release/
  tag-CI artifact digests, nine attestation subjects, unsigned PE boundary, and an ordinary Rust
  regression that rejects inventory, digest, workflow, architecture, or version drift.

## 0.6.1 - 2026-08-15

- Declare Per-Monitor V2 DPI awareness in every Windows executable and make Settings consume the
  `WM_DPICHANGED` suggested rectangle, rebuild fonts from the effective DPI, and relayout from a
  stable 96-DPI baseline. Extend native Settings evidence with an effective-awareness assertion and
  a drift-free synthetic 96→144→96 transition.
- Retain native ARM64 and fixed Windows Server evidence artifacts for 90 days, and preserve the
  independently reviewed exact-main run as 11 canonical machine-readable reports with original and
  canonical SHA-256 inventories, source artifact digests, bilingual scope notes, and an ordinary Rust
  regression that rechecks the principal security assertions.

## 0.6.0 - 2026-08-15

- Add a one-use, AppContainer-preserving anonymous-pipe password channel and sealed internal
  libarchive extractor for encrypted ZIP `preview` and `extract`, with no password-value CLI option,
  environment/config/file/log storage, or unsandboxed fallback.
- Add a bounded bilingual native password dialog, non-`Clone` zeroizing UTF-16/UTF-8 secret
  storage, explicit inherited-handle allowlisting, child token self-verification, pre-creation entry
  rejection, exact separator/alias validation, and fail-closed EOF, timeout, overflow, crash,
  wrong-password, and cancellation paths.
- Deliver the bounded password into a dedicated 4 KiB pipe after external token verification while
  the child remains suspended, close-delimit it without a synchronous flush, and only then resume
  the child, preventing the reader/flush deadlock observed by native ARM64 E2E.
- Follow the documented Windows profile-deletion recovery contract with a sub-second bounded retry;
  persistent AppContainer cleanup failures remain fatal and are never reported as success.
- Expand Windows evidence to schema v5 with native-UI ZipCrypto, WinZip AES-128, and AES-256
  preview/extraction, complete tree comparison, public-fixture password output-absence checks, and
  wrong-password/cancel non-publication. Require standard UI Automation `InvokePattern` exposure,
  set the cross-process protected edit through bounded `WM_SETTEXT` rather than the in-process-only
  `SetWindowTextW` helper,
  drive the real window procedure through the button's bounded synchronous `WM_COMMAND` / `BN_CLICKED`
  notification, and require process-identity-checked dialog closure within a bounded interval so a
  recycled HWND cannot cause a false timeout.
  Keep generator-side fixture passwords ASCII because the stock third-party `bsdtar.exe` has no
  iroha-zip UTF-8 manifest; the native one-use-channel probe independently covers Japanese input.
- Record the independently verified immutable `v0.5.3` publication, exact 11-asset inventory,
  tag-ref attestations, intentionally unsigned PE state, and byte-matched bilingual Pages deployment.
- Record the reviewed test-only CodeQL alert #234 and refresh issue-reporting guidance for `v0.5.3`.
- Document the official Microsoft Visual C++ v14 runtime prerequisite discovered by inspecting every
  published x64/ARM64 executable import table.

## 0.5.3 - 2026-08-14

- Add a crate-version-derived regression for the bilingual Pages fallback, download example,
  language topology, release hooks, skip links, and section structure.
- Add a restrictive static-site CSP, referrer policy, SVG favicon, language-aware sitemap, crawler
  policy, and project-root-safe 404 links.
- Gate Pages deployment on a read-only pinned-toolchain validation job; grant Pages and OIDC write
  permissions only to the dependent deployment job.
- Correct dark-mode contrast for primary actions, the warning icon, and footer content, and lock the
  foreground/background theme tokens into the Pages regression contract.
- Accept dynamic download links only from a stable, immutable Release with the exact 11 uploaded
  asset names and expected GitHub download URLs; otherwise retain static fallbacks.
- Exercise valid and malformed Release inventories plus persisted language selection in a
  dependency-free Node regression on Linux, Windows, and the Pages deployment gate.
- Enable hosted CodeQL extended analysis for Rust, Actions, and JavaScript; record the reviewed
  same-user/local-path baseline without disabling future path-flow alerts.
- Package every document linked by the bilingual guides and reject missing release-document targets
  in ordinary CI before the native Release build.
- Document the measured Windows PE/PDB reproducibility boundary so dry-run digests are never reused
  as expected published-asset digests.
- Bound every MSYS2 backend-export child command independently, preserve batched arguments through
  the timeout launcher, and report the stalled phase before the enclosing CI job limit is reached.

## 0.5.2 - 2026-08-14

- Correct the packaged Japanese/English security policy so the supported line is `0.5.x` and
  `0.4.x` and earlier are identified as historical releases.
- Add a normal-CI regression that derives the release tag and supported line from
  `CARGO_PKG_VERSION` and rejects stale packaged release-document versions.
- Refresh bilingual release, ARM64, build, issue, website, and issue-template evidence after the
  independently verified immutable `v0.5.1` publication.
- Keep the runtime archive and sandbox behavior unchanged from `v0.5.1`; this patch exists so the
  corrected security policy is present inside the immutable x64 and ARM64 packages.

## 0.5.1 - 2026-08-14

- Add two-pass AppContainer handling for standalone GZ, BZ2, XZ, Zstandard, and UNIX compress
  streams. A manifest-pinned libarchive raw reader drains the complete preflight, fixes the expected
  filter and safe output name from the outer extension, repeats extraction in a fresh sandbox, and
  fails closed on filter mismatch, decode failure, byte limits, or existing output.
- Add byte-exact, bounded-decoder fixtures from the official libarchive 3.8.9 tag for RAR, RAR5,
  LHA level 3, and BZIP2-compressed ZIPX, with upstream license and provenance records.
- Expand native ARM64 and fixed Server E2E to 14 additional read formats, three raw-stream negative
  cases, and standalone-stream double-click shell extraction with complete tree/hash comparison.
- Inject a disposable-Windows backend replacement failure after backup creation and require exact
  restoration, zero transaction residue, and a subsequent successful import.

## 0.5.0 - 2026-08-14

- Add a deterministic Windows regression that replaces an audited directory with a real NTFS
  junction and requires the post-audit link rejection before any destination is published.
- Add a controlled Microsoft-signed `makecab.exe` LZX fixture to the fixed Server 2022/2025 matrix;
  preview and extraction must reproduce the source SHA-256 tree and remove the temporary root.
- Extend the signature-verifying MSYS2 backend exporter and evidence validator to CLANGARM64, and
  run the verified native ARM64 backend through the archive, malicious-corpus, shell, Settings, and
  AppContainer/LPAC evidence contracts.
- Publish architecture-separated x64 and native ARM64 packages, standalone executables, sidecars,
  combined checksums, and attestations; reject PE machine mix-ups in direct assets and expanded ZIPs.
- Select UCRT64 or CLANGARM64 automatically from the native Settings build when importing from MSYS2.
- Require every supported UCRT64/CLANGARM64 backend EXE/DLL to match its exact x64/ARM64 PE machine.

## 0.4.1 - 2026-08-14

- Create every sandboxed Windows child suspended, assign it to the Job Object, verify the requested
  AppContainer/LPAC token and zero-capability state, and resume it exactly once only after positive
  verification. Terminate the still-suspended Job on verification failure.
- Add a Windows regression that forces a two-second token-verification failure and requires the
  rejected child to produce no stdout.
- Extend schema-v4 isolation evidence with abnormal child termination and corrupt-PE loader rejection,
  requiring explicit cleanup of all seven AppContainer profiles and temporary roots.
- Record exact fail-closed LPAC results on fixed Server 2022/2025 images without treating an unsupported
  token query as a passing LPAC launch or falling back to normal AppContainer.
- Preserve unrelated values in shared file-association registry keys and verify idempotent registration,
  removal, and byte-for-byte `UserChoice` state preservation across all 18 archive extensions.
- Add native Windows ARM64 tests, Clippy, three-PE inventory validation, and normal-AppContainer isolation
  evidence while explicitly withholding ARM64 release assets until the backend/archive matrix exists.
- Harden future immutable release publication with draft-first upload and exact pre/post-publication
  asset state, name, length, and digest verification.
- Document the disabled-until-signed updater trust, downgrade, rollback, and backend-separation contract.

## 0.4.0 - 2026-08-14

- Updated the SHA-1/SHA-256 implementation dependencies to the compatible
  `sha1`/`sha2` 0.11.0 generation.
- Replaced AppContainer filesystem-tree conversion with a bounded path that copies only audited regular objects into a unique external staging tree, seals every object read/execute-only to the Package SID, serializes a handle-pinned PAX stream in the trusted parent, and gives the backend only fixed `@source.pax.tar`.
- Give libarchive's 7z writer a dedicated resource-monitored AppContainer scratch directory with only the required temporary read/write/delete boundary, and reject any residue after process exit.
- Work around libarchive's Windows 3.8.6+ UTF-8 filename regression with a bounded UTF-8 pathname listing child that loads manifest-pinned DLL candidates only after rechecking zero-capability AppContainer isolation; explicitly request UTF-8 ZIP/PAX header names and retain the byte-verified UTF-8 process manifest on disposable backend copies.
- Localize the complete native Settings surface in Japanese and English, follow the Windows UI language, provide an explicit process-local override, and exercise both languages through UI Automation.

- Added an accessible Japanese/English GitHub Pages site and complete English project guide.
- Publish a stable unsigned Windows x64 release with the ZIP, three individual executables, SHA-256 inventories, and GitHub artifact attestations.
- Document unsigned-binary and SmartScreen expectations without weakening verification guidance.

- Re-extract every generated archive through a second sandbox before publication; require safe raw members, exact source-tree reproduction, unchanged staging/source fingerprints, and the same generated-archive identity/hash through final create-new copy.
- Seal the Windows staging source against its ephemeral AppContainer Package SID before archive creation, and record read success plus root/nested write, create, rename, delete, attribute, DACL, and owner denial in machine-readable isolation evidence.
- Enumerate Windows tree members through bounded directory handles, compare directory identities before/after enumeration and between audit/copy, and reject same-name empty-directory replacement.
- Serialize initial configuration creation under the same save lock and exercise simultaneous non-ASCII-path saves from independent Windows processes.
- Verify that the Windows configuration-save mutex fails closed on timeout and recovers after an owning thread exits without releasing it.
- Restore the previous configuration after a replacement failure and preserve/report its named recovery backup if restoration itself fails.
- Extend native UI Automation through all three safe folder-picker cancellation paths, both Restore Defaults decisions, and both unsaved-change Cancel decisions.
- Split formal Windows releases into validated build, Azure OIDC Authenticode signing, fail-closed publisher/EKU/timestamp verification, and packaging phases; attach signature evidence and offline-verifiable SLSA provenance only through an immutable-release gate.

- Added a deterministic, source-generated malicious ZIP/TAR corpus with 18 reject cases, one benign control, native Windows hardlink/ADS/junction fixtures, JSON-only evidence, and mandatory temporary-root cleanup.
- Added a sandboxed, 64 MiB-bounded raw-name preflight (`bsdtar -t` on Unix and libarchive's UTF-8 pathname API on Windows) that rejects absolute/drive/UNC names, unsafe Windows components, depth/path violations, and case-aliasing duplicate members before extraction can normalize or overwrite them.
- Added fixed-label Windows Server 2022/2025 E2E jobs with machine-readable evidence for verified-backend setup, zero-capability AppContainer tokens, loopback denial, timeout, memory limits, cleanup, archive creation/preview/re-extraction, invalid input, shell invocation, and settings-screen setup.
- Require explicit AppContainer profile/root cleanup on successful create, extract, preview, shell, and doctor operations and on backend launch, timeout, resource, or nonzero-exit failures; cleanup failures are no longer silently hidden.
- Extended `doctor` and the dedicated isolation report with measured token flags and capability counts instead of reporting only the requested isolation mode.
- Added an opt-in zero-capability LPAC prototype, exposed in the settings screen and configuration file.
- Verify the created child token is an AppContainer and, when requested, a less-privileged AppContainer; unsupported or downgraded launches fail closed.
- Documented the fail-closed ConPTY design required for future encrypted-archive input without command-line, environment, log, or configuration exposure.
- Added opt-in Windows Attachment Services handoff policies (`disabled`, `best-effort`, and `required`) to configuration and the settings screen.
- Run enabled handoff on the staged tree before publication, then re-audit file identities, links, reparse points, ADS, content hashes, tree structure, and Mark-of-the-Web before the final rename.
- Report Attachment Services acceptance separately from structural validation and never describe it as a clean malware verdict.
- Added a System-DPI-aware, resizable settings layout with 100–300% logical scaling, horizontal/vertical scrolling, and automatic focus visibility.
- Assigned stable IDs and unique access keys to every setting and action, with exhaustive action dispatch tests and a native Windows UI Automation smoke test.
- Serialize concurrent configuration saves before rollback-safe replacement, using a named Windows mutex across processes in the current session.
- Include every document linked from the packaged README in release ZIPs.
- Refactored extraction into a shared staging boundary used by normal extraction and policy-safe preview.
- Added a `preview` CLI that inventories only a fully extracted, audited, fingerprint-stable temporary tree and publishes nothing.
- Added repeatable `extract --select PATH` filtering after the complete archive audit, followed by selected-tree and source-tree revalidation before the existing atomic publication boundary.
- Make verified backend copies owner-readable/executable on Unix so the explicit unsandboxed integration-test path can exercise real process orchestration.
- Added five pinned `cargo-fuzz` targets, a bounded weekly sanitizer workflow, seed isolation, failure artifacts, and SHA-256-addressed deterministic regression promotion.
- Refactored Windows command-line quoting into a platform-neutral, property-tested encoder that rejects interior NUL, invalid program names, and the `CreateProcessW` length overflow before launch.
- Archive-derived destination names now fall back to `archive` when extension removal would produce a Windows-invalid or reserved component.
- Added atomic backend provenance bundles containing a strict source record, exact payload ownership, SPDX 2.3 SBOM, license inventory, notices, and copied license evidence.
- The supported MSYS2 UCRT64 exporter now refreshes isolated signed databases under `Required TrustedOnly`, records package signatures/archive hashes, verifies current versions, and compares imported bytes with signed package archives.
- Arbitrary backend bundles now require explicit unsupported-source approval; diagnostics warn permanently, and private backend packaging rejects unsupported or inconsistent evidence by default.

## 0.3.1 - 2026-08-09

- Added handle-retaining SHA-256 snapshots for input archives and compression source files.
- On Windows, deny write/delete sharing while source handles are retained and reject reparse points and multi-link files through the open handle.
- On Unix, use `O_NOFOLLOW` for source snapshots and reject multi-link files through handle metadata.
- Replace size-only source-tree comparisons with deterministic path, type, length, and content fingerprints.
- Reject identity replacement, timestamp/length changes, same-size content mutation, links, and source/target root escapes during audited copy, removing partial output on failure.
- Added deterministic full-copy race injection tests and snapshot regression tests for replacement, same-size mutation, rename, hardlinks, and symbolic links.

## 0.3.0 - 2026-08-09

- Refactored backend-manifest parsing into a bounded, platform-neutral API suitable for deterministic tests and future fuzzing.
- Reject oversized manifests, excessive file records, invalid UTF-8, unsafe Windows names, non-normalized paths, ambiguous executables, duplicate paths, and malformed hashes before backend-tree verification.
- Added a versioned backend-manifest specification and regression coverage for valid and hostile parser inputs.
- Unified the package, application, service, executable, configuration, Windows integration, sandbox, backend manifest, and release artifact names under `iroha-zip`.
- Renamed the Rust library crate to `iroha_zip` and its public error type to `IrohaZipError`.
- Moved the default configuration directory to `%LOCALAPPDATA%\iroha-zip` on Windows and the corresponding `iroha-zip` directory on other platforms.
- Renamed the CLI, shell integration, and settings executables to `iroha-zip.exe`, `iroha-zip-shell.exe`, and `iroha-zip-settings.exe`.

## 0.2.1 - 2026-08-09

- Refactored settings form conversion and validation into a platform-neutral, unit-tested module.
- Added human-readable binary size units, field-specific validation focus, and lossless config round trips.
- Added unsaved-change tracking, close confirmation, progress states, destructive-action confirmation, and keyboard dialog navigation to the settings screen.
- Linked the documented security, QA, platform, operations, and UX backlog to GitHub issues.

## 0.2.0 - 2026-08-09

- Added a native Windows settings application covering every configuration field.
- Added backend folder selection, verified bundle import, MSYS2 import, and full diagnostics to the settings screen.
- Added file-association registration/removal and Windows Default Apps shortcuts to the settings screen.
- Added a configurable default filename encoding for shell and CLI extraction, with per-command override.
- Added validated, rollback-safe configuration replacement and expanded configuration tests.
- Added reproducible backend-free Windows release packages and tag-driven GitHub releases.
- Added a checked dependency policy and generated third-party license inventory to release packages.
- Added `iroha-zip settings` as the supported settings entry point.

## 0.1.0 - 2026-08-08

- Initial Rust implementation.
- Ephemeral Windows AppContainer execution for both extraction and archive creation.
- Job Object limits for memory, process count, and timeout.
- Live full-sandbox filesystem monitoring with baseline-aware file, directory, and byte budgets.
- SHA-256-pinned libarchive backend bundle.
- ZIP/LHA filename encoding override for UTF-8, CP932, and CP437.
- Path, link, reparse point, ADS, hardlink, Windows device-name, and archive-bomb checks.
- Mark-of-the-Web propagation.
- ZIP, 7z, TAR, and TAR.GZ creation.
- Per-user Windows file-association scripts.
