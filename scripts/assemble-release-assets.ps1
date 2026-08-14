[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Container })]
    [string]$AssetRoot,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9]+\.[0-9]+\.[0-9]+$')]
    [string]$Version,

    [string]$OutputFile
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$expectedNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($architecture in @("x64", "arm64")) {
    foreach ($name in @(
        "iroha-zip-$Version-windows-$architecture.zip",
        "iroha-zip-$Version-windows-$architecture.zip.sha256",
        "iroha-zip-$Version-windows-$architecture.exe",
        "iroha-zip-settings-$Version-windows-$architecture.exe",
        "iroha-zip-shell-$Version-windows-$architecture.exe"
    )) {
        if (-not $expectedNames.Add($name)) {
            throw "Duplicate expected release asset name: $name"
        }
    }
}

$actualItems = @(Get-ChildItem -LiteralPath $AssetRoot -Force)
if ($actualItems.Count -ne $expectedNames.Count) {
    throw "Expected exactly $($expectedNames.Count) architecture assets; found $($actualItems.Count)."
}
$seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($item in $actualItems) {
    if ($item.PSIsContainer -or $item.LinkType) {
        throw "Release assembly input must be a regular, non-link file: $($item.Name)"
    }
    if (-not $expectedNames.Contains($item.Name)) {
        throw "Unexpected architecture release asset: $($item.Name)"
    }
    if (-not $seen.Add($item.Name)) {
        throw "Duplicate architecture release asset: $($item.Name)"
    }
}

$temporaryRoots = @()
try {
    foreach ($architecture in @("x64", "arm64")) {
        $standalone = @(
            Join-Path $AssetRoot "iroha-zip-$Version-windows-$architecture.exe"
            Join-Path $AssetRoot "iroha-zip-settings-$Version-windows-$architecture.exe"
            Join-Path $AssetRoot "iroha-zip-shell-$Version-windows-$architecture.exe"
        )
        & (Join-Path $PSScriptRoot "verify-pe-architecture.ps1") `
            -Files $standalone `
            -Architecture $architecture

        $zipName = "iroha-zip-$Version-windows-$architecture.zip"
        $zip = Join-Path $AssetRoot $zipName
        $sidecar = "$zip.sha256"
        $zipHash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
        if ([IO.File]::ReadAllText($sidecar) -cne "$zipHash  $zipName`n") {
            throw "ZIP checksum sidecar does not exactly match $zipName."
        }

        $expanded = Join-Path ([IO.Path]::GetTempPath()) (
            "iroha-zip-release-assembly-" + [Guid]::NewGuid().ToString("N")
        )
        $temporaryRoots += $expanded
        Expand-Archive -LiteralPath $zip -DestinationPath $expanded
        $topLevel = @(Get-ChildItem -LiteralPath $expanded -Force)
        if ($topLevel.Count -ne 1 -or -not $topLevel[0].PSIsContainer -or
            $topLevel[0].Name -cne "iroha-zip" -or $topLevel[0].LinkType) {
            throw "$zipName must contain exactly one regular iroha-zip root directory."
        }
        $packaged = @(
            Join-Path $topLevel[0].FullName "iroha-zip.exe"
            Join-Path $topLevel[0].FullName "iroha-zip-settings.exe"
            Join-Path $topLevel[0].FullName "iroha-zip-shell.exe"
        )
        & (Join-Path $PSScriptRoot "verify-pe-architecture.ps1") `
            -Files $packaged `
            -Architecture $architecture

        $expectedPortableFiles = [Collections.Generic.HashSet[string]]::new(
            [StringComparer]::OrdinalIgnoreCase
        )
        foreach ($path in $packaged) {
            $expectedPortableFiles.Add([IO.Path]::GetFullPath($path)) | Out-Null
        }
        $expandedItems = @(Get-ChildItem -LiteralPath $topLevel[0].FullName -Recurse -Force)
        foreach ($item in $expandedItems) {
            if ($item.LinkType) {
                throw "$zipName contains a link after expansion: $($item.FullName)"
            }
        }
        $portableFiles = @($expandedItems | Where-Object {
            -not $_.PSIsContainer -and $_.Extension -in @(".exe", ".dll", ".msi", ".pdb")
        })
        if ($portableFiles.Count -ne 3) {
            throw "$zipName must contain exactly the three expected portable executable files."
        }
        foreach ($item in $portableFiles) {
            if (-not $expectedPortableFiles.Contains($item.FullName)) {
                throw "$zipName contains an unexpected portable executable file: $($item.FullName)"
            }
        }
        $backendItems = @(Get-ChildItem -LiteralPath (Join-Path $topLevel[0].FullName "backend") -Force)
        if ($backendItems.Count -ne 1 -or $backendItems[0].Name -cne "README.md" -or
            $backendItems[0].PSIsContainer -or $backendItems[0].LinkType) {
            throw "$zipName contains an unexpected third-party backend payload."
        }
    }

    $coveredNames = @(
        $actualItems |
            Where-Object { $_.Name -notlike "*.sha256" } |
            ForEach-Object { $_.Name } |
            Sort-Object
    )
    if ($coveredNames.Count -ne 8) {
        throw "Expected eight ZIP/EXE checksum subjects; found $($coveredNames.Count)."
    }
    $checksumLines = foreach ($name in $coveredNames) {
        $path = Join-Path $AssetRoot $name
        $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $name"
    }
    $checksums = Join-Path $AssetRoot "SHA256SUMS.txt"
    [IO.File]::WriteAllText(
        $checksums,
        (($checksumLines -join "`n") + "`n"),
        [Text.UTF8Encoding]::new($false)
    )

    $finalItems = @(Get-ChildItem -LiteralPath $AssetRoot -Force)
    if ($finalItems.Count -ne 11) {
        throw "Expected exactly 11 final release assets; found $($finalItems.Count)."
    }
    if (-not [string]::IsNullOrWhiteSpace($OutputFile)) {
        @(
            "asset_root=$AssetRoot"
            "checksums=$checksums"
        ) | Out-File -LiteralPath $OutputFile -Append -Encoding utf8
    }
    Write-Host "Assembled exactly 11 architecture-separated release assets."
}
finally {
    foreach ($temporaryRoot in $temporaryRoots) {
        if (Test-Path -LiteralPath $temporaryRoot) {
            Remove-Item -LiteralPath $temporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
