[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string[]]$Files,

    [Parameter(Mandatory = $true)]
    [ValidateSet("x64", "arm64")]
    [string]$Architecture
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$expectedMachine = switch ($Architecture) {
    "x64" { [uint16]0x8664 }
    "arm64" { [uint16]0xaa64 }
}

$seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($file in $Files) {
    $item = Get-Item -LiteralPath $file -Force
    if ($item.PSIsContainer -or $item.LinkType) {
        throw "PE input must be a regular, non-link file: $file"
    }
    if (-not $seen.Add($item.FullName)) {
        throw "Duplicate PE input: $($item.FullName)"
    }

    $bytes = [IO.File]::ReadAllBytes($item.FullName)
    if ($bytes.Length -lt 70 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) {
        throw "PE input has no valid DOS header: $($item.Name)"
    }
    $peOffset = [BitConverter]::ToInt32($bytes, 0x3c)
    if ($peOffset -lt 0 -or $peOffset -gt ($bytes.Length - 6)) {
        throw "PE header offset is outside the file: $($item.Name)"
    }
    if ($bytes[$peOffset] -ne 0x50 -or $bytes[$peOffset + 1] -ne 0x45 -or
        $bytes[$peOffset + 2] -ne 0 -or $bytes[$peOffset + 3] -ne 0) {
        throw "PE input has no valid PE signature: $($item.Name)"
    }
    $actualMachine = [BitConverter]::ToUInt16($bytes, $peOffset + 4)
    if ($actualMachine -ne $expectedMachine) {
        throw (
            "PE machine mismatch for {0}: expected {1} (0x{2:X4}), found 0x{3:X4}" -f
            $item.Name,
            $Architecture,
            $expectedMachine,
            $actualMachine
        )
    }
}

Write-Host (
    "Verified {0} {1} PE file(s) with machine 0x{2:X4}." -f
    $Files.Count,
    $Architecture,
    $expectedMachine
)
