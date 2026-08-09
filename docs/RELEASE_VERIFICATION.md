# Signed release and independent verification

Updated: 2026-08-10

SAFE-003 defines the trust boundary for formal iroha-zip releases. The release workflow and fail-closed verification scripts are implemented in the source tree, but no signed release produced by this path has yet been reviewed. An unsigned local ZIP is a development preview, not a formal release.

## Release artifact contract

A formal Windows x64 release contains exactly these four assets, with the version substituted:

- `iroha-zip-<version>-windows-x64.zip`
- `iroha-zip-<version>-windows-x64.zip.sha256`
- `iroha-zip-<version>-windows-x64.signatures.json`
- `iroha-zip-<version>-windows-x64.intoto.jsonl`

The ZIP contains `iroha-zip.exe`, `iroha-zip-settings.exe`, and `iroha-zip-shell.exe`. All three must have a valid embedded Authenticode signature from the same signing certificate, the exact configured publisher, the Code Signing EKU (`1.3.6.1.5.5.7.3.3`), and an RFC3161 timestamp. The detached signature-evidence JSON is byte-identical to `release-signatures.json` inside the ZIP.

The checksum detects byte changes but is not an authenticity mechanism by itself. Authenticode establishes a certificate-backed publisher identity. The Sigstore bundle binds the ZIP, checksum sidecar, and signature evidence digests to the release workflow, repository, source ref, and source commit. These mechanisms have different trust roots and are all required.

## One-time owner configuration

These steps change cloud resources, repository policy, or legal identity state and must be completed by the repository owner. Automation must not accept terms or identity declarations on the owner's behalf.

The read-only repository audit on 2026-08-10 found no `release-signing` environment, no Actions variables, `default_workflow_permissions: read`, and `immutable-releases.enabled: false`. Secret values were not read. This is a safe not-configured state: the workflow cannot currently pass its preflight and no formal release should be attempted until the owner completes and independently reviews the following settings.

### Azure Artifact Signing

1. Complete Microsoft's required identity validation and create an Artifact Signing account and public-trust certificate profile.
2. Create a dedicated Microsoft Entra application or managed identity for GitHub release signing.
3. Grant only the **Artifact Signing Certificate Profile Signer** role, scoped to the release certificate profile.
4. Add a federated credential with issuer `https://token.actions.githubusercontent.com`, audience `api://AzureADTokenExchange`, and subject `repo:hjosugi/iroha-zip:environment:release-signing`.
5. Do not export a PFX, store a signing private key in GitHub, or create an Azure client secret. The workflow uses short-lived GitHub OIDC credentials.

Artifact Signing uses short-lived signing certificates. The release timestamp is therefore mandatory so Windows can validate a signature made while the certificate was valid. Review Azure role assignments and signing audit logs regularly; remove stale identities immediately.

### GitHub repository settings

Use the `hjosugi/iroha-zip` repository settings UI:

1. Open **Settings → General → Releases** and enable **Release immutability**. The workflow checks the immutable-releases API before building and stops if it is disabled.
2. Open **Settings → Environments**, create `release-signing`, allow only protected `v*` tags, add a required reviewer, and prevent self-review where the account plan supports it.
3. In the `release-signing` environment, add these secrets:
   - `AZURE_CLIENT_ID`
   - `AZURE_TENANT_ID`
   - `AZURE_SUBSCRIPTION_ID`
4. Add these environment variables:
   - `AZURE_ARTIFACT_SIGNING_ENDPOINT`
   - `AZURE_ARTIFACT_SIGNING_ACCOUNT`
   - `AZURE_ARTIFACT_SIGNING_PROFILE`
   - `AZURE_ARTIFACT_SIGNING_PUBLISHER` — the exact `SignerCertificate.Subject` string expected from `Get-AuthenticodeSignature`
5. Under **Settings → Actions → General**, ensure repository policy permits the workflow's explicitly declared `contents: write`, `attestations: write`, and `id-token: write` permissions. Do not grant broader permissions to other workflows.

Record the expected publisher subject and approved release commit through a separately authenticated project channel. A value learned only from the same potentially compromised download page is not an independent identity anchor.

## Formal release flow

The release workflow is deliberately non-interactive and fail closed:

1. A `v<version>` tag must match `Cargo.toml`, resolve to the checked-out commit, and point to the current `main` commit.
2. All signing settings must be present and repository release immutability must already be enabled.
3. The pinned Rust toolchain validates, tests, lints, and builds the three executables.
4. GitHub OIDC authenticates to Azure; the pinned Artifact Signing action signs exactly the three named executables with SHA-256 and the Microsoft RFC3161 timestamp service.
5. Windows `WinVerifyTrust` validates publisher, status, Code Signing EKU, and timestamp before packaging and again after ZIP re-expansion.
6. The ZIP and checksum are generated, and the pinned GitHub attestation action produces SLSA build provenance for the ZIP, checksum sidecar, and signature evidence.
7. `gh attestation verify --bundle` verifies all three local subjects with exact repository, workflow, tag ref, and source digest policy before upload.
8. A draft release receives all four assets. Its exact non-empty inventory is checked before publication.
9. Publishing locks the tag and assets. The workflow reads the release back and requires `isImmutable: true`.

