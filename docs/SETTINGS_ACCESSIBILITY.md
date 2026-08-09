# Settings accessibility and UI automation

Updated: 2026-08-10

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
and opens/cancels the backend Browse, arbitrary-bundle Import, and MSYS2 Import folder pickers. It
then invokes Restore Defaults twice, requires accessible No/Yes confirmation paths, verifies both
preserved edits and restored defaults, makes the form dirty again, and invokes the native Cancel
button twice to verify both preservation and confirmed discard through the accessible dialog.

When supplied a verified backend and evidence path by the dedicated Windows E2E job, a second disposable settings process also saves that backend path through the native Save button, dismisses the success dialog, invokes the settings-screen diagnosis, requires the real backend/AppContainer diagnostic success dialog, closes from a clean state, hashes the saved configuration and executable, and writes a JSON report after removing the temporary tree. This does not exercise backend replacement, file associations, Default Apps, or folder-picker side effects.

The smoke test cancels import before a source is selected, and deliberately does not invoke actual
backend replacement, association changes, Default Apps, or the configuration-folder launch because
those actions mutate or open external Windows state. Their ID-to-handler dispatch is covered by the
exhaustive platform-neutral test. Their side effects and rollback must be exercised only on a
disposable Windows worker.

## Concurrent configuration saves

Configuration replacement remains validate-before-write and rollback-safe. Saves now acquire a
process-wide guard on non-Windows systems and a named `Local\iroha-zip.ConfigSave.v1` mutex on
Windows, with a 30-second fail-closed timeout. This serializes settings, CLI, and import saves in
the same Windows session before any temporary or backup file is created. Initial default-file
creation now acquires the same lock before checking for or creating the file, so simultaneous first
runs report exactly one creator rather than exposing a check-then-create race.

Deterministic thread tests start simultaneous default creation and replacement saves, then verify
that the final file is one complete valid configuration and no staging artifact remains. A
Windows-only integration test starts two independent copies of the test executable, releases both
through one file barrier against the same non-ASCII configuration path, and requires one complete
configuration plus zero temporary/backup artifacts. Windows unit tests use isolated mutex names to
require a bounded wait to fail closed, verify a normal acquisition after release, and accept
`WAIT_ABANDONED` after an owning thread exits without release. These Windows-only tests still need
their first passing CI evidence.

The replacement transaction writes and flushes a unique temporary file, renames the prior file to
a unique backup, and only then moves the new file into place. A deterministic injected-failure test
requires a failed final rename to restore the byte-identical prior file and remove both staging
artifacts. A second test forces both replacement and restoration to fail; the returned error then
includes the exact recovery-backup path, and that backup must retain the prior bytes instead of
being silently deleted.

## Real-Windows evidence still required

UX-001 stays open until disposable Windows 10 and 11 systems record:

1. visual fit and keyboard-only traversal at 100, 125, 150, 200, and 300% scaling;
2. low-resolution viewport scrolling and mixed-DPI monitor movement;
3. Narrator and at least one independent screen reader;
4. Japanese and English Windows with long and non-ASCII paths;
5. every external-state action, confirmation, progress indication, failure, and rollback;
6. a passing Windows CI execution of the independent-process, mutex timeout/abandonment, and both
   native UI Automation phases for the current branch.

Primary implementation references:

- [Setting the default DPI awareness for a process](https://learn.microsoft.com/en-us/windows/win32/hidpi/setting-the-default-dpi-awareness-for-a-process)
- [`GetDpiForWindow`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdpiforwindow)
- [UI Automation overview](https://learn.microsoft.com/en-us/dotnet/framework/ui-automation/ui-automation-overview)
- [Supporting UI Automation control types](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-supportinguiautocontroltypes)
