# Release and independent verification

Updated: 2026-08-14

The current official iroha-zip binaries are unsigned. The release boundary combines a pinned source
tag, native architecture-specific hosted runners, exact PE checks, SHA-256 inventories, GitHub artifact
attestations, and immutable asset readback. None of these is an Authenticode publisher signature. See
[About unsigned releases](UNSIGNED_RELEASE.md) for end-user guidance in Japanese and English.

## Current unsigned artifact contract

Version `0.5.0` contains exactly 11 assets. Substitute the version for later releases:

- `iroha-zip-<version>-windows-x64.zip`
- `iroha-zip-<version>-windows-x64.zip.sha256`
- `iroha-zip-<version>-windows-x64.exe`
- `iroha-zip-settings-<version>-windows-x64.exe`
- `iroha-zip-shell-<version>-windows-x64.exe`
- `iroha-zip-<version>-windows-arm64.zip`
- `iroha-zip-<version>-windows-arm64.zip.sha256`
- `iroha-zip-<version>-windows-arm64.exe`
- `iroha-zip-settings-<version>-windows-arm64.exe`
- `iroha-zip-shell-<version>-windows-arm64.exe`
- `SHA256SUMS.txt`

Each ZIP contains the three matching-architecture executables, setup and association scripts,
Japanese and English guides, security documents, configuration examples, and license notices. It
does not contain libarchive, `bsdtar.exe`, or backend DLLs.

`SHA256SUMS.txt` covers both ZIPs and all six standalone executables. Each ZIP-specific `.sha256`
sidecar contains exactly one LF-terminated lowercase digest line for simple automation. A checksum
detects byte changes relative to the compared inventory, but a checksum downloaded from the same
compromised location is not an independent identity anchor.

The release workflow attests both ZIPs, all six standalone executables, and `SHA256SUMS.txt`: nine
attested subjects. An attestation binds a digest to the GitHub repository, workflow, ref, commit, and
hosted-runner identity. It neither grants an unsigned executable an Authenticode publisher identity
nor suppresses SmartScreen.

## Release workflow

Before tagging, maintainers manually dispatch Release on `main` with `publish` disabled. This runs the
complete build, packaging, architecture validation, attestation, aggregation, and artifact upload path
but cannot create or change a GitHub Release. Manual publication additionally requires an existing
matching tag and an explicit `publish` choice.

1. The exact `vX.Y.Z` value must match `Cargo.toml`; a tag must resolve to the checked-out commit; and
   every dry run or tag must point to the current remote `main` commit.
2. Pinned Rust 1.97.1 runs formatting, all-target tests, minimized fuzz regressions, Clippy with denied
   warnings, and a locked release build independently on native `windows-2025` x64 and
   `windows-11-arm` ARM64 runners.
3. Runner OS architecture, process architecture, and Rust host must match the requested target.
4. Packaging creates a backend-free ZIP. Build outputs and expanded ZIP executables must be x64
   machine `0x8664` or ARM64 machine `0xAA64` as named.
5. Each build stages exactly five case-sensitive architecture assets, validates the one-line sidecar,
   attests its ZIP and three standalone EXEs, and uploads only those five files.
6. An independent x64 aggregation job accepts exactly ten architecture inputs. It rechecks every
   standalone and expanded-ZIP PE, rejects x64/ARM64 mix-ups, validates both sidecars, and creates the
   combined eight-subject `SHA256SUMS.txt`.
7. A pinned GitHub action attests that combined inventory. The exact 11 files are retained as a
   downloadable workflow artifact even when publication is disabled.
8. Publication refuses an existing release, creates an explicit draft, and never overwrites assets.
9. Draft readback must match all 11 local files by exact name, upload state, byte length, and SHA-256.
10. Only that verified draft is published as latest. Published readback must be stable, non-prerelease,
    immutable, latest, and an exact match for all 11 assets.

