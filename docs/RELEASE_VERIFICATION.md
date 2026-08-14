# Release and independent verification

Updated: 2026-08-14

The current official iroha-zip release is unsigned. The release boundary combines a pinned source tag, reproducible workflow inputs, SHA-256 inventories, and GitHub artifact attestations. None of these should be described as an Authenticode publisher signature. For end-user guidance in Japanese and English, see [About unsigned releases](UNSIGNED_RELEASE.md).

## Current unsigned artifact contract

A Windows x64 release contains exactly six assets, with the version substituted:

- `iroha-zip-<version>-windows-x64.zip`
- `iroha-zip-<version>-windows-x64.zip.sha256`
- `iroha-zip-<version>-windows-x64.exe`
- `iroha-zip-settings-<version>-windows-x64.exe`
- `iroha-zip-shell-<version>-windows-x64.exe`
- `SHA256SUMS.txt`

The ZIP contains all three executables, setup and association scripts, Japanese and English guides, security documents, configuration examples, and license notices. It does not contain libarchive, `bsdtar.exe`, or backend DLLs.

`SHA256SUMS.txt` covers the ZIP and the three standalone executables. The ZIP-specific `.sha256` sidecar remains available for simple automated checks. A digest detects byte changes relative to the compared inventory, but a digest downloaded from the same compromised location is not an independent identity anchor.

The release workflow generates a GitHub artifact attestation for the ZIP, all three standalone executables, and `SHA256SUMS.txt`. An attestation binds subject digests to the GitHub repository, workflow, ref, commit, and hosted runner identity. It does not give an unsigned executable an Authenticode publisher identity and does not suppress SmartScreen warnings.

## Release workflow

The tag-driven workflow is deliberately non-interactive:

Before creating a tag, maintainers can manually dispatch the Release workflow on `main` with `publish` left disabled. This dry run executes the complete unsigned validation, build, package, attestation, and artifact-upload path but cannot create or change a GitHub Release. Manual publication requires both an existing matching tag and an explicit `publish` selection.

1. The tag must have the exact form `vX.Y.Z`, match `Cargo.toml`, resolve to the checked-out commit, and point to the current `main` commit.
2. The pinned Rust 1.97.1 toolchain runs formatting, all targets, minimized fuzz regressions, Clippy with warnings denied, and a locked Windows x64 release build.
3. Packaging verifies the expected three executable names and creates a backend-free ZIP.
4. The workflow checks every standalone executable for the Windows PE `MZ` header before staging it as an asset.
5. SHA-256 inventories are generated from the final bytes.
6. A pinned GitHub action creates artifact attestations for the published binaries, ZIP, and checksum inventory.
7. The exact assets are also retained as one short-lived workflow artifact.
8. The repository immutable-release policy must be confirmed by an administrator before publication. The workflow creates an explicit draft and refuses to overwrite any existing release.
9. Before publication, the workflow reads the draft back and verifies all six assets by exact case-sensitive name, byte length, upload state, and SHA-256 digest.
10. Only that verified draft is published and marked latest. The workflow then requires a stable, immutable release and repeats the complete asset verification.

The repository immutable-release policy was enabled on 2026-08-14 and applies only to releases published after it was enabled. The unsigned [v0.4.1 release](https://github.com/hjosugi/iroha-zip/releases/tag/v0.4.1) was the first publication under that policy and passed immutable/latest and exact six-asset readback in [Actions run 31769440507](https://github.com/hjosugi/iroha-zip/actions/runs/31769440507). Its public assets and all five attested subjects were then independently downloaded and verified against the tag ref and commit. The older unsigned `v0.4.0` release predates enforcement and remains mutable according to GitHub's API. A failed future draft remains unpublished for investigation rather than being silently deleted or overwritten.

GitHub's policy-status endpoint requires repository `Administration: read`, which the standard Actions `GITHUB_TOKEN` cannot request. Do not add a long-lived administrator token to work around that boundary. Immediately before creating a release tag, an administrator must confirm **Settings → General → Releases → Enable release immutability**, or independently call `GET /repos/hjosugi/iroha-zip/immutable-releases`. The workflow requires immutable readback after publication. If that check ever fails, treat the mutable publication as an incident; do not repair it by replacing assets under the same version.

The workflow uses fixed action commit SHAs, a fixed Windows runner generation, the pinned Rust toolchain, and both locked Cargo workspaces. A failure before the publish transition exposes no stable release. A failure after publication is visible in Actions and requires a new version for corrections; immutable assets must never be replaced.

## Verify a download on Windows

Download the release assets only from `https://github.com/hjosugi/iroha-zip/releases`. First compare the selected file with `SHA256SUMS.txt`:

```powershell
$asset = Get-Item .\iroha-zip-0.4.1-windows-x64.zip
$expected = Get-Content .\SHA256SUMS.txt |
  Where-Object { $_ -match ([regex]::Escape($asset.Name) + '$') }
if (@($expected).Count -ne 1) { throw 'Missing or duplicate checksum entry' }
$actual = (Get-FileHash -LiteralPath $asset.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
$expectedHash = ($expected -split '\s+')[0].ToLowerInvariant()
if ($actual -cne $expectedHash) { throw 'SHA-256 mismatch' }
```

Then verify the GitHub artifact attestation:

```powershell
gh attestation verify .\iroha-zip-0.4.1-windows-x64.zip `
  --repo hjosugi/iroha-zip `
  --signer-workflow hjosugi/iroha-zip/.github/workflows/release.yml `
  --source-ref refs/tags/v0.4.1 `
  --deny-self-hosted-runners
```

Repeat this command for a standalone executable if you downloaded one directly. For a higher-assurance review, also pass an independently obtained `--source-digest` and compare the public tag with the reviewed source commit.

Do not expect `Get-AuthenticodeSignature` to return `Valid` for the current release. The binaries are intentionally unsigned. Do not disable SmartScreen or organization policy globally; make a per-file decision only after verifying provenance and reviewing the remaining risk.

## Future Authenticode path

The repository retains a strict local signing boundary for future use:

1. `scripts/build-release.ps1 -Phase Build` validates and produces exactly three unsigned executables.
2. A separately authorized signing system may sign those exact bytes.
3. `scripts/build-release.ps1 -Phase Package -RequireAuthenticode -ExpectedPublisher <subject>` validates each executable through Windows trust APIs before packaging.
4. `scripts/verify-release-signatures.ps1` requires a valid signature status, the exact publisher subject, the Code Signing EKU (`1.3.6.1.5.5.7.3.3`), and an RFC3161 timestamp, and records deterministic evidence.
5. The same checks run again after ZIP expansion, and embedded and detached evidence must match byte for byte.

This path is not active in the current GitHub release workflow because no repository signing identity has been configured or independently reviewed. No automation may accept a certificate authority's terms, identity declaration, or legal agreement on the owner's behalf.

## Incident response

- If a tag, workflow, maintainer account, or release may be compromised, stop releases and preserve the tag, commit, Actions logs, attestations, and downloaded digests.
- Publish affected digests through an independent project channel and do not replace assets under the same version.
- Revoke compromised credentials or GitHub access before preparing a new version.
- Artifact attestations and checksums establish provenance, not safety. They do not replace Windows E2E evidence, dependency review, malware defenses, or independent security review.
- If Authenticode is enabled in the future, treat the signing profile and its authorization environment as separate high-value trust boundaries.

Primary references: [GitHub immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases), [GitHub artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations), [GitHub CLI attestation verification](https://cli.github.com/manual/gh_attestation_verify), and [Microsoft Authenticode](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/authenticode).
