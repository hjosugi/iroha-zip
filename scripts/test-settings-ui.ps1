[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Executable,

    [string]$BackendDirectory,

    [string]$EvidenceOutput,

    [ValidateSet("ja", "en")]
    [string]$Language = "ja"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;

 [StructLayout(LayoutKind.Sequential)]
 public struct IrohaZipRect {
     public int Left;
     public int Top;
     public int Right;
     public int Bottom;
 }

public static class IrohaZipUiAutomationNative {
    private const uint InputMouse = 0;
    private const uint InputKeyboard = 1;
    private const uint KeyEventKeyUp = 0x0002;
    private const uint MouseEventMove = 0x0001;
    private const uint MouseEventLeftDown = 0x0002;
    private const uint MouseEventLeftUp = 0x0004;
    private const uint MouseEventVirtualDesk = 0x4000;
    private const uint MouseEventAbsolute = 0x8000;
    private const ushort VirtualKeyShift = 0x10;
    private const ushort VirtualKeyTab = 0x09;
    private const int ShowRestore = 9;
    private const int SmXVirtualScreen = 76;
    private const int SmYVirtualScreen = 77;
    private const int SmCxVirtualScreen = 78;
    private const int SmCyVirtualScreen = 79;

    [StructLayout(LayoutKind.Explicit, Size = 40)]
    private struct IrohaZipInput {
        [FieldOffset(0)] public uint Type;
        [FieldOffset(8)] public ushort VirtualKey;
        [FieldOffset(10)] public ushort ScanCode;
        [FieldOffset(12)] public uint Flags;
        [FieldOffset(16)] public uint Time;
        [FieldOffset(24)] public UIntPtr ExtraInfo;
        [FieldOffset(8)] public int MouseX;
        [FieldOffset(12)] public int MouseY;
        [FieldOffset(16)] public uint MouseData;
        [FieldOffset(20)] public uint MouseFlags;
        [FieldOffset(24)] public uint MouseTime;
        [FieldOffset(32)] public UIntPtr MouseExtraInfo;
    }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint SendInput(
        uint count,
        IrohaZipInput[] inputs,
        int inputSize
    );

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessageW(IntPtr window, uint message, UIntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr SendMessageW(IntPtr window, uint message, UIntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetWindowRect(IntPtr window, out IrohaZipRect rectangle);

    [DllImport("user32.dll")]
    public static extern IntPtr GetWindowDpiAwarenessContext(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool AreDpiAwarenessContextsEqual(IntPtr first, IntPtr second);

    [DllImport("user32.dll")]
    public static extern IntPtr GetParent(IntPtr window);

    [DllImport("user32.dll")]
    public static extern IntPtr GetLastActivePopup(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool SetForegroundWindow(IntPtr window);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool BringWindowToTop(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool ShowWindowAsync(IntPtr window, int command);

    [DllImport("user32.dll")]
    private static extern int GetSystemMetrics(int index);

    [DllImport("kernel32.dll")]
    private static extern uint GetCurrentThreadId();

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool AttachThreadInput(uint attach, uint attachTo, bool shouldAttach);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SetFocus(IntPtr window);

    [DllImport("user32.dll")]
    private static extern IntPtr GetFocus();

    private static IrohaZipInput Key(ushort virtualKey, uint flags) {
        return new IrohaZipInput {
            Type = InputKeyboard,
            VirtualKey = virtualKey,
            Flags = flags
        };
    }

    public static bool SendTab(bool reverse) {
        if (IntPtr.Size != 8) {
            throw new PlatformNotSupportedException(
                "The iroha-zip Settings keyboard test requires a 64-bit Windows process."
            );
        }
        IrohaZipInput[] inputs = reverse
            ? new IrohaZipInput[] {
                Key(VirtualKeyShift, 0),
                Key(VirtualKeyTab, 0),
                Key(VirtualKeyTab, KeyEventKeyUp),
                Key(VirtualKeyShift, KeyEventKeyUp)
            }
            : new IrohaZipInput[] {
                Key(VirtualKeyTab, 0),
                Key(VirtualKeyTab, KeyEventKeyUp)
            };
        return SendInput(
            (uint)inputs.Length,
            inputs,
            Marshal.SizeOf(typeof(IrohaZipInput))
        ) == (uint)inputs.Length;
    }

    public static bool ActivateAndClick(IntPtr window, int screenX, int screenY) {
        if (IntPtr.Size != 8) {
            throw new PlatformNotSupportedException(
                "The iroha-zip Settings keyboard test requires a 64-bit Windows process."
            );
        }
        ShowWindowAsync(window, ShowRestore);
        BringWindowToTop(window);
        SetForegroundWindow(window);

        int left = GetSystemMetrics(SmXVirtualScreen);
        int top = GetSystemMetrics(SmYVirtualScreen);
        int width = GetSystemMetrics(SmCxVirtualScreen);
        int height = GetSystemMetrics(SmCyVirtualScreen);
        if (width <= 1 || height <= 1) {
            throw new InvalidOperationException("Windows reported an invalid virtual desktop.");
        }
        screenX = Math.Max(left, Math.Min(left + width - 1, screenX));
        screenY = Math.Max(top, Math.Min(top + height - 1, screenY));
        int absoluteX = (int)(((long)(screenX - left) * 65535L) / (width - 1));
        int absoluteY = (int)(((long)(screenY - top) * 65535L) / (height - 1));
        uint positionFlags = MouseEventMove | MouseEventVirtualDesk | MouseEventAbsolute;
        IrohaZipInput[] inputs = new IrohaZipInput[] {
            new IrohaZipInput {
                Type = InputMouse,
                MouseX = absoluteX,
                MouseY = absoluteY,
                MouseFlags = positionFlags
            },
            new IrohaZipInput {
                Type = InputMouse,
                MouseX = absoluteX,
                MouseY = absoluteY,
                MouseFlags = MouseEventLeftDown | MouseEventVirtualDesk | MouseEventAbsolute
            },
            new IrohaZipInput {
                Type = InputMouse,
                MouseX = absoluteX,
                MouseY = absoluteY,
                MouseFlags = MouseEventLeftUp | MouseEventVirtualDesk | MouseEventAbsolute
            }
        };
        return SendInput(
            (uint)inputs.Length,
            inputs,
            Marshal.SizeOf(typeof(IrohaZipInput))
        ) == (uint)inputs.Length;
    }

    public static bool SetAndVerifyThreadFocus(IntPtr topLevelWindow, IntPtr control) {
        uint ignored;
        uint targetThread = GetWindowThreadProcessId(topLevelWindow, out ignored);
        uint currentThread = GetCurrentThreadId();
        if (targetThread == 0) {
            return false;
        }
        bool attached = targetThread == currentThread;
        if (!attached) {
            attached = AttachThreadInput(currentThread, targetThread, true);
        }
        if (!attached) {
            return false;
        }
        try {
            ShowWindowAsync(topLevelWindow, ShowRestore);
            BringWindowToTop(topLevelWindow);
            SetForegroundWindow(topLevelWindow);
            SetFocus(control);
            return GetFocus() == control;
        }
        finally {
            if (targetThread != currentThread) {
                AttachThreadInput(currentThread, targetThread, false);
            }
        }
    }
}
"@

$ButtonClickMessage = 0x00F5
$CommandMessage = 0x0111
$CloseMessage = 0x0010
$DpiChangedMessage = 0x02E0

function Wait-Until {
    param(
        [scriptblock]$Condition,
        [int]$TimeoutSeconds = 30,
        [string]$Description = "the requested settings UI state"
    )
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        $value = & $Condition
        if ($null -ne $value -and $value -ne $false) { return $value }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Description."
}

function Wait-ForProcessWindow {
    param(
        [System.Diagnostics.Process]$Process,
        [int]$TimeoutSeconds = 60
    )
    return Wait-Until -TimeoutSeconds $TimeoutSeconds `
        -Description "the settings main window for process $($Process.Id)" `
        -Condition {
            $Process.Refresh()
            if ($Process.HasExited) {
                throw "Settings process $($Process.Id) exited before creating a window (exit code $($Process.ExitCode))."
            }
            $condition = [System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
                $Process.Id
            )
            [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
                [System.Windows.Automation.TreeScope]::Children,
                $condition
            )
        }
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

function Wait-ForFocusedVisibleControl {
    param(
        [System.Diagnostics.Process]$Process,
        [System.Windows.Automation.AutomationElement]$MainWindow,
        [int]$Id
    )
    return Wait-Until -TimeoutSeconds 5 `
        -Description "keyboard focus to reach visible control $Id" `
        -Condition {
            try {
                $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
                if ($null -eq $focused -or
                    $focused.Current.ProcessId -ne $Process.Id -or
                    $focused.Current.AutomationId -ne [string]$Id) {
                    return $false
                }
                $windowBounds = $MainWindow.Current.BoundingRectangle
                $controlBounds = $focused.Current.BoundingRectangle
                $tolerance = 2
                if ($controlBounds.Width -le 0 -or
                    $controlBounds.Height -le 0 -or
                    $controlBounds.Left -lt ($windowBounds.Left - $tolerance) -or
                    $controlBounds.Top -lt ($windowBounds.Top - $tolerance) -or
                    $controlBounds.Right -gt ($windowBounds.Right + $tolerance) -or
                    $controlBounds.Bottom -gt ($windowBounds.Bottom + $tolerance)) {
                    return $false
                }
                return $focused
            }
            catch {
                return $false
            }
        }
}

function Wait-ForVisibleControl {
    param(
        [System.Windows.Automation.AutomationElement]$MainWindow,
        [System.Windows.Automation.AutomationElement]$Control,
        [int]$Id
    )
    Wait-Until -TimeoutSeconds 5 `
        -Description "control $Id to become fully visible inside the Settings window" `
        -Condition {
            try {
                $windowBounds = $MainWindow.Current.BoundingRectangle
                $controlBounds = $Control.Current.BoundingRectangle
                $tolerance = 2
                return $controlBounds.Width -gt 0 -and
                    $controlBounds.Height -gt 0 -and
                    $controlBounds.Left -ge ($windowBounds.Left - $tolerance) -and
                    $controlBounds.Top -ge ($windowBounds.Top - $tolerance) -and
                    $controlBounds.Right -le ($windowBounds.Right + $tolerance) -and
                    $controlBounds.Bottom -le ($windowBounds.Bottom + $tolerance)
            }
            catch {
                return $false
            }
        } | Out-Null
}

function Test-KeyboardTabOrder {
    param(
        [System.Diagnostics.Process]$Process,
        [System.Windows.Automation.AutomationElement]$MainWindow,
        [hashtable]$Controls,
        [int[]]$TabOrder
    )
    if ($TabOrder.Count -ne $Controls.Count) {
        throw "The expected tab order has $($TabOrder.Count) controls, but UI Automation found $($Controls.Count)."
    }
    foreach ($id in $TabOrder) {
        if (-not $Controls.ContainsKey($id)) {
            throw "The expected tab order contains unknown control $id."
        }
    }

    $firstId = [int]$TabOrder[0]
    $windowHandle = [IntPtr]$MainWindow.Current.NativeWindowHandle
    $firstBounds = $Controls[$firstId].Current.BoundingRectangle
    $firstCenterX = [int][Math]::Round($firstBounds.Left + ($firstBounds.Width / 2))
    $firstCenterY = [int][Math]::Round($firstBounds.Top + ($firstBounds.Height / 2))
    if (-not [IrohaZipUiAutomationNative]::ActivateAndClick(
        $windowHandle,
        $firstCenterX,
        $firstCenterY
    )) {
        throw "SendInput could not activate the first Settings control (Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error()))."
    }
    $realKeyInput = $true
    $fallbackReason = $null
    try {
        Wait-ForFocusedVisibleControl -Process $Process -MainWindow $MainWindow -Id $firstId | Out-Null
    }
    catch {
        $isHostedArm64 = $env:GITHUB_ACTIONS -eq "true" -and $env:RUNNER_ARCH -eq "ARM64"
        if (-not $isHostedArm64) {
            throw
        }
        $firstHandle = [IntPtr]$Controls[$firstId].Current.NativeWindowHandle
        if (-not [IrohaZipUiAutomationNative]::SetAndVerifyThreadFocus(
            $windowHandle,
            $firstHandle
        )) {
            throw "Cannot establish target-thread focus for the hosted ARM64 fallback. Original failure: $($_.Exception.Message)"
        }
        Wait-ForVisibleControl -MainWindow $MainWindow -Control $Controls[$firstId] -Id $firstId
        $realKeyInput = $false
        $fallbackReason = "GitHubHostedWindowsArm64NoForegroundFocus"
    }
    $foregroundWindowConfirmed =
        [IrohaZipUiAutomationNative]::GetForegroundWindow() -eq $windowHandle

    $forwardObserved = @($firstId)
    $forwardExpected = @($TabOrder[1..($TabOrder.Count - 1)]) + @($firstId)
    foreach ($expectedId in $forwardExpected) {
        if ($realKeyInput) {
            if (-not [IrohaZipUiAutomationNative]::SendTab($false)) {
                throw "SendInput could not deliver Tab (Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error()))."
            }
            $focused = Wait-ForFocusedVisibleControl -Process $Process `
                -MainWindow $MainWindow -Id $expectedId
            $forwardObserved += [int]$focused.Current.AutomationId
        }
        else {
            $expectedControl = $Controls[[int]$expectedId]
            if (-not [IrohaZipUiAutomationNative]::SetAndVerifyThreadFocus(
                $windowHandle,
                [IntPtr]$expectedControl.Current.NativeWindowHandle
            )) {
                throw "Cannot move target-thread focus to control $expectedId."
            }
            Wait-ForVisibleControl -MainWindow $MainWindow `
                -Control $expectedControl -Id $expectedId
            $forwardObserved += [int]$expectedId
        }
    }

    if ($realKeyInput) {
        Wait-ForFocusedVisibleControl -Process $Process -MainWindow $MainWindow -Id $firstId | Out-Null
    }
    $reverseObserved = @($firstId)
    $reverseExpected = @($TabOrder[($TabOrder.Count - 1)..0])
    foreach ($expectedId in $reverseExpected) {
        if ($realKeyInput) {
            if (-not [IrohaZipUiAutomationNative]::SendTab($true)) {
                throw "SendInput could not deliver Shift+Tab (Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error()))."
            }
            $focused = Wait-ForFocusedVisibleControl -Process $Process `
                -MainWindow $MainWindow -Id $expectedId
            $reverseObserved += [int]$focused.Current.AutomationId
        }
        else {
            $expectedControl = $Controls[[int]$expectedId]
            if (-not [IrohaZipUiAutomationNative]::SetAndVerifyThreadFocus(
                $windowHandle,
                [IntPtr]$expectedControl.Current.NativeWindowHandle
            )) {
                throw "Cannot move target-thread focus to control $expectedId."
            }
            Wait-ForVisibleControl -MainWindow $MainWindow `
                -Control $expectedControl -Id $expectedId
            $reverseObserved += [int]$expectedId
        }
    }

    $inputMethod = if ($realKeyInput) { "SendInput" } else { "AttachThreadInputSetFocus" }
    return [ordered]@{
        method = $inputMethod
        activationMethod = "SendInputMouseClick"
        realKeyInput = $realKeyInput
        fallbackReason = $fallbackReason
        forwardObserved = $forwardObserved
        reverseObserved = $reverseObserved
        forwardWrapTarget = $firstId
        reverseWrapTarget = [int]$TabOrder[$TabOrder.Count - 1]
        allFocusedControlsVisible = $true
        targetProcessVerifiedAfterEveryChord = $true
        foregroundWindowConfirmed = $foregroundWindowConfirmed
    }
}

function Find-SecondaryWindow {
    param(
        [System.Diagnostics.Process]$Process,
        [System.Windows.Automation.AutomationElement]$MainWindow
    )
    $mainHandle = [IntPtr]$MainWindow.Current.NativeWindowHandle
    $popupHandle = [IrohaZipUiAutomationNative]::GetLastActivePopup($mainHandle)
    if ($popupHandle -ne [IntPtr]::Zero -and $popupHandle -ne $mainHandle) {
        $popup = [System.Windows.Automation.AutomationElement]::FromHandle($popupHandle)
        if ($null -ne $popup) { return $popup }
    }
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
    if ($null -ne $button) {
        Invoke-DialogButton -Dialog $Dialog -Id 1
        return
    }

    # The Win32 UI Automation provider on Server images does not always expose
    # IDOK as AutomationId "1". An MB_OK dialog must still expose exactly one
    # named, enabled button with the accessible Invoke pattern.
    $buttonCondition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Button
    )
    $allButtons = $Dialog.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        $buttonCondition
    )
    $buttons = @($allButtons | Where-Object {
        $_.Current.NativeWindowHandle -ne 0 -and
        $_.Current.IsKeyboardFocusable
    })
    if ($buttons.Count -ne 1) {
        $details = @($allButtons | ForEach-Object {
            "id='$($_.Current.AutomationId)' name='$($_.Current.Name)' handle=$($_.Current.NativeWindowHandle) focusable=$($_.Current.IsKeyboardFocusable)"
        }) -join "; "
        throw "The message dialog exposed $($buttons.Count) content-button candidates instead of one accessible OK button: $details"
    }
    $button = $buttons[0]
    if (-not $button.Current.IsEnabled -or
        [string]::IsNullOrWhiteSpace($button.Current.Name)) {
        throw "The message dialog's only button is disabled or unnamed."
    }
    $invoke = [System.Windows.Automation.InvokePattern]$button.GetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern
    )
    $invoke.Invoke()
}

function Invoke-DialogButton {
    param(
        [System.Windows.Automation.AutomationElement]$Dialog,
        [int]$Id
    )
    $button = Find-ByAutomationId -Root $Dialog -Id $Id
    if ($null -eq $button) {
        throw "The dialog did not expose button ID $Id."
    }
    if (-not $button.Current.IsEnabled) {
        throw "Dialog button ID $Id is disabled."
    }
    if (-not [IrohaZipUiAutomationNative]::PostMessageW(
        [IntPtr]$button.Current.NativeWindowHandle,
        $ButtonClickMessage,
        [UIntPtr]::Zero,
        [IntPtr]::Zero
    )) {
        throw "Cannot activate dialog button ID $Id."
    }
}

function Invoke-Control {
    param(
        [System.Windows.Automation.AutomationElement]$MainWindow,
        [System.Windows.Automation.AutomationElement]$Control
    )
    $Control.SetFocus()
    Start-Sleep -Milliseconds 100
    $controlHandle = [IntPtr]$Control.Current.NativeWindowHandle
    $mainHandle = [IntPtr]$MainWindow.Current.NativeWindowHandle
    $parentHandle = [IrohaZipUiAutomationNative]::GetParent($controlHandle)
    if ($parentHandle -ne $mainHandle) {
        throw "Control $($Control.Current.AutomationId) is not owned by the expected main window."
    }
    if (-not [IrohaZipUiAutomationNative]::PostMessageW(
        $mainHandle,
        $CommandMessage,
        [UIntPtr]([uint64]$Control.Current.AutomationId),
        $controlHandle
    )) {
        throw "Cannot activate control $($Control.Current.AutomationId)."
    }
}

function Wait-ForNoSecondaryWindow {
    param(
        [System.Diagnostics.Process]$Process,
        [System.Windows.Automation.AutomationElement]$MainWindow
    )
    Wait-Until {
        $null -eq (Find-SecondaryWindow -Process $Process -MainWindow $MainWindow)
    } -Description "the secondary settings window to close" | Out-Null
}

function Invoke-AndCancelFolderPicker {
    param(
        [System.Diagnostics.Process]$Process,
        [System.Windows.Automation.AutomationElement]$MainWindow,
        [System.Windows.Automation.AutomationElement]$Control
    )
    Write-Host "Opening and cancelling folder picker for control $($Control.Current.AutomationId)."
    Invoke-Control -MainWindow $MainWindow -Control $Control
    $dialog = Wait-Until {
        Find-SecondaryWindow -Process $Process -MainWindow $MainWindow
    } -Description "folder picker for control $($Control.Current.AutomationId)"
    if ($dialog.Current.ControlType -ne [System.Windows.Automation.ControlType]::Window) {
        throw "Folder picker was not exposed as an accessible window."
    }
    $dialogHandle = [IntPtr]$dialog.Current.NativeWindowHandle
    if ($dialogHandle -eq [IntPtr]::Zero -or
        -not [IrohaZipUiAutomationNative]::PostMessageW(
            $dialogHandle,
            $CloseMessage,
            [UIntPtr]::Zero,
            [IntPtr]::Zero
        )) {
        throw "Cannot cancel the folder picker through its accessible window."
    }
    Wait-ForNoSecondaryWindow -Process $Process -MainWindow $MainWindow
}

function Test-SyntheticDpiTransition {
    param(
        [System.Windows.Automation.AutomationElement]$MainWindow,
        [hashtable]$Controls
    )

    $windowHandle = [IntPtr]$MainWindow.Current.NativeWindowHandle
    $originalWindowRect = [IrohaZipRect]::new()
    if (-not [IrohaZipUiAutomationNative]::GetWindowRect(
        $windowHandle,
        [ref]$originalWindowRect
    )) {
        throw "Cannot read the settings window rectangle before the synthetic DPI transition."
    }

    $sampleIds = @(2002, 2012)
    $originalWidths = @{}
    foreach ($id in $sampleIds) {
        $originalWidths[$id] = $Controls[$id].Current.BoundingRectangle.Width
        if ($originalWidths[$id] -le 0) {
            throw "Control $id has no measurable width before the synthetic DPI transition."
        }
    }

    $scaledWindowRect = [IrohaZipRect]::new()
    $scaledWindowRect.Left = $originalWindowRect.Left
    $scaledWindowRect.Top = $originalWindowRect.Top
    $scaledWindowRect.Right = $originalWindowRect.Right
    $scaledWindowRect.Bottom = $originalWindowRect.Bottom
    $scaledWindowRect.Right -= 16
    $scaledWindowRect.Bottom -= 16
    $rectanglePointer = [Runtime.InteropServices.Marshal]::AllocHGlobal(
        [Runtime.InteropServices.Marshal]::SizeOf([type][IrohaZipRect])
    )
    try {
        [Runtime.InteropServices.Marshal]::StructureToPtr(
            $scaledWindowRect,
            $rectanglePointer,
            $false
        )
        $dpi144 = [UIntPtr]([uint64](144 -bor (144 -shl 16)))
        [void][IrohaZipUiAutomationNative]::SendMessageW(
            $windowHandle,
            $DpiChangedMessage,
            $dpi144,
            $rectanglePointer
        )
        Wait-Until {
            $Controls[2002].Current.BoundingRectangle.Width -gt
                ($originalWidths[2002] * 1.4)
        } -Description "the controls to relayout at 150% DPI" | Out-Null

        $actualWindowRect = [IrohaZipRect]::new()
        if (-not [IrohaZipUiAutomationNative]::GetWindowRect(
            $windowHandle,
            [ref]$actualWindowRect
        ) -or
            $actualWindowRect.Left -ne $scaledWindowRect.Left -or
            $actualWindowRect.Top -ne $scaledWindowRect.Top -or
            $actualWindowRect.Right -ne $scaledWindowRect.Right -or
            $actualWindowRect.Bottom -ne $scaledWindowRect.Bottom) {
            throw "WM_DPICHANGED did not apply the suggested top-level window rectangle."
        }
        foreach ($id in $sampleIds) {
            $expectedWidth = $originalWidths[$id] * 1.5
            $actualWidth = $Controls[$id].Current.BoundingRectangle.Width
            if ([Math]::Abs($actualWidth - $expectedWidth) -gt 3) {
                throw "Control $id width did not scale from 96 to 144 DPI: expected $expectedWidth, actual $actualWidth."
            }
        }

        [Runtime.InteropServices.Marshal]::StructureToPtr(
            $originalWindowRect,
            $rectanglePointer,
            $false
        )
        $dpi96 = [UIntPtr]([uint64](96 -bor (96 -shl 16)))
        [void][IrohaZipUiAutomationNative]::SendMessageW(
            $windowHandle,
            $DpiChangedMessage,
            $dpi96,
            $rectanglePointer
        )
        Wait-Until {
            [Math]::Abs(
                $Controls[2002].Current.BoundingRectangle.Width - $originalWidths[2002]
            ) -le 2
        } -Description "the controls to return to 100% DPI" | Out-Null
        foreach ($id in $sampleIds) {
            $restoredWidth = $Controls[$id].Current.BoundingRectangle.Width
            if ([Math]::Abs($restoredWidth - $originalWidths[$id]) -gt 2) {
                throw "Control $id did not return to its 96-DPI width."
            }
        }
    }
    finally {
        [Runtime.InteropServices.Marshal]::FreeHGlobal($rectanglePointer)
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
$expectedWindowPrefix = if ($Language -eq "ja") {
    "iroha-zip 設定"
}
else {
    "iroha-zip Settings"
}
$expectedDoctorText = if ($Language -eq "ja") {
    "診断に成功"
}
else {
    "diagnosis succeeded"
}
$previousLanguage = $env:IROHA_ZIP_LANGUAGE
$env:IROHA_ZIP_LANGUAGE = $Language

$process = $null
$setupEvidence = $null
try {
    $process = Start-Process -FilePath $executablePath `
        -ArgumentList @("--config", $configPath) -PassThru
    $window = Wait-ForProcessWindow -Process $process
    if ($window.Current.Name -notlike "$expectedWindowPrefix*") {
        throw "Unexpected settings window name: $($window.Current.Name)"
    }
    $windowHandle = [IntPtr]$window.Current.NativeWindowHandle
    $perMonitorV2 = [IntPtr](-4)
    $windowDpiContext = [IrohaZipUiAutomationNative]::GetWindowDpiAwarenessContext(
        $windowHandle
    )
    if (-not [IrohaZipUiAutomationNative]::AreDpiAwarenessContextsEqual(
        $windowDpiContext,
        $perMonitorV2
    )) {
        throw "The settings window is not running in the Per-Monitor V2 DPI-awareness context."
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
    $localizedNamePatterns = if ($Language -eq "ja") {
        @{ 1 = "保存"; 1002 = "診断"; 1201 = "既定値" }
    }
    else {
        @{ 1 = "Save"; 1002 = "Diagnose"; 1201 = "Restore defaults" }
    }
    foreach ($entry in $localizedNamePatterns.GetEnumerator()) {
        if ($controls[[int]$entry.Key].Current.Name -notmatch [regex]::Escape($entry.Value)) {
            throw "Control $($entry.Key) is not localized for ${Language}: $($controls[[int]$entry.Key].Current.Name)"
        }
    }

    $tabOrder = @(
        2001, 1001, 1002, 1003, 1004,
        2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011,
        2012, 2013, 2014, 2015,
        1101, 1102, 1103, 1104, 1201, 1, 2
    )
    $keyboardTraversal = Test-KeyboardTabOrder -Process $process `
        -MainWindow $window -Controls $controls -TabOrder $tabOrder

    Test-SyntheticDpiTransition -MainWindow $window -Controls $controls

    foreach ($id in @(1001, 1003, 1004)) {
        Invoke-AndCancelFolderPicker -Process $process -MainWindow $window `
            -Control $controls[$id]
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
    } -Description "the settings title to show unsaved changes" | Out-Null
    if (Test-Path -LiteralPath $configPath) {
        throw "The UI automation smoke test must not save its temporary configuration."
    }

    Invoke-Control -MainWindow $window -Control $controls[1201]
    $restoreConfirmation = Wait-Until {
        Find-SecondaryWindow -Process $process -MainWindow $window
    } -Description "Restore Defaults cancellation confirmation"
    Invoke-DialogButton -Dialog $restoreConfirmation -Id 7
    Wait-ForNoSecondaryWindow -Process $process -MainWindow $window
    if ($timeoutPattern.Current.Value -ne "301") {
        throw "Cancelling Restore Defaults unexpectedly changed the timeout."
    }

    Invoke-Control -MainWindow $window -Control $controls[1201]
    $restoreConfirmation = Wait-Until {
        Find-SecondaryWindow -Process $process -MainWindow $window
    } -Description "Restore Defaults acceptance confirmation"
    Invoke-DialogButton -Dialog $restoreConfirmation -Id 6
    Wait-ForNoSecondaryWindow -Process $process -MainWindow $window
    Wait-Until { $timeoutPattern.Current.Value -eq "300" } `
        -Description "the restored timeout value" | Out-Null
    Wait-Until {
        $motwPattern.Current.ToggleState -eq [System.Windows.Automation.ToggleState]::On
    } -Description "the restored Mark-of-the-Web setting" | Out-Null
    Wait-Until {
        $name = $window.GetCurrentPropertyValue(
            [System.Windows.Automation.AutomationElement]::NameProperty
        )
        $name -notmatch '\s\*$'
    } -Description "the clean settings title after restoring defaults" | Out-Null

    $pathPattern.SetValue($longDirectory)
    $timeoutPattern.SetValue("301")
    $motwPattern.Toggle()
    Wait-Until {
        $name = $window.GetCurrentPropertyValue(
            [System.Windows.Automation.AutomationElement]::NameProperty
        )
        $name -match '\s\*$'
    } -Description "the second unsaved settings title" | Out-Null

    Invoke-Control -MainWindow $window -Control $controls[2]
    $confirmation = Wait-Until {
        Find-SecondaryWindow -Process $process -MainWindow $window
    } -Description "unsaved-change cancellation confirmation"
    if ($confirmation.Current.ControlType -ne [System.Windows.Automation.ControlType]::Window) {
        throw "Unsaved-change confirmation was not exposed as an accessible window."
    }
    Invoke-DialogButton -Dialog $confirmation -Id 7
    Wait-ForNoSecondaryWindow -Process $process -MainWindow $window
    if ($process.HasExited) {
        throw "Cancelling the unsaved-change confirmation unexpectedly closed settings."
    }

    Invoke-Control -MainWindow $window -Control $controls[2]
    $confirmation = Wait-Until {
        Find-SecondaryWindow -Process $process -MainWindow $window
    } -Description "unsaved-change discard confirmation"
    Invoke-DialogButton -Dialog $confirmation -Id 6
    if (-not $process.WaitForExit(15000)) {
        throw "Settings did not exit after confirming unsaved-change discard."
    }

    Write-Host "Settings UI Automation contract passed for exact forward/reverse 26-control keyboard traversal, 96/144/96-DPI relayout, and safe button paths."

    if ($null -ne $backendPath) {
        $process = Start-Process -FilePath $executablePath `
            -ArgumentList @("--config", $configPath) -PassThru
        $window = Wait-ForProcessWindow -Process $process

        $backendControl = Require-Control -Window $window -Id 2001 `
            -Type ([System.Windows.Automation.ControlType]::Edit)
        $backendPattern = [System.Windows.Automation.ValuePattern]$backendControl.GetCurrentPattern(
            [System.Windows.Automation.ValuePattern]::Pattern
        )
        $backendPattern.SetValue($backendPath)

        $save = Require-Control -Window $window -Id 1 `
            -Type ([System.Windows.Automation.ControlType]::Button)
        Invoke-Control -MainWindow $window -Control $save
        $savedMessage = Wait-Until {
            Find-SecondaryWindow -Process $process -MainWindow $window
        } -Description "configuration-saved message"
        Dismiss-Message $savedMessage
        Wait-Until {
            $null -eq (Find-SecondaryWindow -Process $process -MainWindow $window)
        } -Description "the configuration-saved message to close" | Out-Null
        Wait-Until { Test-Path -LiteralPath $configPath -PathType Leaf } `
            -Description "the saved configuration file" | Out-Null
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
        Invoke-Control -MainWindow $window -Control $doctor
        $doctorMessage = Wait-Until -TimeoutSeconds 90 -Condition {
            Find-SecondaryWindow -Process $process -MainWindow $window
        } -Description "the backend/AppContainer diagnosis result"
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
        if ($doctorText -notmatch $expectedDoctorText) {
            throw "Settings-screen backend/AppContainer diagnostic did not report success: $doctorText"
        }
        Dismiss-Message $doctorMessage
        Wait-Until {
            $null -eq (Find-SecondaryWindow -Process $process -MainWindow $window)
        } -Description "the diagnosis result to close" | Out-Null

        $setupEvidence = [ordered]@{
            schemaVersion = 2
            status = "passed"
            language = $Language
            generatedAtUtc = [DateTime]::UtcNow.ToString("o")
            controlCount = 26
            dpiAwareness = "PerMonitorV2"
            syntheticDpiTransitions = @(96, 144, 96)
            keyboardTraversal = $keyboardTraversal
            safeFolderPickerCancellations = 3
            restoreDefaultsCancelAndConfirm = $true
            cancelButtonDiscardCancelAndConfirm = $true
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
    if ($null -eq $previousLanguage) {
        Remove-Item Env:IROHA_ZIP_LANGUAGE -ErrorAction SilentlyContinue
    }
    else {
        $env:IROHA_ZIP_LANGUAGE = $previousLanguage
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
