[CmdletBinding(SupportsShouldProcess)]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet(
        "backend_manifest",
        "windows_paths",
        "archive_name",
        "command_line",
        "config_round_trip"
    )]
    [string]$Target,

    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$Artifact
)

$ErrorActionPreference = "Stop"
$repository = Split-Path -Parent $PSScriptRoot
$resolvedArtifact = (Resolve-Path -LiteralPath $Artifact).Path
$length = (Get-Item -LiteralPath $resolvedArtifact).Length
if ($length -gt 65536) {
    throw "Regression input exceeds the 65,536-byte fuzz limit: $length bytes"
}

$digest = (Get-FileHash -LiteralPath $resolvedArtifact -Algorithm SHA256).Hash.ToLowerInvariant()
$directory = Join-Path $repository "fuzz/regressions/$Target"
$destination = Join-Path $directory "$digest.bin"

if ($PSCmdlet.ShouldProcess($destination, "Promote minimized fuzz regression")) {
    New-Item -ItemType Directory -Path $directory -Force | Out-Null
    Copy-Item -LiteralPath $resolvedArtifact -Destination $destination -ErrorAction Stop
    Write-Output $destination
}
