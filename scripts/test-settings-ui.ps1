[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Executable
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

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

$executablePath = (Resolve-Path -LiteralPath $Executable).Path
$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) `
    ("iroha-zip-ui-日本語-" + [Guid]::NewGuid().ToString("N"))
$longDirectory = Join-Path $testRoot ("長い保存先-" + ("x" * 96))
$configPath = Join-Path $testRoot "設定.toml"
[System.IO.Directory]::CreateDirectory($longDirectory) | Out-Null

$process = $null
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
        $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Children,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        foreach ($candidate in $windows) {
            if ($candidate.Current.ProcessId -eq $process.Id -and $candidate -ne $window) {
                return $candidate
            }
        }
        return $null
    }
    if ($confirmation.Current.ControlType -ne [System.Windows.Automation.ControlType]::Window) {
        throw "Unsaved-change confirmation was not exposed as an accessible window."
    }

    Write-Host "Settings UI Automation contract passed for 26 controls."
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
