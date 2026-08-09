[CmdletBinding()]
param(
    [string]$InstallDirectory = (Split-Path -Parent $PSScriptRoot),
    [switch]$DoNotOpenSettings
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$InstallDirectory = (Resolve-Path -LiteralPath $InstallDirectory).Path
$shellExe = Join-Path $InstallDirectory "safearc-shell.exe"
if (-not (Test-Path -LiteralPath $shellExe -PathType Leaf)) {
    throw "safearc-shell.exe was not found: $shellExe"
}

$progId = "SafeArc.Archive"
$extensions = @(
    ".zip", ".zipx", ".7z", ".rar", ".lzh", ".lha", ".tar", ".gz", ".tgz",
    ".bz2", ".tbz", ".tbz2", ".xz", ".txz", ".zst", ".tzst", ".z", ".cab"
)
$quotedCommand = '"' + $shellExe + '" "%1"'

$progKey = "HKCU:\Software\Classes\$progId"
New-Item -Path $progKey -Force | Out-Null
Set-Item -Path $progKey -Value "SafeArc archive"
New-Item -Path "$progKey\DefaultIcon" -Force | Out-Null
Set-Item -Path "$progKey\DefaultIcon" -Value ('"' + $shellExe + '",0')
New-Item -Path "$progKey\shell\open\command" -Force | Out-Null
Set-Item -Path "$progKey\shell\open\command" -Value $quotedCommand

$applicationKey = "HKCU:\Software\Classes\Applications\safearc-shell.exe"
New-Item -Path "$applicationKey\shell\open\command" -Force | Out-Null
Set-Item -Path "$applicationKey\shell\open\command" -Value $quotedCommand
New-Item -Path "$applicationKey\SupportedTypes" -Force | Out-Null

$capabilitiesKey = "HKCU:\Software\SafeArc\Capabilities"
New-Item -Path "$capabilitiesKey\FileAssociations" -Force | Out-Null
New-ItemProperty -Path $capabilitiesKey -Name "ApplicationName" -Value "SafeArc" `
    -PropertyType String -Force | Out-Null
New-ItemProperty -Path $capabilitiesKey -Name "ApplicationDescription" `
    -Value "Extract archives through a constrained libarchive process" `
    -PropertyType String -Force | Out-Null

foreach ($extension in $extensions) {
    $openWith = "HKCU:\Software\Classes\$extension\OpenWithProgids"
    New-Item -Path $openWith -Force | Out-Null
    New-ItemProperty -Path $openWith -Name $progId -Value "" -PropertyType String -Force | Out-Null
    New-ItemProperty -Path "$applicationKey\SupportedTypes" -Name $extension -Value "" `
        -PropertyType String -Force | Out-Null
    New-ItemProperty -Path "$capabilitiesKey\FileAssociations" -Name $extension -Value $progId `
        -PropertyType String -Force | Out-Null
}

$registeredApps = "HKCU:\Software\RegisteredApplications"
New-Item -Path $registeredApps -Force | Out-Null
New-ItemProperty -Path $registeredApps -Name "SafeArc" `
    -Value "Software\SafeArc\Capabilities" -PropertyType String -Force | Out-Null

Write-Host "SafeArc was registered as an archive-app candidate for the current user."
Write-Host "Windows must still confirm each default file association."
if (-not $DoNotOpenSettings) {
    Start-Process "ms-settings:defaultapps"
}
