# Windows end-to-end evidence

Updated: 2026-08-10

This document defines the automated SAFE-001 evidence contract. The workflow and harness are implemented, but the current local branch has not yet produced a GitHub Actions artifact. A parsed workflow is not a passing Windows result.

## Automated matrix

The `windows-e2e` CI job uses explicit `windows-2022` and `windows-2025` x64 labels. GitHub currently maps those labels to Windows Server 2022 and Windows Server 2025 virtual machines. They are disposable real-Windows kernels, but they are not Windows 10 or Windows 11 desktop evidence. GitHub also notes that `-latest` can migrate, so this matrix deliberately uses fixed labels.

Primary runner references:

- [GitHub-hosted runners reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [GitHub Actions runner images and labels](https://github.com/actions/runner-images#available-images)

Each job builds the release executables, exports a current MSYS2 UCRT64 libarchive bundle through the signed-package evidence path, and runs:

```powershell
./scripts/test-windows-e2e.ps1 `
  -Executable ./target/release/iroha-zip.exe `
  -ShellExecutable ./target/release/iroha-zip-shell.exe `
  -BackendDirectory $verifiedBackend `
  -EvidenceOutput $evidence
```

It then runs `test-settings-ui.ps1` with the same verified backend. Both JSON reports are uploaded as `windows-e2e-windows-2022` or `windows-e2e-windows-2025` artifacts for 14 days.

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
| Cleanup | All three probe profiles are deleted and all three temporary roots are absent after explicit cleanup. |
| Doctor | A real `bsdtar --version` run reports measured AppContainer and zero-capability evidence. |
| Create/read | ZIP, 7z, TAR, and TAR.GZ are created, previewed, extracted, and compared by relative path, type, length, and SHA-256. |
| Additional read filters | The verified backend creates controlled TAR.BZ2, TAR.XZ, TAR.ZST, and TAR.Z fixtures; iroha-zip previews and extracts them to the same tree hash. |
| Paths | The source includes Japanese names, an empty directory, deterministic binary data, and a relative path longer than 260 characters. |
| Failure | A deliberately invalid ZIP exits nonzero, publishes no destination, and takes the cleanup-required backend-failure path. |
| Shell | `iroha-zip-shell.exe` uses an isolated `%LOCALAPPDATA%` configuration and produces a hash-identical sibling tree. |
| Settings | UI Automation reaches all 26 controls, observes dirty-state confirmation, saves the real backend path, runs the settings-screen backend/AppContainer diagnosis, and removes its temporary configuration tree. |

Normal create, preview, extract, shell, and doctor success now call explicit sandbox cleanup. Backend launch, timeout, resource-monitor, and nonzero-exit failures also attempt cleanup before returning the original failure; a cleanup failure is combined into the returned error instead of being hidden by `Drop`.

## Evidence format

`windows-e2e.json` is UTF-8 JSON with `schemaVersion: 1`. It records:

- runner OS/image identity and executable/backend-manifest SHA-256 values;
- source-tree counts, byte total, manifest hash, and longest path length;
- token, network, timeout, memory, profile deletion, and root deletion evidence;
- per-format archive size/hash, tree-manifest hash, operation durations, and controlled additional read-filter fixtures;
- invalid-input publication result and shell extraction result;
- final harness-root cleanup and any failure message.

`settings-e2e.json` records the settings executable hash, saved configuration hash, control count, save/doctor results, elapsed time, and cleanup result. These artifacts are diagnostic evidence, not release attestations or signatures; SAFE-003 tracks authenticated release provenance.

## Remaining SAFE-001 work

The automated Server matrix does not close SAFE-001. Still required:

1. run the same contract on disposable Windows 10 and Windows 11 x64 machines;
2. add legally redistributable read fixtures for RAR/RAR5, LHA/LZH, CAB, ZIPX, and raw compressed streams through SAFE-002;
3. record LPAC format and broader filesystem/registry/COM/LAN/Internet denial results;
4. exercise crash/loader failure and Windows reparse-race paths;
5. preserve reviewed evidence outside the 14-day CI artifact window;
6. complete visual DPI, keyboard-only, and screen-reader validation on desktop Windows.

Do not describe the matrix as a sandbox audit, malware verdict, Windows 10/11 certification, or proof for formats that are not named in a passing report.
