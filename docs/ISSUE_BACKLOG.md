# Issue backlog

Updated: 2026-08-09

This backlog records the remaining work discovered during the v0.2.0 refactor. Each stable ID links to its GitHub issue. Priority is based on security and release risk; an item being listed here does not mean its unsafe behavior is currently enabled.

## P0 — release and security evidence

### [SAFE-001: Real Windows end-to-end matrix](https://github.com/hjosugi/iroha-zip/issues/3)

Run extraction and creation on disposable Windows 10/11 x64 machines with a pinned libarchive bundle. Cover every documented format, Japanese filenames, long paths, timeout, memory exhaustion, cleanup after failure, shell invocation, and settings-screen setup.

Acceptance: the matrix is automated where possible; created archives are re-extracted and content-hashed; AppContainer identity and network denial are recorded; no residual profile or temporary tree remains after every tested exit path.

### [SAFE-002: Malicious archive regression corpus](https://github.com/hjosugi/iroha-zip/issues/4)

Build a legally redistributable local corpus for Zip Slip, absolute/drive/UNC paths, symlinks, hardlinks, junctions, reparse points, ADS, device names, trailing-dot aliases, duplicate file identities, deep paths, sparse expansion, and count/size bombs.

Acceptance: every sample has expected policy output, runs on a disposable worker, cannot publish weaponized payloads as ordinary issue attachments, and fails closed before final publication.

### [SAFE-003: Signed release chain](https://github.com/hjosugi/iroha-zip/issues/5)

Add Authenticode signing, documented certificate custody, signature verification, checksummed artifacts, and release provenance/attestations.

Acceptance: all three executables are signed and verified in CI; ZIP/SHA-256/provenance are attached to an immutable release; verification steps are documented independently of GitHub transport security.

### [SAFE-004: Backend provenance, SBOM, and license evidence](https://github.com/hjosugi/iroha-zip/issues/6)

Define supported libarchive sources and verify package signatures/hashes before import. Produce an SBOM and third-party notices for every included backend file when a private package uses `-IncludeBackend`.

Acceptance: provenance is machine-readable, unsupported sources generate an explicit warning, and the generated manifest/SBOM/license inventory agree exactly with the imported tree.

## P1 — hardening and core capability

### [SAFE-005: Reduce source-tree TOCTOU windows](https://github.com/hjosugi/iroha-zip/issues/7)

Hold handles while validating/copying input archives and compression sources, and compare identity, length, timestamps, and content hashes across the audited copy.

Acceptance: replacement, same-size mutation, rename, hardlink, and reparse races have deterministic regression tests and cannot cause unaudited bytes to enter the sandbox or final archive.

### [SAFE-006: Evaluate LPAC and explicit capabilities](https://github.com/hjosugi/iroha-zip/issues/8)

Prototype LPAC, document OS-version behavior, and measure whether required backend operations work without widening access.

Acceptance: default isolation is never weakened on fallback; unsupported systems fail closed; the threat model documents verified ACL/capability differences.

### [SAFE-007: Secure encrypted-archive input](https://github.com/hjosugi/iroha-zip/issues/9)

Support passwords without command-line, environment, log, crash-report, or persistent-config exposure.

Acceptance: use a protected anonymous channel or equivalent one-use mechanism, zero sensitive buffers where practical, prevent inherited handles, and test cancellation/wrong-password paths.

### [SAFE-008: Defender/antimalware handoff](https://github.com/hjosugi/iroha-zip/issues/10)

Evaluate `IAttachmentExecute`, AMSI, or supported Defender interfaces after publication while preserving Mark-of-the-Web.

Acceptance: scanning cannot silently downgrade fail-closed extraction, engine unavailability has an explicit policy, and results are distinguishable from SafeArc structural validation.

### [UX-001: Settings accessibility and integration automation](https://github.com/hjosugi/iroha-zip/issues/11)

Exercise the native settings application at 100–300% DPI, keyboard-only navigation, Japanese/English Windows, screen readers, long/non-ASCII paths, import failure rollback, association changes, and concurrent saves.

Acceptance: all controls are reachable and labelled, content fits supported displays, destructive/long-running actions show progress or confirmation, and UI automation covers every button and setting.

## P2 — usability and platform breadth

### [UX-002: Archive preview without backend privilege expansion](https://github.com/hjosugi/iroha-zip/issues/12)

Design a read-only file listing and selective extraction flow while treating metadata as untrusted.

Acceptance: listing has the same timeout/resource/path policy, does not parse archives in the main process, and selection cannot bypass post-extraction tree audit.

### [OPS-001: Signed updater](https://github.com/hjosugi/iroha-zip/issues/13)

Design an opt-in updater with rollback, channel selection, signature verification, and no implicit backend replacement.

Acceptance: packages are verified before execution, downgrade policy is explicit, and backend trust remains independently controlled by the user.

### [QA-001: Fuzzing and property tests](https://github.com/hjosugi/iroha-zip/issues/14)

Fuzz manifest parsing, Windows path validation, archive-name normalization, command-line quoting, and configuration round-trips.

Acceptance: reproducible fuzz targets run in CI on a bounded schedule and all minimized regressions become deterministic tests.

### [PORT-001: Windows ARM64 package](https://github.com/hjosugi/iroha-zip/issues/15)

Validate dependency/toolchain support and add a separately checksummed ARM64 artifact.

Acceptance: native ARM64 AppContainer and archive matrix pass; assets cannot be confused with x64 packages; setup and backend sourcing are documented.

## Dependency order

1. SAFE-001 and SAFE-002 establish behavioral evidence.
2. SAFE-003 and SAFE-004 establish release and supply-chain evidence.
3. SAFE-005 through SAFE-008 harden or expand trust boundaries only after regression coverage exists.
4. UX-002, OPS-001, and PORT-001 remain gated by the same signed-release and Windows matrix requirements.
