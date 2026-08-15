[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$verifier = Join-Path $PSScriptRoot "verify-github-release-attestation.ps1"
$testRoot = Join-Path (
    [IO.Path]::GetTempPath()
) ("iroha-zip-release-attestation-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $testRoot | Out-Null

function Invoke-ExpectedFailure {
    param(
        [Parameter(Mandatory = $true)]
        [scriptblock]$Action,

        [Parameter(Mandatory = $true)]
        [string]$Expected
    )

    $rejected = $false
    try {
        & $Action
    }
    catch {
        if ($_.Exception.Message -cne $Expected) {
            throw "Unexpected rejection: $($_.Exception.Message)"
        }
        $rejected = $true
    }
    if (-not $rejected) {
        throw "Verifier accepted a case that must fail: $Expected"
    }
}

try {
    $assetFiles = @()
    foreach ($index in 1..11) {
        $path = Join-Path $testRoot ("asset-$index.bin")
        [IO.File]::WriteAllBytes($path, [byte[]](0..$index))
        $assetFiles += $path
    }

    $calls = [Collections.Generic.List[string]]::new()
    $mockState = [pscustomobject]@{ ReleaseAttempts = 0 }
    $eventualSuccess = {
        param([string[]]$InvocationArguments)

        $calls.Add(($InvocationArguments -join "|")) | Out-Null
        if ($InvocationArguments[0] -cne "release") {
            return [pscustomobject]@{ ExitCode = 64; Output = @("unexpected command") }
        }
        if ($InvocationArguments[1] -ceq "verify") {
            $mockState.ReleaseAttempts += 1
            if ($mockState.ReleaseAttempts -eq 1) {
                return [pscustomobject]@{ ExitCode = 1; Output = @("release proof unavailable") }
            }
            return [pscustomobject]@{ ExitCode = 0; Output = @("release proof verified") }
        }
        if ($InvocationArguments[1] -ceq "verify-asset") {
            return [pscustomobject]@{ ExitCode = 0; Output = @("asset proof verified") }
        }
        return [pscustomobject]@{ ExitCode = 64; Output = @("unexpected command") }
    }.GetNewClosure()

    & $verifier `
        -Tag v1.2.3 `
        -Repository hjosugi/iroha-zip `
        -AssetFiles $assetFiles `
        -MaxAttempts 3 `
        -RetryDelayMilliseconds 1 `
        -CommandInvoker $eventualSuccess

    if ($calls.Count -ne 13) {
        throw "Expected two release checks and 11 asset checks; found $($calls.Count) calls."
    }
    if ($calls[0] -cne "release|verify|v1.2.3|--repo|hjosugi/iroha-zip" -or
        $calls[1] -cne $calls[0]) {
        throw "Release verification arguments or retry order changed."
    }
    $expectedAssets = @(Get-Item -LiteralPath $assetFiles | Sort-Object Name)
    foreach ($index in 0..10) {
        $expectedCall = "release|verify-asset|v1.2.3|$($expectedAssets[$index].FullName)|--repo|hjosugi/iroha-zip"
        if ($calls[$index + 2] -cne $expectedCall) {
            throw "Asset verification arguments changed at index ${index}: $($calls[$index + 2])"
        }
    }

    $assetMismatch = {
        param([string[]]$InvocationArguments)

        if ($InvocationArguments[1] -ceq "verify") {
            return [pscustomobject]@{ ExitCode = 0; Output = @("release proof verified") }
        }
        if ([IO.Path]::GetFileName($InvocationArguments[3]) -ceq "asset-5.bin") {
            return [pscustomobject]@{ ExitCode = 1; Output = @("asset proof mismatch") }
        }
        return [pscustomobject]@{ ExitCode = 0; Output = @("asset proof verified") }
    }
    Invoke-ExpectedFailure `
        -Expected "GitHub release attestation verification failed after 1 attempt for v1.2.3: asset verification failed for asset-5.bin: asset proof mismatch" `
        -Action {
            & $verifier `
                -Tag v1.2.3 `
                -Repository hjosugi/iroha-zip `
                -AssetFiles $assetFiles `
                -MaxAttempts 1 `
                -RetryDelayMilliseconds 1 `
                -CommandInvoker $assetMismatch
        }

    $alwaysUnavailable = {
        param([string[]]$InvocationArguments)
        return [pscustomobject]@{ ExitCode = 1; Output = @("release proof unavailable") }
    }
    Invoke-ExpectedFailure `
        -Expected "GitHub release attestation verification failed after 3 attempts for v1.2.3: release proof unavailable" `
        -Action {
            & $verifier `
                -Tag v1.2.3 `
                -Repository hjosugi/iroha-zip `
                -AssetFiles $assetFiles `
                -MaxAttempts 3 `
                -RetryDelayMilliseconds 1 `
                -CommandInvoker $alwaysUnavailable
        }

    Invoke-ExpectedFailure `
        -Expected "Expected exactly 11 release assets; found 10." `
        -Action {
            & $verifier `
                -Tag v1.2.3 `
                -Repository hjosugi/iroha-zip `
                -AssetFiles @($assetFiles[0..9]) `
                -MaxAttempts 1 `
                -CommandInvoker $alwaysUnavailable
        }

    $duplicateRoot = Join-Path $testRoot "duplicate"
    New-Item -ItemType Directory -Path $duplicateRoot | Out-Null
    $duplicatePath = Join-Path $duplicateRoot "asset-1.bin"
    [IO.File]::WriteAllBytes($duplicatePath, [byte[]](1, 2, 3))
    $duplicateAssets = @($assetFiles)
    $duplicateAssets[10] = $duplicatePath
    Invoke-ExpectedFailure `
        -Expected "Duplicate local release asset name: asset-1.bin" `
        -Action {
            & $verifier `
                -Tag v1.2.3 `
                -Repository hjosugi/iroha-zip `
                -AssetFiles $duplicateAssets `
                -MaxAttempts 1 `
                -CommandInvoker $alwaysUnavailable
        }

    Write-Host "GitHub release attestation verifier self-test passed."
}
finally {
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
