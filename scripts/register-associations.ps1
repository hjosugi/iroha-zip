[CmdletBinding()]
param(
    [string]$InstallDirectory = (Split-Path -Parent $PSScriptRoot),
    [switch]$DoNotOpenSettings
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$InstallDirectory = (Resolve-Path -LiteralPath $InstallDirectory).Path
$shellExe = Join-Path $InstallDirectory "iroha-zip-shell.exe"
if (-not (Test-Path -LiteralPath $shellExe -PathType Leaf)) {
    throw "iroha-zip-shell.exe was not found: $shellExe"
}

$progId = "iroha-zip.Archive"
$extensions = @(
    ".zip", ".zipx", ".7z", ".rar", ".lzh", ".lha", ".tar", ".gz", ".tgz",
    ".bz2", ".tbz", ".tbz2", ".xz", ".txz", ".zst", ".tzst", ".z", ".cab"
)
$quotedCommand = '"' + $shellExe + '" "%1"'

$progKey = "HKCU:\Software\Classes\$progId"
New-Item -Path $progKey -Force | Out-Null
Set-Item -Path $progKey -Value "iroha-zip archive"
New-Item -Path "$progKey\DefaultIcon" -Force | Out-Null
Set-Item -Path "$progKey\DefaultIcon" -Value ('"' + $shellExe + '",0')
New-Item -Path "$progKey\shell\open\command" -Force | Out-Null
Set-Item -Path "$progKey\shell\open\command" -Value $quotedCommand

$applicationKey = "HKCU:\Software\Classes\Applications\iroha-zip-shell.exe"
New-Item -Path "$applicationKey\shell\open\command" -Force | Out-Null
Set-Item -Path "$applicationKey\shell\open\command" -Value $quotedCommand
New-Item -Path "$applicationKey\SupportedTypes" -Force | Out-Null

$capabilitiesKey = "HKCU:\Software\iroha-zip\Capabilities"
New-Item -Path "$capabilitiesKey\FileAssociations" -Force | Out-Null
New-ItemProperty -Path $capabilitiesKey -Name "ApplicationName" -Value "iroha-zip" `
    -PropertyType String -Force | Out-Null
New-ItemProperty -Path $capabilitiesKey -Name "ApplicationDescription" `
    -Value "Extract archives through a constrained libarchive process" `
    -PropertyType String -Force | Out-Null

foreach ($extension in $extensions) {
    $openWith = "HKCU:\Software\Classes\$extension\OpenWithProgids"
    if (-not (Test-Path -LiteralPath $openWith)) {
        New-Item -Path $openWith | Out-Null
    }
    New-ItemProperty -Path $openWith -Name $progId -Value "" -PropertyType String -Force | Out-Null
    New-ItemProperty -Path "$applicationKey\SupportedTypes" -Name $extension -Value "" `
        -PropertyType String -Force | Out-Null
    New-ItemProperty -Path "$capabilitiesKey\FileAssociations" -Name $extension -Value $progId `
        -PropertyType String -Force | Out-Null
}

$registeredApps = "HKCU:\Software\RegisteredApplications"
if (-not (Test-Path -LiteralPath $registeredApps)) {
    New-Item -Path $registeredApps | Out-Null
}
New-ItemProperty -Path $registeredApps -Name "iroha-zip" `
    -Value "Software\iroha-zip\Capabilities" -PropertyType String -Force | Out-Null

Write-Host "iroha-zip was registered as an archive-app candidate for the current user."
Write-Host "Windows must still confirm each default file association."
if (-not $DoNotOpenSettings) {
    Start-Process "ms-settings:defaultapps"
}
