<#
.SYNOPSIS
Validates, builds, and packages iroha-zip Windows executables.

.DESCRIPTION
The default All phase creates an unsigned local preview. Formal releases use the Build
phase, sign the three resulting executables outside this script, and then use Package
with RequireAuthenticode and the exact expected publisher certificate subject.
#>
[CmdletBinding()]
param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$IncludeBackend,
    [switch]$AllowUnsupportedBackendSource,
    [ValidateSet("All", "Build", "Package")]
    [string]$Phase = "All",
    [switch]$RequireAuthenticode,
    [string]$ExpectedPublisher
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ExpectedBinaryNames = @(
    "iroha-zip.exe",
    "iroha-zip-settings.exe",
    "iroha-zip-shell.exe"
)
$FailedPackageCleanupPaths = @()

function Assert-BackendEvidence(
    [string]$Validator,
    [string]$BackendDirectory,
    [bool]$AllowUnsupported
) {
    $validationArguments = @("verify-backend-evidence", $BackendDirectory)
    if (-not $AllowUnsupported) {
        $validationArguments += "--require-supported"
    }
    & $Validator @validationArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Backend provenance, SPDX SBOM, or license evidence validation failed: $BackendDirectory"
    }
}

Push-Location $ProjectRoot
try {
    $shouldBuild = $Phase -in @("All", "Build")
    $shouldPackage = $Phase -in @("All", "Package")

    if ($AllowUnsupportedBackendSource -and -not $IncludeBackend) {
        throw "-AllowUnsupportedBackendSource is valid only together with -IncludeBackend."
    }
    if ($RequireAuthenticode -and $Phase -ne "Package") {
        throw "-RequireAuthenticode is valid only with -Phase Package after external signing."
    }
    if ($RequireAuthenticode -and [string]::IsNullOrWhiteSpace($ExpectedPublisher)) {
        throw "-RequireAuthenticode requires -ExpectedPublisher."
    }
    if ($shouldBuild) {
        if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
            throw "cargo was not found. Install the Rust toolchain specified by rust-toolchain.toml."
        }
        if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
            throw "rustup was not found."
        }
    }

    if ($IncludeBackend) {
        $backendManifest = Join-Path $ProjectRoot "backend\libarchive\backend-manifest.tsv"
        if (-not (Test-Path -LiteralPath $backendManifest -PathType Leaf)) {
            throw "-IncludeBackend requires an installed backend. Use the settings screen or an install script first."
        }
    }

    if ($shouldBuild) {
        & rustup target add $Target
        if ($LASTEXITCODE -ne 0) { throw "rustup target add failed." }

        if (-not (Test-Path -LiteralPath (Join-Path $ProjectRoot "Cargo.lock"))) {
            & cargo generate-lockfile
            if ($LASTEXITCODE -ne 0) { throw "cargo generate-lockfile failed." }
            Write-Warning "Cargo.lock was generated. Review and commit it before a formal release."
        }

        & cargo fmt --all -- --check
        if ($LASTEXITCODE -ne 0) { throw "cargo fmt check failed." }

        & cargo test --all-targets --locked
        if ($LASTEXITCODE -ne 0) { throw "cargo test failed." }

        & cargo test --locked --features fuzzing --test fuzz_regressions
        if ($LASTEXITCODE -ne 0) { throw "fuzz regression test failed." }

        & cargo clippy --all-targets --locked -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed." }

        & cargo build --release --target $Target --locked
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed." }
    }

    if ($IncludeBackend) {
        $backendDirectory = Join-Path $ProjectRoot "backend\libarchive"
        $validator = Join-Path $ProjectRoot "target\$Target\release\iroha-zip.exe"
        Assert-BackendEvidence $validator $backendDirectory $AllowUnsupportedBackendSource
        if ($AllowUnsupportedBackendSource) {
            Write-Warning "The private package may include a backend from an explicitly unsupported source. Review its embedded evidence."
        }
    }

    if (-not $shouldPackage) {
        Write-Host "Release executables built in target\$Target\release."
        return
    }
    if (-not $RequireAuthenticode) {
        Write-Warning "Creating an unsigned local preview. Formal releases require externally signed binaries and -RequireAuthenticode."
    }

    $version = ((Select-String -LiteralPath "Cargo.toml" -Pattern '^version\s*=\s*"([^\"]+)"').Matches[0].Groups[1].Value)
    $releaseSource = Join-Path $ProjectRoot "target\$Target\release"
    $distRoot = Join-Path $ProjectRoot "dist"
    $appRoot = Join-Path $distRoot "iroha-zip"
    $zip = Join-Path $distRoot "iroha-zip-$version-windows-x64.zip"
    $signatureEvidenceAsset = Join-Path $distRoot "iroha-zip-$version-windows-x64.signatures.json"
    $FailedPackageCleanupPaths = @($appRoot, $zip, "$zip.sha256", $signatureEvidenceAsset)
    if (Test-Path -LiteralPath $appRoot) {
        Remove-Item -LiteralPath $appRoot -Recurse -Force
    }
    foreach ($staleAsset in @($zip, "$zip.sha256", $signatureEvidenceAsset)) {
        if (Test-Path -LiteralPath $staleAsset) {
            Remove-Item -LiteralPath $staleAsset -Force
        }
    }
    New-Item -ItemType Directory -Path $appRoot | Out-Null

    foreach ($binary in @("iroha-zip.exe", "iroha-zip-shell.exe", "iroha-zip-settings.exe")) {
        $source = Join-Path $releaseSource $binary
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Built binary is missing: $source"
        }
        Copy-Item -LiteralPath $source -Destination (Join-Path $appRoot $binary)
    }

    if ($RequireAuthenticode) {
        $signatureVerifier = Join-Path $ProjectRoot "scripts\verify-release-signatures.ps1"
        $embeddedSignatureEvidence = Join-Path $appRoot "release-signatures.json"
        $packagedBinaries = @($ExpectedBinaryNames | ForEach-Object { Join-Path $appRoot $_ })
        & $signatureVerifier `
            -Files $packagedBinaries `
            -ExpectedPublisher $ExpectedPublisher `
            -EvidencePath $embeddedSignatureEvidence `
            -RequireTimestamp
        Copy-Item -LiteralPath $embeddedSignatureEvidence -Destination $signatureEvidenceAsset
    }

    $releaseBackend = Join-Path $appRoot "backend"
    New-Item -ItemType Directory -Force -Path $releaseBackend | Out-Null
    Copy-Item -LiteralPath (Join-Path $ProjectRoot "backend\README.md") `
        -Destination (Join-Path $releaseBackend "README.md")
    if ($IncludeBackend) {
        Copy-Item -LiteralPath (Join-Path $ProjectRoot "backend\libarchive") `
            -Destination (Join-Path $releaseBackend "libarchive") -Recurse
        Assert-BackendEvidence `
            $validator `
            (Join-Path $releaseBackend "libarchive") `
            $AllowUnsupportedBackendSource
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $appRoot "scripts") | Out-Null
    foreach ($script in @(
        "install-backend.ps1",
        "backend-evidence.ps1",
        "export-msys2-backend.ps1",
        "register-associations.ps1",
        "test-settings-ui.ps1",
        "test-windows-e2e.ps1",
        "unregister-associations.ps1",
        "verify-release-signatures.ps1"
    )) {
        Copy-Item -LiteralPath (Join-Path $ProjectRoot "scripts\$script") `
            -Destination (Join-Path $appRoot "scripts\$script")
    }

    foreach ($file in @(
        "README.md",
        "SECURITY.md",
        "config.example.toml",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "THIRD-PARTY-NOTICES.md",
        "THIRD-PARTY-LICENSES.html"
    )) {
        Copy-Item -LiteralPath (Join-Path $ProjectRoot $file) -Destination (Join-Path $appRoot $file)
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $appRoot "docs") | Out-Null
    foreach ($document in @(
        "ANTIMALWARE_HANDOFF.md",
        "ARCHIVE_PREVIEW.md",
        "BACKEND_EVIDENCE.md",
        "BACKEND_MANIFEST.md",
        "BUILD_STATUS.md",
        "ENCRYPTED_ARCHIVES.md",
        "FUZZING.md",
        "ISSUE_BACKLOG.md",
        "LPAC_EVALUATION.md",
        "MALICIOUS_CORPUS.md",
        "RELEASE_VERIFICATION.md",
        "SETTINGS_ACCESSIBILITY.md",
        "THREAT_MODEL.md",
        "WINDOWS_E2E.md"
    )) {
        Copy-Item -LiteralPath (Join-Path $ProjectRoot "docs\$document") `
            -Destination (Join-Path $appRoot "docs\$document")
    }

    Compress-Archive -LiteralPath $appRoot -DestinationPath $zip -CompressionLevel Optimal
    if ($IncludeBackend -or $RequireAuthenticode) {
        $expandedPackage = Join-Path $distRoot (".iroha-zip-package-check-" + [Guid]::NewGuid().ToString("N"))
        try {
            Expand-Archive -LiteralPath $zip -DestinationPath $expandedPackage
            if ($IncludeBackend) {
                Assert-BackendEvidence `
                    $validator `
                    (Join-Path $expandedPackage "iroha-zip\backend\libarchive") `
                    $AllowUnsupportedBackendSource
            }
            if ($RequireAuthenticode) {
                $expandedRoot = Join-Path $expandedPackage "iroha-zip"
                $expandedBinaries = @($ExpectedBinaryNames | ForEach-Object {
                    Join-Path $expandedRoot $_
                })
                $expandedEvidence = Join-Path $expandedRoot "release-signatures.json"
                & $signatureVerifier `
                    -Files $expandedBinaries `
                    -ExpectedPublisher $ExpectedPublisher `
                    -ExpectedEvidencePath $expandedEvidence `
                    -RequireTimestamp

                $embeddedHash = (Get-FileHash -LiteralPath $expandedEvidence -Algorithm SHA256).Hash
                $assetHash = (Get-FileHash -LiteralPath $signatureEvidenceAsset -Algorithm SHA256).Hash
                if ($embeddedHash -cne $assetHash) {
                    throw "Embedded and detached signature evidence differ."
                }
            }
        }
        finally {
            if (Test-Path -LiteralPath $expandedPackage) {
                Remove-Item -LiteralPath $expandedPackage -Recurse -Force
            }
        }
    }
    $hash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    [System.IO.File]::WriteAllText(
        "$zip.sha256",
        "$hash  $([System.IO.Path]::GetFileName($zip))`n",
        [System.Text.UTF8Encoding]::new($false)
    )

    Write-Host "Release folder: $appRoot"
    Write-Host "Release ZIP:    $zip"
    Write-Host "SHA-256:        $hash"
    if ($RequireAuthenticode) {
        Write-Host "Signatures:     $signatureEvidenceAsset"
    }
    if (-not $IncludeBackend) {
        Write-Host "Backend:        not bundled; install it from the settings screen"
    }
}
catch {
    foreach ($failedOutput in $FailedPackageCleanupPaths) {
        if (Test-Path -LiteralPath $failedOutput) {
            Remove-Item -LiteralPath $failedOutput -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
    throw
}
finally {
    Pop-Location
}