The repository immutable-release policy was enabled on 2026-08-14. Unsigned
[`v0.4.1`](https://github.com/hjosugi/iroha-zip/releases/tag/v0.4.1) was the first publication under
that policy and passed exact six-asset immutable readback in
[Actions run 31769440507](https://github.com/hjosugi/iroha-zip/actions/runs/31769440507). The older
`v0.4.0` predates enforcement and remains mutable according to GitHub's API. A failed future draft
remains unpublished for investigation; it is not silently deleted or overwritten.

GitHub's immutable-policy status endpoint requires repository `Administration: read`, which the
standard Actions token cannot request. Do not add a long-lived administrator token. Immediately
before a tag, an administrator must independently confirm **Settings → General → Releases → Enable
release immutability** or call `GET /repos/hjosugi/iroha-zip/immutable-releases`. If immutable readback
ever fails, treat it as an incident and publish a new version rather than replacing assets.

The workflow uses action commit SHAs, fixed runner labels, a pinned Rust toolchain, and locked Cargo
workspaces. A pre-publication failure exposes no stable release. A post-publication failure remains
visible and requires a new version.

## Verify a download on Windows

Download only from `https://github.com/hjosugi/iroha-zip/releases`. Select `x64` for Intel/AMD or
`arm64` for Windows on ARM, then compare it with `SHA256SUMS.txt`:

```powershell
$asset = Get-Item .\iroha-zip-0.5.0-windows-arm64.zip
$expected = Get-Content .\SHA256SUMS.txt |
  Where-Object { $_ -match ([regex]::Escape($asset.Name) + '$') }
if (@($expected).Count -ne 1) { throw 'Missing or duplicate checksum entry' }
$actual = (Get-FileHash -LiteralPath $asset.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
$expectedHash = ($expected -split '\s+')[0].ToLowerInvariant()
if ($actual -cne $expectedHash) { throw 'SHA-256 mismatch' }
```

Then verify the GitHub artifact attestation:

```powershell
gh attestation verify .\iroha-zip-0.5.0-windows-arm64.zip `
  --repo hjosugi/iroha-zip `
  --signer-workflow hjosugi/iroha-zip/.github/workflows/release.yml `
  --source-ref refs/tags/v0.5.0 `
  --deny-self-hosted-runners
```

Repeat for a directly downloaded executable. For a higher-assurance review, also pass an
independently obtained `--source-digest` and compare the tag with the reviewed commit. Do not expect
`Get-AuthenticodeSignature` to return `Valid`: these binaries are intentionally unsigned. Do not
disable SmartScreen or organization policy globally.

## Future Authenticode path

The repository retains a strict local signing boundary:

1. `scripts/build-release.ps1 -Target <target> -Phase Build` validates and produces three unsigned EXEs.
2. A separately authorized signing system may sign those exact bytes.
3. `scripts/build-release.ps1 -Target <target> -Phase Package -RequireAuthenticode
   -ExpectedPublisher <subject>` validates every executable through Windows trust APIs.
4. `scripts/verify-release-signatures.ps1` requires valid status, the exact publisher subject, Code
   Signing EKU `1.3.6.1.5.5.7.3.3`, and an RFC3161 timestamp, and records deterministic evidence.
5. Checks repeat after ZIP expansion; embedded and detached evidence must match byte for byte.

No signing identity has been configured or independently reviewed. Automation must never accept a
certificate authority's terms, identity declaration, or legal agreement for the owner.

## Incident response

- Preserve suspected tags, commits, logs, attestations, and downloaded digests; stop releases.
- Publish affected digests through an independent channel and never replace same-version assets.
- Revoke compromised credentials before preparing a new version.
- Checksums and attestations establish provenance, not safety. They do not replace Windows E2E,
  dependency review, malware defenses, or independent security review.

Primary references: [GitHub immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases),
[GitHub artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations),
[GitHub CLI attestation verification](https://cli.github.com/manual/gh_attestation_verify), and
[Microsoft Authenticode](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/authenticode).
