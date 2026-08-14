[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$testRoot = Join-Path (
    [IO.Path]::GetTempPath()
) ("iroha-zip-release-inventory-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $testRoot | Out-Null

try {
    $assetFiles = @()
    $remoteAssets = @()
    foreach ($index in 1..6) {
        $path = Join-Path $testRoot ("asset-$index.bin")
        [IO.File]::WriteAllBytes($path, [byte[]](0..$index))
        $item = Get-Item -LiteralPath $path
        $digest = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        $assetFiles += $path
        $remoteAssets += [ordered]@{
            name = $item.Name
            size = [int64]$item.Length
            digest = "sha256:$digest"
            state = "uploaded"
        }
    }

    $jsonPath = Join-Path $testRoot "release.json"
    $release = [ordered]@{
        isDraft = $true
        isImmutable = $false
        isPrerelease = $false
        assets = $remoteAssets
    }
    $release | ConvertTo-Json -Depth 5 | Out-File -LiteralPath $jsonPath -Encoding utf8
    & (Join-Path $PSScriptRoot "verify-release-inventory.ps1") `
        -ReleaseJson $jsonPath `
        -AssetFiles $assetFiles `
        -ExpectedState Draft

    $release.isDraft = $false
    $release.isImmutable = $true
    $release | ConvertTo-Json -Depth 5 | Out-File -LiteralPath $jsonPath -Encoding utf8
    & (Join-Path $PSScriptRoot "verify-release-inventory.ps1") `
        -ReleaseJson $jsonPath `
        -AssetFiles $assetFiles `
        -ExpectedState PublishedImmutable

    $rejectCases = @(
        @{
            Label = "mutable published state"
            Expected = "Expected a published, non-prerelease immutable release."
            Mutate = {
                param($candidate)
                $candidate.isImmutable = $false
            }
        },
        @{
            Label = "case-changed name"
            Expected = "Unexpected release asset: ASSET-1.BIN"
            Mutate = {
                param($candidate)
                $candidate.assets[0].name = "ASSET-1.BIN"
            }
        },
        @{
            Label = "length drift"
            Expected = "Release asset size mismatch: asset-1.bin"
            Mutate = {
                param($candidate)
                $candidate.assets[0].size = [int64]$candidate.assets[0].size + 1
            }
        },
        @{
            Label = "digest drift"
            Expected = "Release asset digest mismatch: asset-1.bin"
            Mutate = {
                param($candidate)
                $candidate.assets[0].digest = "sha256:" + ("0" * 64)
            }
        }
    )

    $validJson = $release | ConvertTo-Json -Depth 5
    foreach ($case in $rejectCases) {
        $candidate = $validJson | ConvertFrom-Json
        & $case.Mutate $candidate
        $candidate | ConvertTo-Json -Depth 5 | Out-File -LiteralPath $jsonPath -Encoding utf8
        $rejected = $false
        try {
            & (Join-Path $PSScriptRoot "verify-release-inventory.ps1") `
                -ReleaseJson $jsonPath `
                -AssetFiles $assetFiles `
                -ExpectedState PublishedImmutable
        }
        catch {
            if ($_.Exception.Message -cne $case.Expected) {
                throw "Unexpected rejection for $($case.Label): $($_.Exception.Message)"
            }
            $rejected = $true
        }
        if (-not $rejected) {
            throw "Release inventory verifier accepted $($case.Label)."
        }
    }

    Write-Host "Release inventory verifier self-test passed."
}
finally {
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
