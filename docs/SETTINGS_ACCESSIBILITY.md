# Settings accessibility and UI automation

Updated: 2026-08-09

This document records the implemented UX-001 contract and the evidence that is still required on
real Windows systems. It does not claim screen-reader or high-DPI certification.

## Layout and keyboard contract

`iroha-zip-settings.exe` embeds an `asInvoker`, System-DPI-aware Windows manifest. All coordinates
are expressed at the 96-DPI logical baseline and scaled from the window DPI. The initial window is
capped to the current display, is resizable and maximizable, and exposes horizontal and vertical
scrollbars when the scaled content is larger than the client area. Moving focus with the keyboard
automatically scrolls the focused control into view.

The platform-neutral scaling tests cover 96, 120, 144, 192, and 288 DPI (100–300%). This proves the
integer conversion contract, not visual fit on a particular monitor. The current executable uses
System DPI awareness rather than Per-Monitor V2; moving a running window between monitors with
different scale factors remains part of the real-Windows matrix.

Every editable setting, combo box, checkbox, and action button has:

- a stable, non-zero Win32 control ID;
- a visible Japanese label with a unique access key;
- standard Win32 keyboard behavior through `IsDialogMessageW`;
- Tab/Shift+Tab focus, Enter-to-save, and Escape-to-close behavior;
- an unsaved-change marker and a close confirmation when edits have not been saved.

The 15 setting IDs occupy 2001–2015. The 11 action IDs include 1001–1004, 1101–1104, 1201, `IDOK`
(1), and `IDCANCEL` (2). A platform-neutral `SettingsAction` mapping is exhaustively tested so every
action ID has exactly one dispatch target.

## UI Automation smoke test

Windows CI builds the release settings executable and runs
[test-settings-ui.ps1](../scripts/test-settings-ui.ps1) with the native .NET UI Automation API.
The script creates a temporary configuration path and a long, non-ASCII backend path, then checks
all 26 controls for:

- expected AutomationId and control type;
- enabled and keyboard-focusable state;
- non-empty accessible name, access key, and bounds;
- successful focus traversal, including controls initially outside the viewport.

It edits a path and numeric value through `ValuePattern`, toggles a checkbox through
`TogglePattern`, verifies the dirty-title contract without writing the temporary configuration,
closes through `WindowPattern`, and requires the unsaved-change confirmation to be exposed as an
accessible window.

The smoke test deliberately does not invoke backend import, association, Default Apps, or folder
launch buttons because those actions mutate or open external Windows state. Their ID-to-handler
dispatch is covered by the exhaustive platform-neutral test. Their side effects and rollback must
be exercised only on a disposable Windows worker.

## Concurrent configuration saves

Configuration replacement remains validate-before-write and rollback-safe. Saves now acquire a
process-wide guard on non-Windows systems and a named `Local\iroha-zip.ConfigSave.v1` mutex on
Windows, with a 30-second fail-closed timeout. This serializes settings, CLI, and import saves in
the same Windows session before any temporary or backup file is created.

A deterministic two-thread regression test starts both saves together and verifies that the final
file is exactly one complete configuration and that no staging artifact remains. A two-process
Windows test, abandoned-owner test, and timeout test remain open.

## Real-Windows evidence still required

UX-001 stays open until disposable Windows 10 and 11 systems record:

1. visual fit and keyboard-only traversal at 100, 125, 150, 200, and 300% scaling;
2. low-resolution viewport scrolling and mixed-DPI monitor movement;
3. Narrator and at least one independent screen reader;
4. Japanese and English Windows with long and non-ASCII paths;
5. every external-state action, confirmation, progress indication, failure, and rollback;
6. concurrent saves from two independent processes;
7. Windows CI execution of the native UI Automation smoke test.

Primary implementation references:

- [Setting the default DPI awareness for a process](https://learn.microsoft.com/en-us/windows/win32/hidpi/setting-the-default-dpi-awareness-for-a-process)
- [`GetDpiForWindow`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdpiforwindow)
- [UI Automation overview](https://learn.microsoft.com/en-us/dotnet/framework/ui-automation/ui-automation-overview)
- [Supporting UI Automation control types](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-supportinguiautocontroltypes)
