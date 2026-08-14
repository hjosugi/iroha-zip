# Windows end-to-end evidence

Updated: 2026-08-14

This document defines the automated SAFE-001 evidence contract. The workflow and harness have run on both fixed-label GitHub runners. An exact in-container `GetTempPathW`, CNG random-number, and delete-on-close file probe traced the reproducible libarchive 7z error to applying AppContainer temporary-path virtualization twice. The next run supplies the host LocalAppData temporary path and requires Windows to resolve it to the one dedicated `AC\Temp` scratch. A failing diagnostic artifact is not a passing Windows result.

## Automated matrix

The `windows-e2e` CI job uses explicit `windows-2022` and `windows-2025` x64 labels. GitHub currently maps those labels to Windows Server 2022 and Windows Server 2025 virtual machines. They are disposable real-Windows kernels, but they are not Windows 10 or Windows 11 desktop evidence. GitHub also notes that `-latest` can migrate, so this matrix deliberately uses fixed labels.

Primary runner references:

- [GitHub-hosted runners reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [GitHub Actions runner images and labels](https://github.com/actions/runner-images#available-images)

Each job builds the release executables, exports a current MSYS2 UCRT64 libarchive bundle through the signed-package evidence path, and runs the matrix below. The Server 2025 job also reuses that export for supported/unsupported provenance, rollback, strict-source, and evidence-tamper gates; CI does not perform a redundant third export in the fast test matrix.

```powershell
./scripts/test-windows-e2e.ps1 `
  -Executable ./target/release/iroha-zip.exe `
  -ShellExecutable ./target/release/iroha-zip-shell.exe `
  -BackendDirectory $verifiedBackend `
  -EvidenceOutput $evidence
```

It then runs the generated malicious archive corpus and `test-settings-ui.ps1` with the same verified backend. The three JSON reports are uploaded as `windows-e2e-windows-2022` or `windows-e2e-windows-2025` artifacts for 14 days. Generated hostile ZIP/TAR files are deleted and are never uploaded; see the [corpus contract](MALICIOUS_CORPUS.md).

The harness invokes verified `bsdtar.exe` directly only to generate BZ2/XZ/Zstandard/compress fixtures from deterministic, harness-owned data on the disposable runner. All reads of those archives and every product create operation still pass through iroha-zip's AppContainer boundary. Direct backend execution is not a supported path for untrusted or user-owned input.

## Assertions

The archive harness fails the job unless all of these checks pass:

| Area | Automated assertion |
|---|---|
| Backend | Manifest, provenance, SPDX, package verification, license inventory, notices, and evidence hashes pass with `--require-supported`. |
| AppContainer token | The child token reports `TokenIsAppContainer`, not LPAC for the default mode, and exactly zero capabilities. |
| Network | A copied, byte-identical probe cannot connect to an active parent-owned loopback listener. |
| Timeout | A 5-second child is terminated by a 250-millisecond sandbox timeout. |
| Memory | A child requesting and touching 256 MiB fails inside a Job Object limited to 64 MiB. |
| Process temp | A child resolves its effective Windows temporary path, obtains random bytes through CNG, and creates and removes a file using libarchive's read/write/delete and delete-on-close access pattern. Its dedicated temporary directory must be empty after exit. |
| Staging source | The source is staged outside the AppContainer's intrinsically writable profile storage, then receives a protected, inheritable Package SID read/execute-only ACE. Its parent receives only non-inheriting traversal/list access. A byte-identical child can enumerate the parent/root/nested directories and read nested data, but cannot overwrite, append, create in the parent or source, rename, delete, change file attributes, open the DACL for writing, or open the owner for writing. |
| Create input | The trusted parent copies only audited regular objects into a unique external staging tree, compares the complete source fingerprint, and protects every file/directory DACL individually. It serializes that sealed tree into a bounded PAX stream held by identity/length/SHA-256; the backend receives only fixed sandbox-local `@source.pax.tar`, never the normal source path or a filesystem-tree operand. A dedicated monitored scratch grants the 7z writer only the temporary read/write/delete boundary it needs and must be empty after process exit. |
| UTF-8 backend | A dedicated listing child loads only verified-manifest DLL candidates after rechecking AppContainer and zero-capability token state, then reads `archive_entry_pathname_utf8`. ZIP/PAX creation explicitly requests UTF-8 headers. After exact source and sandbox-copy hashes pass, the temporary backend EXE also receives a fixed `asInvoker`/long-path/UTF-8 process manifest whose resource is read back byte-for-byte. Japanese names in every create/read path must survive on the default English runner locale. |
| Cleanup | All five probe profiles, their profile roots, and their owned external staged-source roots are absent after explicit cleanup. |
| Doctor | A real `bsdtar --version` run reports measured AppContainer and zero-capability evidence. |
| Create/read | ZIP, 7z, TAR, and TAR.GZ are converted from the trusted bounded PAX stream, internally re-extracted in a second AppContainer before publication, then independently previewed, extracted, and compared by relative path, type, length, and SHA-256 by the harness. The backend bundle in every sandbox is recursively read/execute-only to its Package SID. |
| Pass separation | The sandbox archive copy remains handle-pinned and recursively Package-SID read-only across listing/extraction. The extraction directory is absent during listing and is created new only after the child exits, policy accepts the list, and the archive fingerprint still matches. |
| Additional read filters | The verified backend creates controlled TAR.BZ2, TAR.XZ, TAR.ZST, and TAR.Z fixtures; iroha-zip previews and extracts them to the same tree hash. |
| Paths | The source includes Japanese names, an empty directory, deterministic binary data, and a relative path longer than 260 characters. |
| Directory handles | Windows unit tests enumerate Unicode names through a bounded directory handle, reject an undersized entry budget, and require the retained handle to block directory rename. The archive matrix then exercises the same enumeration path over nested and long-path trees. |
| Failure | A deliberately invalid ZIP exits nonzero, publishes no destination, and takes the cleanup-required backend-failure path. |
| Malicious corpus | One control extracts, 18 generated hostile archives fail before destination publication, hardlink/ADS/junction fixtures return policy errors, and the temporary root is removed. |
| Shell | `iroha-zip-shell.exe` uses an isolated `%LOCALAPPDATA%` configuration and produces a hash-identical sibling tree. |
| Settings | UI Automation reaches all 26 controls, observes dirty-state confirmation, saves the real backend path, runs the settings-screen backend/AppContainer diagnosis, and removes its temporary configuration tree. |

Normal create, preview, extract, shell, and doctor success now call explicit sandbox cleanup. Backend launch, timeout, resource-monitor, and nonzero-exit failures also attempt cleanup before returning the original failure; a cleanup failure is combined into the returned error instead of being hidden by `Drop`.

## Evidence format

`windows-e2e.json` is UTF-8 JSON with `schemaVersion: 1`. Its nested isolation report uses `schemaVersion: 3`. It records:

- runner OS/image identity and executable/backend-manifest SHA-256 values;
- source-tree counts, byte total, manifest hash, and longest path length;
- token, network, timeout, memory, effective process-temp path, CNG/delete-on-close results, staging-source read/write ACL, profile deletion, and root deletion evidence;
- per-format archive size/hash, tree-manifest hash, operation durations, and controlled additional read-filter fixtures;
- invalid-input publication result and shell extraction result;
- final harness-root cleanup and any failure message.

`malicious-corpus.json` records generated archive hashes and lengths, expected results, exit/rejection classes, publication booleans, native policy fixtures, and cleanup. `settings-e2e.json` records the settings executable hash, saved configuration hash, control count, safe folder-picker cancellation count, Restore Defaults/Cancel confirmation paths, save/doctor results, elapsed time, and cleanup result. These artifacts are diagnostic evidence, not release attestations or signatures; SAFE-003 tracks authenticated release provenance.

## Remaining SAFE-001 work

The automated Server matrix does not close SAFE-001. Still required:

1. run the same contract on disposable Windows 10 and Windows 11 x64 machines;
2. add legally redistributable read fixtures for RAR/RAR5, LHA/LZH, CAB, ZIPX, and raw compressed streams through SAFE-002;
3. record LPAC format and broader filesystem/registry/COM/LAN/Internet denial results;
4. exercise crash/loader failure and Windows reparse-race paths;
5. preserve reviewed evidence outside the 14-day CI artifact window;
6. complete visual DPI, keyboard-only, and screen-reader validation on desktop Windows.

Do not describe the matrix as a sandbox audit, malware verdict, Windows 10/11 certification, or proof for formats that are not named in a passing report.
