[CmdletBinding()]
param(
    [string]$Msys2Root = "C:\msys64",
    [string]$DestinationDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($DestinationDirectory)) {
    $DestinationDirectory = Join-Path $ProjectRoot "backend\libarchive"
}

$Msys2Root = [System.IO.Path]::GetFullPath($Msys2Root)
$bash = Join-Path $Msys2Root "usr\bin\bash.exe"
$bsdtar = Join-Path $Msys2Root "ucrt64\bin\bsdtar.exe"
if (-not (Test-Path -LiteralPath $bash -PathType Leaf)) {
    throw "MSYS2 bash.exe was not found: $bash"
}
if (-not (Test-Path -LiteralPath $bsdtar -PathType Leaf)) {
    throw @"
MSYS2 UCRT64 bsdtar.exe was not found: $bsdtar
Install it from an MSYS2 UCRT64 shell:
  pacman -S mingw-w64-ucrt-x86_64-libarchive
"@
}

function Invoke-Ldd([string]$UnixPath) {
    # Pass the path as bash $1 instead of interpolating it into shell syntax.
    $output = @(
        & $bash --noprofile --norc -lc 'PATH=/ucrt64/bin:/usr/bin ldd "$1"' `
            safearc-ldd $UnixPath 2>&1
    )
    if ($LASTEXITCODE -ne 0) {
        throw "ldd failed for $UnixPath`n$($output -join "`n")"
    }
    return $output
}

function Convert-UcrtPath([string]$UnixPath) {
    if (-not $UnixPath.StartsWith("/ucrt64/bin/", [System.StringComparison]::Ordinal)) {
        throw "Not a UCRT64 binary path: $UnixPath"
    }
    $name = $UnixPath.Substring("/ucrt64/bin/".Length).Replace('/', '\')
    return Join-Path (Join-Path $Msys2Root "ucrt64\bin") $name
}

$pending = [System.Collections.Generic.Queue[string]]::new()
$seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
$pending.Enqueue("/ucrt64/bin/bsdtar.exe")

while ($pending.Count -gt 0) {
    $current = $pending.Dequeue()
    if (-not $seen.Add($current)) {
        continue
    }

    foreach ($lineObject in (Invoke-Ldd $current)) {
        $line = [string]$lineObject
        $dependency = $null
        if ($line -match '=>\s+(/ucrt64/bin/[^\s]+)') {
            $dependency = $Matches[1]
        }
        elseif ($line -match '^\s*(/ucrt64/bin/[^\s]+)\s+\(') {
            $dependency = $Matches[1]
        }
        if ($null -ne $dependency -and -not $seen.Contains($dependency)) {
            $pending.Enqueue($dependency)
        }
    }
}

$temporaryBundle = Join-Path ([System.IO.Path]::GetTempPath()) ("safearc-msys2-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $temporaryBundle | Out-Null
try {
    foreach ($unixPath in ($seen | Sort-Object)) {
        $source = Convert-UcrtPath $unixPath
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "A dependency reported by ldd is missing: $source"
        }
        $target = Join-Path $temporaryBundle ([System.IO.Path]::GetFileName($source))
        Copy-Item -LiteralPath $source -Destination $target
    }

    & (Join-Path $PSScriptRoot "install-backend.ps1") `
        -SourceDirectory $temporaryBundle `
        -DestinationDirectory $DestinationDirectory

    Write-Host "Collected UCRT64 runtime files: $($seen.Count)"
    Write-Host "Backend source: MSYS2 $Msys2Root"
}
finally {
    if (Test-Path -LiteralPath $temporaryBundle) {
        Remove-Item -LiteralPath $temporaryBundle -Recurse -Force
    }
}
