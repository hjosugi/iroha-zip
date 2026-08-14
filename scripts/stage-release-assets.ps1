[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9]+\.[0-9]+\.[0-9]+$')]
    [string]$Version,

    [Parameter(Mandatory = $true)]
    [ValidateSet("x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc")]
    [string]$Target,

    [string]$OutputFile
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$projectRoot = Split-Path -Parent $PSScriptRoot
$architecture = switch ($Target) {
    "x86_64-pc-windows-msvc" { "x64" }
    "aarch64-pc-windows-msvc" { "arm64" }
}
$manifestVersion = (
    (Select-String -LiteralPath (Join-Path $projectRoot "Cargo.toml") `
        -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
)
if ($Version -cne $manifestVersion) {
    throw "Requested version $Version does not match Cargo.toml version $manifestVersion."
}

$distRoot = Join-Path $projectRoot "dist"
$assetRoot = Join-Path $distRoot "publish-$architecture"
if (Test-Path -LiteralPath $assetRoot) {
    throw "Release staging directory already exists: $assetRoot"
}
New-Item -ItemType Directory -Path $assetRoot | Out-Null

try {
    $releaseRoot = Join-Path $projectRoot "target\$Target\release"
    $assetMap = [ordered]@{
        "iroha-zip.exe" = "iroha-zip-$Version-windows-$architecture.exe"
        "iroha-zip-settings.exe" = "iroha-zip-settings-$Version-windows-$architecture.exe"
        "iroha-zip-shell.exe" = "iroha-zip-shell-$Version-windows-$architecture.exe"
    }
    $builtFiles = @($assetMap.Keys | ForEach-Object { Join-Path $releaseRoot $_ })
    & (Join-Path $PSScriptRoot "verify-pe-architecture.ps1") `
        -Files $builtFiles `
        -Architecture $architecture

    $stagedFiles = [ordered]@{}
    foreach ($entry in $assetMap.GetEnumerator()) {
        $source = Join-Path $releaseRoot $entry.Key
        $destination = Join-Path $assetRoot $entry.Value
        Copy-Item -LiteralPath $source -Destination $destination
        $stagedFiles[$entry.Key] = $destination
    }

    $zipName = "iroha-zip-$Version-windows-$architecture.zip"
    $zipSource = Join-Path $distRoot $zipName
    $sidecarSource = "$zipSource.sha256"
    foreach ($source in @($zipSource, $sidecarSource)) {
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Release package output is missing: $source"
        }
        Copy-Item -LiteralPath $source -Destination (Join-Path $assetRoot ([IO.Path]::GetFileName($source)))
    }

    $zip = Join-Path $assetRoot $zipName
    $sidecar = "$zip.sha256"
    $zipHash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    $expectedSidecar = "$zipHash  $zipName`n"
    if ([IO.File]::ReadAllText($sidecar) -cne $expectedSidecar) {
        throw "ZIP checksum sidecar does not exactly match the staged ZIP: $([IO.Path]::GetFileName($sidecar))"
    }

    $actualNames = @(
        Get-ChildItem -LiteralPath $assetRoot -Force |
            ForEach-Object {
                if ($_.PSIsContainer -or $_.LinkType) {
                    throw "Release staging contains a directory or link: $($_.Name)"
                }
                $_.Name
            }
    )
    if ($actualNames.Count -ne 5) {
        throw "Expected exactly five $architecture release assets; found $($actualNames.Count)."
    }

    if (-not [string]::IsNullOrWhiteSpace($OutputFile)) {
        @(
            "architecture=$architecture"
            "asset_root=$assetRoot"
            "zip=$zip"
            "zip_checksum=$sidecar"
            "main_exe=$($stagedFiles['iroha-zip.exe'])"
            "settings_exe=$($stagedFiles['iroha-zip-settings.exe'])"
            "shell_exe=$($stagedFiles['iroha-zip-shell.exe'])"
        ) | Out-File -LiteralPath $OutputFile -Append -Encoding utf8
    }
    Write-Host "Staged exactly five $architecture assets in $assetRoot."
}
catch {
    if (Test-Path -LiteralPath $assetRoot) {
        Remove-Item -LiteralPath $assetRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
    throw
}
