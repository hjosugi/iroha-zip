[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Executable,

    [string]$BackendDirectory,

    [string]$EvidenceOutput
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class IrohaZipUiAutomationNative {
    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessageW(IntPtr window, uint message, UIntPtr wParam, IntPtr lParam);
}
"@

$ButtonClickMessage = 0x00F5

function Wait-Until {
    param(
        [scriptblock]$Condition,
        [int]$TimeoutSeconds = 15
    )
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        $value = & $Condition
        if ($null -ne $value -and $value -ne $false) { return $value }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed out waiting for the settings UI."
}

function Find-ByAutomationId {
    param(
        [System.Windows.Automation.AutomationElement]$Root,
        [int]$Id
    )
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty,
        [string]$Id
    )
    return $Root.FindFirst(
        [System.Windows.Automation.TreeScope]::Descendants,
        $condition
    )
}

function Require-Control {
    param(
        [System.Windows.Automation.AutomationElement]$Window,
        [int]$Id,
        [System.Windows.Automation.ControlType]$Type
    )
    $control = Find-ByAutomationId -Root $Window -Id $Id
    if ($null -eq $control) { throw "Missing UI Automation control ID $Id." }
    if ($control.Current.ControlType -ne $Type) {
        throw "Control $Id has type $($control.Current.ControlType.ProgrammaticName), expected $($Type.ProgrammaticName)."
    }
    if (-not $control.Current.IsEnabled) { throw "Control $Id is disabled." }
    if (-not $control.Current.IsKeyboardFocusable) { throw "Control $Id is not keyboard-focusable." }
    if ([string]::IsNullOrWhiteSpace($control.Current.Name)) {
        throw "Control $Id has no accessible name."
    }
    if ([string]::IsNullOrWhiteSpace($control.Current.AccessKey)) {
        throw "Control $Id has no accessible access key."
    }
    $bounds = $control.Current.BoundingRectangle
    if ($bounds.Width -le 0 -or $bounds.Height -le 0) {
        throw "Control $Id has empty bounds."
    }
    $control.SetFocus()
    return $control
}

function Find-SecondaryWindow {
    param(
        [System.Diagnostics.Process]$Process,
        [System.Windows.Automation.AutomationElement]$MainWindow
    )
    $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Children,
        [System.Windows.Automation.Condition]::TrueCondition
    )
    foreach ($candidate in $windows) {
        if ($candidate.Current.ProcessId -eq $Process.Id -and
            $candidate.Current.NativeWindowHandle -ne $MainWindow.Current.NativeWindowHandle) {
            return $candidate
        }
    }
    return $null
}

function Dismiss-Message {
    param([System.Windows.Automation.AutomationElement]$Dialog)
    $button = Find-ByAutomationId -Root $Dialog -Id 1
    if ($null -eq $button) {
        throw "The settings message did not expose its OK button."
    }
    if (-not [IrohaZipUiAutomationNative]::PostMessageW(
        [IntPtr]$button.Current.NativeWindowHandle,
        $ButtonClickMessage,
        [UIntPtr]::Zero,
        [IntPtr]::Zero
    )) {
        throw "Cannot activate the settings message OK button."
    }
}

function Invoke-Control {
    param([System.Windows.Automation.AutomationElement]$Control)
    if (-not [IrohaZipUiAutomationNative]::PostMessageW(
        [IntPtr]$Control.Current.NativeWindowHandle,
        $ButtonClickMessage,
        [UIntPtr]::Zero,
        [IntPtr]::Zero
    )) {
        throw "Cannot activate control $($Control.Current.AutomationId)."
    }
}

if ([string]::IsNullOrWhiteSpace($BackendDirectory) -ne
    [string]::IsNullOrWhiteSpace($EvidenceOutput)) {
    throw "-BackendDirectory and -EvidenceOutput must be provided together."
}

