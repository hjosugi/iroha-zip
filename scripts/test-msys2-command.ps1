[CmdletBinding()]
param(
    [string]$BashPath = "/usr/bin/bash"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "msys2-command.ps1")

if (-not (Test-Path -LiteralPath $BashPath -PathType Leaf)) {
    throw "Test bash executable was not found: $BashPath"
}

$existingTemporaryRoots = @(
    Get-ChildItem -LiteralPath ([System.IO.Path]::GetTempPath()) -Directory -Force |
        Where-Object { $_.Name -like "iroha-zip-msys2-command-*" } |
        ForEach-Object { $_.Name }
)

$exporterSource = Get-Content -LiteralPath (Join-Path $PSScriptRoot "export-msys2-backend.ps1") -Raw
if ($exporterSource -notmatch '\. \(Join-Path \$PSScriptRoot "msys2-command\.ps1"\)' -or
    $exporterSource -notmatch 'Invoke-IrohaZipMsys2Command') {
    throw "The backend exporter is not wired to the tested bounded launcher."
}
if ($exporterSource -match '(?m)^\s*&\s+\$bsdtar\b') {
    throw "The backend exporter invokes bsdtar outside the tested bounded launcher."
}
foreach ($marker in @(
    'function Invoke-Ldd([string[]]$UnixPaths)',
    'ldd "$@"',
    '$maximumRuntimeFiles = 256',
    '$lddBatchSize = 64',
    '$queued.Add($dependency)',
    '(?:api|ext)-ms-win-[A-Za-z0-9._-]+\.dll',
    "ldd reported an unresolved runtime dependency",
    'Invoke-Ldd $batch.ToArray()'
)) {
    if (-not $exporterSource.Contains($marker)) {
        throw "The backend exporter is missing the bounded batched ldd contract: $marker"
    }
}
if ($exporterSource -match 'Invoke-Ldd\s+\$current\b') {
    throw "The backend exporter invokes one ldd process per runtime file."
}
$packagerSource = Get-Content -LiteralPath (Join-Path $PSScriptRoot "build-release.ps1") -Raw
if ($packagerSource -notmatch '"msys2-command\.ps1"') {
    throw "Release packages do not include the backend exporter's bounded launcher."
}

$arguments = @("alpha beta", 'literal*?[$]', "日本語")
$lines = @(
    Invoke-IrohaZipMsys2Command `
        -BashPath $BashPath `
        -Script 'printf "%s\n" "$#" "$1" "$2" "$3"' `
        -Arguments $arguments `
        -TimeoutSeconds 5
)
$expected = @("3") + $arguments
if ($lines.Count -ne $expected.Count) {
    throw "Bounded launcher changed the argument count: expected=$($expected.Count) actual=$($lines.Count)"
}
for ($index = 0; $index -lt $expected.Count; $index++) {
    if ([string]$lines[$index] -cne [string]$expected[$index]) {
        throw "Bounded launcher changed argument $index."
    }
}

$nonzeroRejected = $false
try {
    Invoke-IrohaZipMsys2Command `
        -BashPath $BashPath `
        -Script 'printf "bounded failure evidence\n" >&2; exit 23' `
        -TimeoutSeconds 5 | Out-Null
}
catch {
    if ($_.Exception.Message -notmatch 'failed \(exit 23\)' -or
        $_.Exception.Message -notmatch 'bounded failure evidence') {
        throw
    }
    $nonzeroRejected = $true
}
if (-not $nonzeroRejected) {
    throw "A nonzero child unexpectedly passed the bounded launcher."
}

$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
$timeoutRejected = $false
try {
    Invoke-IrohaZipMsys2Command `
        -BashPath $BashPath `
        -Script 'sleep 30' `
        -TimeoutSeconds 1 | Out-Null
}
catch {
    if ($_.Exception.Message -notmatch 'exceeded the 1-second limit \(exit 124\)') {
        throw
    }
    $timeoutRejected = $true
}
$stopwatch.Stop()
if (-not $timeoutRejected) {
    throw "A timed-out child unexpectedly passed the bounded launcher."
}
if ($stopwatch.Elapsed.TotalSeconds -gt 8) {
    throw "The bounded child was not terminated promptly: $($stopwatch.Elapsed.TotalSeconds)s"
}

$temporaryResidue = @(
    Get-ChildItem -LiteralPath ([System.IO.Path]::GetTempPath()) -Directory -Force |
        Where-Object {
            $_.Name -like "iroha-zip-msys2-command-*" -and
            $existingTemporaryRoots -notcontains $_.Name
        }
)
if ($temporaryResidue.Count -ne 0) {
    throw "Bounded launcher temporary residue remains: $($temporaryResidue.Name -join ', ')"
}

Write-Host "MSYS2 command timeout and argument-preservation tests passed."
$global:LASTEXITCODE = 0
