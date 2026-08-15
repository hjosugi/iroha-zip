# Windows end-to-end evidence

Updated: 2026-08-15

This document defines the automated SAFE-001 evidence contract. The expanded schema-v4 contract passed on both fixed-label GitHub runners and the native `windows-11-arm` runner in [Actions run 31778764604](https://github.com/hjosugi/iroha-zip/actions/runs/31778764604) from commit `27610e69f21bf85709f70a68695acc1113d22dca`; the native ARM64 path also passed independently on push in [Actions run 31778405711](https://github.com/hjosugi/iroha-zip/actions/runs/31778405711). The active schema-v5 harness retains that matrix and adds the encrypted-ZIP/password assertions below. Independently downloaded schema-v4 JSON records one effective `AC\Temp` path, successful in-container CNG and delete-on-close probes, abnormal-exit and corrupt-loader rejection, seven explicitly removed AppContainer profiles/roots per environment, all 14 named additional read formats and three raw-stream negative cases below, the generated malicious corpus, x64 English Settings, and native ARM64 Japanese/English Settings. These results are evidence for the named disposable Server and hosted Windows 11 ARM images, not Windows 10/11 x64 desktop certification or a security audit.

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

The harness invokes verified `bsdtar.exe` directly only to generate BZ2/XZ/Zstandard/compress and encrypted-ZIP fixtures from deterministic, harness-owned data on the disposable runner. It generates both PAX-containing TAR filters and standalone GZ/BZ2/XZ/Zstandard/compress streams, plus ZipCrypto, WinZip AES-128, and WinZip AES-256 ZIPs. The deliberately public encrypted-fixture sentinel is the only E2E use of generator-side `--passphrase`; product password handling never puts a secret on a command line. The generator uses a separate ASCII-only path because the unmodified source binary has no iroha-zip UTF-8 process manifest; trusted PowerShell then copies the result to a Japanese-named product input path. It also generates one single-file LZX CAB with the OS `System32\makecab.exe`, but only after requiring a valid Microsoft Authenticode signature; the report records the generator file hash, signer subject, compression setting, and output hash. All reads of those archives and every product create operation still pass through iroha-zip's AppContainer boundary. Direct generator execution is not a supported path for untrusted or user-owned input.

RAR, RAR5, LHA level 3, and BZIP2-compressed ZIPX use four benign reference fixtures copied byte for byte from the official libarchive `v3.8.9` tag. The repository stores only their UUencoded upstream form together with the upstream BSD-2-Clause terms. A bounded PowerShell decoder requires the exact encoded hash, envelope, decoded length, and decoded hash before creating a temporary archive. CI never asks host-side `bsdtar` to list or extract these inputs. iroha-zip performs both `preview` and `extract` through AppContainer, then the harness compares every path, object kind, byte length, and file SHA-256 with a pinned expected inventory. Exact provenance and hashes are in [`tests/fixtures/libarchive-v3.8.9`](../tests/fixtures/libarchive-v3.8.9/README.md).

## Assertions

The archive harness fails the job unless all of these checks pass:

| Area | Automated assertion |
|---|---|
| Backend | Manifest, provenance, SPDX, package verification, license inventory, notices, and evidence hashes pass with `--require-supported`. |
| AppContainer token | The child token reports `TokenIsAppContainer`, not LPAC for the default mode, and exactly zero capabilities. |
| Launch gate | Each child is created suspended and assigned to its Job; the parent positively verifies the requested token mode and zero capabilities before one exact resume. A Windows regression delays a forced verification failure for two seconds, requires empty child stdout, and cleans the rejected sandbox. |
| Network | A copied, byte-identical probe cannot connect to an active parent-owned loopback listener. |
| Timeout | A 5-second child is terminated by a 250-millisecond sandbox timeout. |
| Memory | A child requesting and touching 256 MiB fails inside a Job Object limited to 64 MiB. |
| Crash and loader failure | A positively verified in-sandbox child aborts with a nonzero status, while a deliberately corrupted PE is rejected by process creation. Both paths must perform explicit profile/root cleanup. |
| Process temp | A child resolves its effective Windows temporary path, obtains random bytes through CNG, and creates and removes a file using libarchive's read/write/delete and delete-on-close access pattern. Its dedicated temporary directory must be empty after exit. |
| Staging source | The source is staged outside the AppContainer's intrinsically writable profile storage, then receives a protected, inheritable Package SID read/execute-only ACE. Its parent receives only non-inheriting traversal/list access. A byte-identical child can enumerate the parent/root/nested directories and read nested data, but cannot overwrite, append, create in the parent or source, rename, delete, change file attributes, open the DACL for writing, or open the owner for writing. |
| Create input | The trusted parent copies only audited regular objects into a unique external staging tree, compares the complete source fingerprint, and protects every file/directory DACL individually. It serializes that sealed tree into a bounded PAX stream held by identity/length/SHA-256; the backend receives only fixed sandbox-local `@source.pax.tar`, never the normal source path or a filesystem-tree operand. A dedicated monitored scratch grants the 7z writer only the temporary read/write/delete boundary it needs and must be empty after process exit. |
| UTF-8 backend | A dedicated listing child loads only verified-manifest DLL candidates after rechecking AppContainer and zero-capability token state, then reads `archive_entry_pathname_utf8`. ZIP/PAX creation explicitly requests UTF-8 headers. After exact source and sandbox-copy hashes pass, the temporary backend EXE also receives a fixed `asInvoker`/long-path/UTF-8 process manifest whose resource is read back byte-for-byte. Japanese names in every create/read path must survive on the default English runner locale. |
| Encrypted ZIP | The verified backend generates controlled ZipCrypto, AES-128, and AES-256 ZIPs. UI Automation requires the exact bilingual native dialog, an enabled/focusable protected edit, and accessible confirm/cancel buttons. iroha-zip previews and extracts each variant through its one-use ConPTY/AppContainer path and reproduces the complete SHA-256 tree. A Japanese public sentinel must be absent from stdout/stderr. Wrong-password exit and explicit cancellation must publish no destination; cancellation must produce no output. |
| Cleanup | All seven probe profiles, their profile roots, and their owned external staged-source roots are absent after explicit cleanup. |
| Doctor | A real `bsdtar --version` run reports measured AppContainer and zero-capability evidence. |
| Create/read | ZIP, 7z, TAR, and TAR.GZ are converted from the trusted bounded PAX stream, internally re-extracted in a second AppContainer before publication, then independently previewed, extracted, and compared by relative path, type, length, and SHA-256 by the harness. The backend bundle in every sandbox is recursively read/execute-only to its Package SID. |
| Pass separation | The sandbox archive copy remains handle-pinned and recursively Package-SID read-only across listing/extraction. The extraction directory is absent during listing and is created new only after the child exits, policy accepts the list, and the archive fingerprint still matches. |
| Additional reads | The verified backend creates controlled TAR.BZ2, TAR.XZ, TAR.ZST, and TAR.Z fixtures plus standalone GZ, BZ2, XZ, Zstandard, and compress streams. The raw-stream path derives one safe output name from the outer archive, drains the entire input during preflight, requires the exact extension-selected filter, and repeats the operation in a fresh sandbox. Separate negative cases require non-publication for deliberate gzip-as-XZ bytes, a 41-byte expansion under a 32-byte limit, and an invalid gzip compressed payload that libarchive reports as a decode failure. Validly Microsoft-signed `makecab.exe` creates a controlled LZX CAB. Four pinned upstream fixtures cover RAR, RAR5, LHA level 3, and BZIP2-compressed ZIPX. iroha-zip previews and extracts the exact 14-format inventory and compares complete trees. |
| Paths | The source includes Japanese names, an empty directory, deterministic binary data, and a relative path longer than 260 characters. |
| Directory handles | Windows unit tests enumerate Unicode names through a bounded directory handle, reject an undersized entry budget, and require the retained handle to block directory rename. The archive matrix then exercises the same enumeration path over nested and long-path trees. |
| Failure | A deliberately invalid ZIP exits nonzero, publishes no destination, and takes the cleanup-required backend-failure path. |
| Malicious corpus | One control extracts, 18 generated hostile archives fail before destination publication, hardlink/ADS/junction fixtures return policy errors, and the temporary root is removed. |
| Shell | `iroha-zip-shell.exe` uses an isolated `%LOCALAPPDATA%` configuration and produces a hash-identical sibling tree for both a normal ZIP and a standalone gzip stream; its copied internal child dispatches raw reads without opening UI. |
| Settings | UI Automation reaches all 26 controls, observes dirty-state confirmation, saves the real backend path, runs the settings-screen backend/AppContainer diagnosis, and removes its temporary configuration tree. |

Normal create, preview, extract, shell, and doctor success now call explicit sandbox cleanup. Backend launch, timeout, resource-monitor, and nonzero-exit failures also attempt cleanup before returning the original failure; a cleanup failure is combined into the returned error instead of being hidden by `Drop`.

## Evidence format

`windows-e2e.json` is UTF-8 JSON with `schemaVersion: 5`. Its nested isolation report remains independently versioned at `schemaVersion: 4`. It records:

- runner OS/image identity and executable/backend-manifest SHA-256 values;
- source-tree counts, byte total, manifest hash, and longest path length;
- token, network, timeout, memory, abnormal-exit status, corrupt-loader rejection, effective process-temp path, CNG/delete-on-close results, staging-source read/write ACL, profile deletion, and root deletion evidence;
- per-format archive size/hash, tree-manifest hash, operation durations, controlled additional read-filter fixtures, raw-stream output names/source hashes, and the filter-mismatch non-publication result;
- ZipCrypto/AES-128/AES-256 archive hashes and tree hashes, native-dialog/password-control assertions, public-sentinel output absence, and wrong-password/cancel non-publication;
- for every pinned upstream fixture, the libarchive version, annotated tag object, commit, license, UU filename/length/hash, decoded archive length/hash, and expected extracted-tree hash;
- invalid-input publication result and shell extraction result;
- final harness-root cleanup and any failure message.

`malicious-corpus.json` records generated archive hashes and lengths, expected results, exit/rejection classes, publication booleans, native policy fixtures, and cleanup. `settings-e2e.json` records the settings executable hash, saved configuration hash, control count, safe folder-picker cancellation count, Restore Defaults/Cancel confirmation paths, save/doctor results, elapsed time, and cleanup result. These artifacts are diagnostic evidence, not release attestations or signatures; SAFE-003 tracks authenticated release provenance.

## Remaining SAFE-001 work

The automated Server matrix does not close SAFE-001. Still required:

1. run the same contract on disposable Windows 10 and Windows 11 x64 machines;
2. add further independently redistributable legacy and malformed-format fixtures through SAFE-002;
3. record LPAC format and broader filesystem/registry/COM/LAN/Internet denial results;
4. add concurrent reparse-race stress beyond the deterministic real-junction replacement regression, plus broader backend-specific crash/cancellation cases;
5. preserve reviewed evidence outside the 14-day CI artifact window;
6. complete visual DPI, keyboard-only, and screen-reader validation on desktop Windows.

Do not describe the matrix as a sandbox audit, malware verdict, Windows 10/11 certification, or proof for formats that are not named in a passing report.