$executablePath = (Resolve-Path -LiteralPath $Executable).Path
$backendPath = $null
$evidencePath = $null
if (-not [string]::IsNullOrWhiteSpace($BackendDirectory)) {
    $backendPath = (Resolve-Path -LiteralPath $BackendDirectory).Path
    if (-not (Test-Path -LiteralPath $backendPath -PathType Container)) {
        throw "BackendDirectory is not a directory: $BackendDirectory"
    }
    $evidencePath = [System.IO.Path]::GetFullPath($EvidenceOutput)
}
$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) `
    ("iroha-zip-ui-日本語-" + [Guid]::NewGuid().ToString("N"))
$longDirectory = Join-Path $testRoot ("長い保存先-" + ("x" * 96))
$configPath = Join-Path $testRoot "設定.toml"
[System.IO.Directory]::CreateDirectory($longDirectory) | Out-Null

$process = $null
$setupEvidence = $null
try {
    $process = Start-Process -FilePath $executablePath `
        -ArgumentList @("--config", $configPath) -PassThru
    $window = Wait-Until {
        $condition = [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
            $process.Id
        )
        [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
            [System.Windows.Automation.TreeScope]::Children,
            $condition
        )
    }
    if ($window.Current.Name -notlike "iroha-zip 設定*") {
        throw "Unexpected settings window name: $($window.Current.Name)"
    }

    $editIds = @(2001, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011)
    $comboIds = @(2002, 2014, 2015)
    $checkBoxIds = @(2012, 2013)
    $buttonIds = @(1001, 1002, 1003, 1004, 1101, 1102, 1103, 1104, 1201, 1, 2)

    $controls = @{}
    foreach ($id in $editIds) {
        $controls[$id] = Require-Control -Window $window -Id $id `
            -Type ([System.Windows.Automation.ControlType]::Edit)
    }
    foreach ($id in $comboIds) {
        $controls[$id] = Require-Control -Window $window -Id $id `
            -Type ([System.Windows.Automation.ControlType]::ComboBox)
    }
    foreach ($id in $checkBoxIds) {
        $controls[$id] = Require-Control -Window $window -Id $id `
            -Type ([System.Windows.Automation.ControlType]::CheckBox)
    }
    foreach ($id in $buttonIds) {
        $controls[$id] = Require-Control -Window $window -Id $id `
            -Type ([System.Windows.Automation.ControlType]::Button)
    }

    $pathPattern = [System.Windows.Automation.ValuePattern]$controls[2001].GetCurrentPattern(
        [System.Windows.Automation.ValuePattern]::Pattern
    )
    $pathPattern.SetValue($longDirectory)
    $timeoutPattern = [System.Windows.Automation.ValuePattern]$controls[2003].GetCurrentPattern(
        [System.Windows.Automation.ValuePattern]::Pattern
    )
    $timeoutPattern.SetValue("301")
    $motwPattern = [System.Windows.Automation.TogglePattern]$controls[2012].GetCurrentPattern(
        [System.Windows.Automation.TogglePattern]::Pattern
    )
    $motwPattern.Toggle()

    Wait-Until {
        $name = $window.GetCurrentPropertyValue(
            [System.Windows.Automation.AutomationElement]::NameProperty
        )
        $name -match '\s\*$'
    } | Out-Null
    if (Test-Path -LiteralPath $configPath) {
        throw "The UI automation smoke test must not save its temporary configuration."
    }

    $windowPattern = [System.Windows.Automation.WindowPattern]$window.GetCurrentPattern(
        [System.Windows.Automation.WindowPattern]::Pattern
    )
    $windowPattern.Close()
    $confirmation = Wait-Until {
        Find-SecondaryWindow -Process $process -MainWindow $window
    }
    if ($confirmation.Current.ControlType -ne [System.Windows.Automation.ControlType]::Window) {
        throw "Unsaved-change confirmation was not exposed as an accessible window."
    }

    Write-Host "Settings UI Automation contract passed for 26 controls."

    if ($null -ne $backendPath) {
        $process.Kill()
        $process.WaitForExit()
        $process = Start-Process -FilePath $executablePath `
            -ArgumentList @("--config", $configPath) -PassThru
        $window = Wait-Until {
            $condition = [System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
                $process.Id
            )
            [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
                [System.Windows.Automation.TreeScope]::Children,
                $condition
            )
        }

        $backendControl = Require-Control -Window $window -Id 2001 `
            -Type ([System.Windows.Automation.ControlType]::Edit)
        $backendPattern = [System.Windows.Automation.ValuePattern]$backendControl.GetCurrentPattern(
            [System.Windows.Automation.ValuePattern]::Pattern
        )
        $backendPattern.SetValue($backendPath)

        $save = Require-Control -Window $window -Id 1 `
            -Type ([System.Windows.Automation.ControlType]::Button)
        Invoke-Control $save
        $savedMessage = Wait-Until {
            Find-SecondaryWindow -Process $process -MainWindow $window
        }
        Dismiss-Message $savedMessage
        Wait-Until {
            $null -eq (Find-SecondaryWindow -Process $process -MainWindow $window)
        } | Out-Null
        Wait-Until { Test-Path -LiteralPath $configPath -PathType Leaf } | Out-Null
        $savedConfig = [System.IO.File]::ReadAllText($configPath)
        $escapedBackendPath = $backendPath.Replace(
            [string][char]92,
            ([string][char]92 + [char]92)
        )
        if (-not $savedConfig.Contains($backendPath) -and
            -not $savedConfig.Contains($escapedBackendPath)) {
            throw "The settings screen did not persist the selected backend path."
        }

        $doctor = Require-Control -Window $window -Id 1002 `
            -Type ([System.Windows.Automation.ControlType]::Button)
        $doctorStarted = [DateTime]::UtcNow
        Invoke-Control $doctor
        $doctorMessage = Wait-Until -TimeoutSeconds 90 -Condition {
            Find-SecondaryWindow -Process $process -MainWindow $window
        }
        $textCondition = [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Text
        )
        $doctorText = @(
            $doctorMessage.FindAll(
                [System.Windows.Automation.TreeScope]::Descendants,
                $textCondition
            ) | ForEach-Object { $_.Current.Name }
        ) -join "`n"
        if ($doctorText -notmatch "診断に成功") {
            throw "Settings-screen backend/AppContainer diagnostic did not report success: $doctorText"
        }
        Dismiss-Message $doctorMessage
        Wait-Until {
            $null -eq (Find-SecondaryWindow -Process $process -MainWindow $window)
        } | Out-Null

        $setupEvidence = [ordered]@{
            schemaVersion = 1
            status = "passed"
            generatedAtUtc = [DateTime]::UtcNow.ToString("o")
            controlCount = 26
            longAndNonAsciiPathEdited = $true
            unsavedChangeConfirmationExposed = $true
            backendPathSaved = $true
            backendDoctorSucceeded = $true
            doctorElapsedMilliseconds = [int64]([DateTime]::UtcNow - $doctorStarted).TotalMilliseconds
            configSha256 = (Get-FileHash -LiteralPath $configPath -Algorithm SHA256).Hash.ToLowerInvariant()
            settingsExecutableSha256 = (Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash.ToLowerInvariant()
            temporaryRootRemoved = $false
        }

        $windowPattern = [System.Windows.Automation.WindowPattern]$window.GetCurrentPattern(
            [System.Windows.Automation.WindowPattern]::Pattern
        )
        $windowPattern.Close()
        if (-not $process.WaitForExit(15000)) {
            throw "Settings application did not exit after a clean saved-state close."
        }
    }
}
finally {
    if ($null -ne $process -and -not $process.HasExited) {
        $process.Kill()
        $process.WaitForExit()
    }
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}

if ($null -ne $setupEvidence) {
    $setupEvidence.temporaryRootRemoved = -not (Test-Path -LiteralPath $testRoot)
    $evidenceParent = Split-Path -Parent $evidencePath
    [System.IO.Directory]::CreateDirectory($evidenceParent) | Out-Null
    [System.IO.File]::WriteAllText(
        $evidencePath,
        "$(ConvertTo-Json -InputObject $setupEvidence -Depth 10)`n",
        [System.Text.UTF8Encoding]::new($false)
    )
    Write-Host "Settings setup evidence: $evidencePath"
}
