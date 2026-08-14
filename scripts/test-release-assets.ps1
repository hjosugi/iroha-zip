[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$testRoot = Join-Path (
    [IO.Path]::GetTempPath()
) ("iroha-zip-release-assets-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $testRoot | Out-Null

function New-TestPe([string]$Path, [uint16]$Machine) {
    $bytes = [byte[]]::new(128)
    $bytes[0] = 0x4d
    $bytes[1] = 0x5a
    [BitConverter]::GetBytes([int32]0x40).CopyTo($bytes, 0x3c)
    $bytes[0x40] = 0x50
    $bytes[0x41] = 0x45
    [BitConverter]::GetBytes($Machine).CopyTo($bytes, 0x44)
    [IO.File]::WriteAllBytes($Path, $bytes)
}

function New-ArchitectureAssets([string]$Root) {
    New-Item -ItemType Directory -Path $Root | Out-Null
    foreach ($contract in @(
        @{ Architecture = "x64"; Machine = [uint16]0x8664 },
        @{ Architecture = "arm64"; Machine = [uint16]0xaa64 }
    )) {
        $architecture = $contract.Architecture
        $packageParent = Join-Path $testRoot ("package-" + $architecture + "-" + [Guid]::NewGuid().ToString("N"))
        $packageRoot = Join-Path $packageParent "iroha-zip"
        New-Item -ItemType Directory -Path $packageRoot | Out-Null
        $backendRoot = Join-Path $packageRoot "backend"
        New-Item -ItemType Directory -Path $backendRoot | Out-Null
        [IO.File]::WriteAllText((Join-Path $backendRoot "README.md"), "backend not bundled`n")
        foreach ($baseName in @("iroha-zip", "iroha-zip-settings", "iroha-zip-shell")) {
            $packageExe = Join-Path $packageRoot "$baseName.exe"
            New-TestPe $packageExe $contract.Machine
            Copy-Item -LiteralPath $packageExe -Destination (
                Join-Path $Root "$baseName-0.5.0-windows-$architecture.exe"
            )
        }
        $zipName = "iroha-zip-0.5.0-windows-$architecture.zip"
        $zip = Join-Path $Root $zipName
        Compress-Archive -LiteralPath $packageRoot -DestinationPath $zip
        $hash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
        [IO.File]::WriteAllText(
            "$zip.sha256",
            "$hash  $zipName`n",
            [Text.UTF8Encoding]::new($false)
        )
        Remove-Item -LiteralPath $packageParent -Recurse -Force
    }
}

function Assert-Rejected([scriptblock]$Action, [string]$ExpectedMessage) {
    $rejected = $false
    try {
        & $Action
    }
    catch {
        if ($_.Exception.Message -cne $ExpectedMessage) {
            throw "Unexpected rejection: $($_.Exception.Message)"
        }
        $rejected = $true
    }
    if (-not $rejected) {
        throw "Expected rejection was not raised: $ExpectedMessage"
    }
}

try {
    $valid = Join-Path $testRoot "valid"
    New-ArchitectureAssets $valid
    & (Join-Path $PSScriptRoot "assemble-release-assets.ps1") `
        -AssetRoot $valid `
        -Version 0.5.0
    $finalFiles = @(Get-ChildItem -LiteralPath $valid -File -Force)
    if ($finalFiles.Count -ne 11) {
        throw "Valid assembly did not contain exactly 11 files."
    }
    $checksumLines = @(Get-Content -LiteralPath (Join-Path $valid "SHA256SUMS.txt"))
    if ($checksumLines.Count -ne 8) {
        throw "Combined checksum inventory did not contain exactly eight lines."
    }

    $wrongMachine = Join-Path $testRoot "wrong-machine"
    New-ArchitectureAssets $wrongMachine
    New-TestPe (Join-Path $wrongMachine "iroha-zip-0.5.0-windows-arm64.exe") 0x8664
    Assert-Rejected {
        & (Join-Path $PSScriptRoot "assemble-release-assets.ps1") `
            -AssetRoot $wrongMachine `
            -Version 0.5.0
    } "PE machine mismatch for iroha-zip-0.5.0-windows-arm64.exe: expected arm64 (0xAA64), found 0x8664"

    $badSidecar = Join-Path $testRoot "bad-sidecar"
    New-ArchitectureAssets $badSidecar
    [IO.File]::WriteAllText(
        (Join-Path $badSidecar "iroha-zip-0.5.0-windows-x64.zip.sha256"),
        (("0" * 64) + "  iroha-zip-0.5.0-windows-x64.zip`n")
    )
    Assert-Rejected {
        & (Join-Path $PSScriptRoot "assemble-release-assets.ps1") `
            -AssetRoot $badSidecar `
            -Version 0.5.0
    } "ZIP checksum sidecar does not exactly match iroha-zip-0.5.0-windows-x64.zip."

    Write-Host "Dual-architecture release asset assembly self-test passed."
}
finally {
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
