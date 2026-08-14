[CmdletBinding()]
param(
    [string]$ValidatorExecutable
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$installer = Join-Path $PSScriptRoot "install-backend.ps1"
$validatorCandidates = @(
    (Join-Path $projectRoot "target\debug\iroha-zip.exe"),
    (Join-Path $projectRoot "target/debug/iroha-zip"),
    (Join-Path $projectRoot "target\release\iroha-zip.exe"),
    (Join-Path $projectRoot "target/release/iroha-zip")
)
$validator = if ([string]::IsNullOrWhiteSpace($ValidatorExecutable)) {
    $validatorCandidates |
        Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
        Select-Object -First 1
}
else {
    (Resolve-Path -LiteralPath $ValidatorExecutable -ErrorAction Stop).Path
}
if ($null -eq $validator) {
    throw "Build iroha-zip before running the backend install rollback test."
}

function Get-TreeSnapshot([string]$Root) {
    $rootItem = Get-Item -LiteralPath $Root -Force -ErrorAction Stop
    $separators = [char[]]@(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
    $records = [System.Collections.Generic.List[string]]::new()
    foreach ($item in @(Get-ChildItem -LiteralPath $rootItem.FullName -Recurse -Force |
            Sort-Object { $_.FullName.Substring($rootItem.FullName.Length) })) {
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Test tree unexpectedly contains a reparse point: $($item.FullName)"
        }
        $relative = $item.FullName.Substring($rootItem.FullName.Length).TrimStart($separators)
        if ($item.PSIsContainer) {
            $records.Add("directory`t$relative")
        }
        else {
            $hash = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            $records.Add("file`t$relative`t$($item.Length)`t$hash")
        }
    }
    return [string]::Join("`n", $records)
}

function Assert-NoTransactionResidue([string]$Parent) {
    $residue = @(
        Get-ChildItem -LiteralPath $Parent -Force |
            Where-Object {
                $_.Name -like ".iroha-zip-backend-stage-*" -or
                $_.Name -like ".iroha-zip-backend-backup-*"
            }
    )
    if ($residue.Count -ne 0) {
        throw "Backend transaction residue remains: $($residue.Name -join ', ')"
    }
}

$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) `
    ("iroha-zip-backend-rollback-" + [Guid]::NewGuid().ToString("N"))
$source = Join-Path $testRoot "source"
$destination = Join-Path $testRoot "installed"
$nested = Join-Path $destination "既存"
$previousCi = $env:CI
$previousFailure = $env:IROHA_ZIP_TEST_INSTALL_FAILURE

try {
    [System.IO.Directory]::CreateDirectory($source) | Out-Null
    [System.IO.Directory]::CreateDirectory($nested) | Out-Null
    [System.IO.File]::WriteAllBytes(
        (Join-Path $source "bsdtar.exe"),
        [byte[]](0x49, 0x52, 0x4f, 0x48, 0x41, 0x00, 0xff)
    )
    [System.IO.File]::WriteAllText(
        (Join-Path $source "support.dll"),
        "deterministic test payload",
        [System.Text.UTF8Encoding]::new($false)
    )
    [System.IO.File]::WriteAllText(
        (Join-Path $destination "preserve.marker"),
        "existing backend",
        [System.Text.UTF8Encoding]::new($false)
    )
    [System.IO.File]::WriteAllBytes(
        (Join-Path $nested "bytes.bin"),
        [byte[]](0x00, 0x01, 0x7f, 0x80, 0xfe, 0xff)
    )
    $originalSnapshot = Get-TreeSnapshot $destination

    $unapprovedRejected = $false
    try {
        & $installer -SourceDirectory $source -DestinationDirectory $destination
    }
    catch {
        if ($_.Exception.Message -notmatch "Re-run with -AllowUnsupportedSource") {
            throw
        }
        $unapprovedRejected = $true
    }
    if (-not $unapprovedRejected) {
        throw "An unapproved local bundle was unexpectedly installed."
    }
    if ((Get-TreeSnapshot $destination) -cne $originalSnapshot) {
        throw "Pre-commit validation failure changed the existing backend tree."
    }
    Assert-NoTransactionResidue $testRoot

    $env:CI = "true"
    $env:IROHA_ZIP_TEST_INSTALL_FAILURE = "after-backup"
    $postBackupRejected = $false
    try {
        & $installer -SourceDirectory $source -DestinationDirectory $destination `
            -AllowUnsupportedSource
    }
    catch {
        if ($_.Exception.Message -notmatch "Injected backend replacement failure") {
            throw
        }
        $postBackupRejected = $true
    }
    if (-not $postBackupRejected) {
        throw "The post-backup replacement failure was not injected."
    }
    if ((Get-TreeSnapshot $destination) -cne $originalSnapshot) {
        throw "Post-backup failure did not restore the byte-identical prior backend tree."
    }
    Assert-NoTransactionResidue $testRoot

    Remove-Item Env:IROHA_ZIP_TEST_INSTALL_FAILURE -ErrorAction SilentlyContinue
    & $installer -SourceDirectory $source -DestinationDirectory $destination `
        -AllowUnsupportedSource
    if (Test-Path -LiteralPath (Join-Path $destination "preserve.marker")) {
        throw "The successful replacement retained a file from the prior backend."
    }
    & $validator verify-backend-evidence $destination
    if ($LASTEXITCODE -ne 0) {
        throw "The successfully replaced backend failed independent evidence validation."
    }
    Assert-NoTransactionResidue $testRoot
    Write-Host "Backend install pre-commit rejection, post-backup rollback, cleanup, and recovery import passed."
}
finally {
    if ($null -eq $previousCi) {
        Remove-Item Env:CI -ErrorAction SilentlyContinue
    }
    else {
        $env:CI = $previousCi
    }
    if ($null -eq $previousFailure) {
        Remove-Item Env:IROHA_ZIP_TEST_INSTALL_FAILURE -ErrorAction SilentlyContinue
    }
    else {
        $env:IROHA_ZIP_TEST_INSTALL_FAILURE = $previousFailure
    }
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
