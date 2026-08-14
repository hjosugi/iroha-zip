[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$testRoot = Join-Path (
    [IO.Path]::GetTempPath()
) ("iroha-zip-pe-architecture-" + [Guid]::NewGuid().ToString("N"))
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
    $x64 = Join-Path $testRoot "x64.exe"
    $arm64 = Join-Path $testRoot "arm64.exe"
    $invalid = Join-Path $testRoot "invalid.exe"
    New-TestPe $x64 0x8664
    New-TestPe $arm64 0xaa64
    [IO.File]::WriteAllBytes($invalid, [byte[]](0..15))

    & (Join-Path $PSScriptRoot "verify-pe-architecture.ps1") `
        -Files @($x64) -Architecture x64
    & (Join-Path $PSScriptRoot "verify-pe-architecture.ps1") `
        -Files @($arm64) -Architecture arm64

    # Cargo's top-level release binary is normally a hardlink to the matching
    # artifact under release/deps. It is a regular file, not a reparse point.
    $cargoStyleHardlink = Join-Path $testRoot "x64-hardlink.exe"
    New-Item -ItemType HardLink -Path $cargoStyleHardlink -Target $x64 | Out-Null
    & (Join-Path $PSScriptRoot "verify-pe-architecture.ps1") `
        -Files @($x64, $cargoStyleHardlink) -Architecture x64

    Assert-Rejected {
        & (Join-Path $PSScriptRoot "verify-pe-architecture.ps1") `
            -Files @($arm64) -Architecture x64
    } "PE machine mismatch for arm64.exe: expected x64 (0x8664), found 0xAA64"
    Assert-Rejected {
        & (Join-Path $PSScriptRoot "verify-pe-architecture.ps1") `
            -Files @($invalid) -Architecture arm64
    } "PE input has no valid DOS header: invalid.exe"
    Assert-Rejected {
        & (Join-Path $PSScriptRoot "verify-pe-architecture.ps1") `
            -Files @($x64, $x64) -Architecture x64
    } "Duplicate PE input: $x64"

    Write-Host "PE architecture verifier self-test passed."
}
finally {
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