The workflow uses fixed action commit SHAs, a fixed Windows runner generation, a pinned Rust toolchain, and `Cargo.lock`. A failure before publication can leave a mutable draft for owner review; it cannot produce a published non-immutable release through this workflow.

## Verify a downloaded release on Windows

Obtain the expected publisher subject and approved source commit from the independent channel described above. Keep the four assets in one directory.

First verify the exact ZIP digest. This checks the download against the sidecar, but does not authenticate a sidecar obtained from the same location:

```powershell
$zip = Get-Item .\iroha-zip-0.3.1-windows-x64.zip
$line = (Get-Content -LiteralPath "$($zip.FullName).sha256" -Raw).Trim()
$expected = "{0}  {1}" -f `
  (Get-FileHash -LiteralPath $zip.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), `
  $zip.Name
if ($line -cne $expected) { throw "ZIP checksum mismatch" }
```

Expand into a new directory and ask Windows to validate each embedded signature. Do not click through a SmartScreen or certificate error:

```powershell
$expectedPublisher = 'CN=<publisher subject obtained independently>'
Expand-Archive -LiteralPath $zip.FullName -DestinationPath .\verified-release
$files = @(
  '.\verified-release\iroha-zip\iroha-zip.exe',
  '.\verified-release\iroha-zip\iroha-zip-settings.exe',
  '.\verified-release\iroha-zip\iroha-zip-shell.exe'
)
foreach ($file in $files) {
  $signature = Get-AuthenticodeSignature -LiteralPath $file
  if ($signature.Status -ne 'Valid') { throw "Invalid signature: $file" }
  if ($signature.SignerCertificate.Subject -cne $expectedPublisher) {
    throw "Unexpected publisher: $file"
  }
  if ($null -eq $signature.TimeStamperCertificate) {
    throw "Missing timestamp: $file"
  }
}
```

For the stricter EKU, exact inventory, and JSON-evidence checks used by CI, use a separately reviewed copy of `scripts/verify-release-signatures.ps1`:

```powershell
.\verify-release-signatures.ps1 `
  -Files $files `
  -ExpectedPublisher $expectedPublisher `
  -ExpectedEvidencePath .\verified-release\iroha-zip\release-signatures.json `
  -RequireTimestamp
```

The copy of this verifier inside the ZIP is convenient after the ZIP itself has been authenticated; it is not a trust anchor for an otherwise untrusted ZIP.

Verify the downloaded Sigstore bundle without fetching an attestation from GitHub. Replace the tag and commit with independently approved values:

```powershell
gh attestation verify .\iroha-zip-0.3.1-windows-x64.zip `
  --bundle .\iroha-zip-0.3.1-windows-x64.intoto.jsonl `
  --repo hjosugi/iroha-zip `
  --signer-workflow hjosugi/iroha-zip/.github/workflows/release.yml `
  --source-ref refs/tags/v0.3.1 `
  --source-digest <approved-40-character-commit> `
  --deny-self-hosted-runners
```

Repeat the same command for the `.zip.sha256` and `.signatures.json` files using the same bundle. All three subjects must verify.

`--bundle` makes the attestation lookup offline; certificate-chain and transparency-log verification still rely on the trusted Sigstore root bundled or selected by the verifier. Preserve the four assets, expected publisher, approved source commit, verifier version, and verification output for release evidence.

## Custody and incident response

- GitHub stores only non-secret identifiers plus OIDC configuration; no signing private key or reusable Azure password belongs in the repository.
- The Azure signing profile is the certificate custody boundary. Limit its signer role to the release identity and review every signing event.
- The GitHub `release-signing` environment is the workflow-authorization boundary. Required reviewers must compare the tag with `main` before approval.
- If the Azure identity, certificate profile, GitHub environment, workflow, or maintainer account may be compromised, stop releases, revoke the federated credential/role, preserve audit logs, publish the affected digests through an independent channel, and rotate the affected trust anchor.
- Release immutability prevents later asset/tag replacement; it does not make a malicious build safe and does not replace source review, Windows E2E evidence, dependency review, or reproducible-build work.

Primary references: [GitHub artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations), [GitHub immutable releases](https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/establish-provenance-and-integrity/prevent-release-changes), [Azure Artifact Signing integrations](https://learn.microsoft.com/en-us/azure/artifact-signing/how-to-signing-integrations), and [Azure Login with OpenID Connect](https://learn.microsoft.com/en-us/azure/developer/github/connect-from-azure-openid-connect).
