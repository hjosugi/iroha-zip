[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$ShellExecutable
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$extensions = @(
    ".zip", ".zipx", ".7z", ".rar", ".lzh", ".lha", ".tar", ".gz", ".tgz",
    ".bz2", ".tbz", ".tbz2", ".xz", ".txz", ".zst", ".tzst", ".z", ".cab"
)
$progId = "iroha-zip.Archive"
$progKey = "HKCU:\Software\Classes\$progId"
$applicationKey = "HKCU:\Software\Classes\Applications\iroha-zip-shell.exe"
$capabilitiesKey = "HKCU:\Software\iroha-zip\Capabilities"
$registeredApps = "HKCU:\Software\RegisteredApplications"
$ownedKeys = @($progKey, $applicationKey, "HKCU:\Software\iroha-zip")

foreach ($ownedKey in $ownedKeys) {
    if (Test-Path -LiteralPath $ownedKey) {
        throw "Association test refuses to replace pre-existing iroha-zip state: $ownedKey"
    }
}
if (Test-Path -LiteralPath $registeredApps) {
    $existingRegistration = Get-ItemPropertyValue -LiteralPath $registeredApps `
        -Name "iroha-zip" -ErrorAction SilentlyContinue
    if ($null -ne $existingRegistration) {
        throw "Association test refuses to replace a pre-existing RegisteredApplications value."
    }
}

$testRoot = Join-Path ([IO.Path]::GetTempPath()) `
    ("iroha-zip-association-日本語-" + [Guid]::NewGuid().ToString("N"))
$installRoot = Join-Path $testRoot "install path 日本語"
$testShell = Join-Path $installRoot "iroha-zip-shell.exe"
$sentinelName = "unrelated-" + [Guid]::NewGuid().ToString("N")
$sentinelValue = "preserve-" + [Guid]::NewGuid().ToString("N")
$createdOpenWithKeys = [Collections.Generic.List[string]]::new()
$registeredAppsCreated = $false

try {
    [IO.Directory]::CreateDirectory($installRoot) | Out-Null
    Copy-Item -LiteralPath $ShellExecutable -Destination $testShell

    foreach ($extension in $extensions) {
        $openWith = "HKCU:\Software\Classes\$extension\OpenWithProgids"
        if (-not (Test-Path -LiteralPath $openWith)) {
            New-Item -Path $openWith -Force | Out-Null
            $createdOpenWithKeys.Add($openWith)
        }
        New-ItemProperty -Path $openWith -Name $sentinelName -Value $sentinelValue `
            -PropertyType String -Force | Out-Null
    }
    if (-not (Test-Path -LiteralPath $registeredApps)) {
        New-Item -Path $registeredApps -Force | Out-Null
        $registeredAppsCreated = $true
    }
    New-ItemProperty -Path $registeredApps -Name $sentinelName -Value $sentinelValue `
        -PropertyType String -Force | Out-Null

    foreach ($attempt in 1..2) {
        & (Join-Path $PSScriptRoot "register-associations.ps1") `
            -InstallDirectory $installRoot `
            -DoNotOpenSettings

        $expectedCommand = '"' + $testShell + '" "%1"'
        if ((Get-Item -LiteralPath $progKey).GetValue("") -cne "iroha-zip archive" -or
            (Get-Item -LiteralPath "$progKey\DefaultIcon").GetValue("") -cne `
                ('"' + $testShell + '",0') -or
            (Get-Item -LiteralPath "$progKey\shell\open\command").GetValue("") -cne `
                $expectedCommand -or
            (Get-Item -LiteralPath "$applicationKey\shell\open\command").GetValue("") -cne `
                $expectedCommand) {
            throw "Association command or icon registration was not exact on attempt $attempt."
        }

        foreach ($extension in $extensions) {
            $openWith = "HKCU:\Software\Classes\$extension\OpenWithProgids"
            $candidate = Get-ItemPropertyValue -LiteralPath $openWith -Name $progId
            $supported = Get-ItemPropertyValue `
                -LiteralPath "$applicationKey\SupportedTypes" -Name $extension
            $association = Get-ItemPropertyValue `
                -LiteralPath "$capabilitiesKey\FileAssociations" -Name $extension
            if ([string]$candidate -cne "" -or [string]$supported -cne "" -or
                [string]$association -cne $progId) {
                throw "Association registration is incomplete for $extension on attempt $attempt."
            }
        }
        if ((Get-ItemPropertyValue -LiteralPath $capabilitiesKey -Name "ApplicationName") `
                -cne "iroha-zip" -or
            (Get-ItemPropertyValue -LiteralPath $registeredApps -Name "iroha-zip") `
                -cne "Software\iroha-zip\Capabilities") {
            throw "Application capabilities registration is incomplete on attempt $attempt."
        }
    }

    & (Join-Path $PSScriptRoot "unregister-associations.ps1")

    foreach ($ownedKey in $ownedKeys) {
        if (Test-Path -LiteralPath $ownedKey) {
            throw "Association removal left an owned registry key: $ownedKey"
        }
    }
    foreach ($extension in $extensions) {
        $openWith = "HKCU:\Software\Classes\$extension\OpenWithProgids"
        if ($null -ne (Get-ItemPropertyValue -LiteralPath $openWith -Name $progId `
                -ErrorAction SilentlyContinue)) {
            throw "Association removal left the project candidate for $extension."
        }
        if ((Get-ItemPropertyValue -LiteralPath $openWith -Name $sentinelName) `
                -cne $sentinelValue) {
            throw "Association removal damaged unrelated state for $extension."
        }
    }
    if ($null -ne (Get-ItemPropertyValue -LiteralPath $registeredApps -Name "iroha-zip" `
            -ErrorAction SilentlyContinue) -or
        (Get-ItemPropertyValue -LiteralPath $registeredApps -Name $sentinelName) `
            -cne $sentinelValue) {
        throw "Association removal damaged RegisteredApplications state."
    }

    Write-Host "Association registration/removal passed for 18 extensions without changing UserChoice."
}
finally {
    & (Join-Path $PSScriptRoot "unregister-associations.ps1")
    foreach ($extension in $extensions) {
        $openWith = "HKCU:\Software\Classes\$extension\OpenWithProgids"
        if (Test-Path -LiteralPath $openWith) {
            Remove-ItemProperty -LiteralPath $openWith -Name $sentinelName `
                -ErrorAction SilentlyContinue
        }
    }
    foreach ($openWith in $createdOpenWithKeys) {
        if (Test-Path -LiteralPath $openWith) {
            $key = Get-Item -LiteralPath $openWith
            if ($key.SubKeyCount -eq 0 -and $key.ValueCount -eq 0) {
                Remove-Item -LiteralPath $openWith -Force
            }
        }
    }
    if (Test-Path -LiteralPath $registeredApps) {
        Remove-ItemProperty -LiteralPath $registeredApps -Name $sentinelName `
            -ErrorAction SilentlyContinue
        if ($registeredAppsCreated) {
            $key = Get-Item -LiteralPath $registeredApps
            if ($key.SubKeyCount -eq 0 -and $key.ValueCount -eq 0) {
                Remove-Item -LiteralPath $registeredApps -Force
            }
        }
    }
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
