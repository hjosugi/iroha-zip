# Settings accessibility and UI automation

Updated: 2026-08-15

This document records the implemented UX-001 contract and the evidence that is still required on
real Windows systems. It does not claim screen-reader or high-DPI certification.

## Layout and keyboard contract

`iroha-zip-settings.exe` embeds an `asInvoker`, Per-Monitor V2 Windows manifest, with Per-Monitor
and legacy `true/pm` fallbacks. All control coordinates remain expressed at the exact 96-DPI
logical baseline. On `WM_DPICHANGED`, the application applies Windows' suggested top-level
rectangle, lays out every child HWND again from that baseline, rescales the transient scroll
position, recreates the Windows message font for the new DPI, and recalculates both scrollbars.
The initial window is capped to the current display, is resizable and maximizable, and exposes
horizontal and vertical scrollbars when the scaled content is larger than the client area. Moving
focus with the keyboard automatically scrolls the focused control into view.

The platform-neutral scaling tests cover 96, 120, 144, 192, and 288 DPI (100–300%). This proves the
integer conversion contract, not visual fit on a particular monitor. Windows UI Automation also
requires the live top-level HWND to report the Per-Monitor V2 awareness context, sends a bounded
synthetic 96→144→96 `WM_DPICHANGED` sequence, verifies Windows' suggested rectangle, and requires
two representative controls to scale to 150% and return to their original widths. A synthetic
message does not move a window between physical displays, so real mixed-monitor visual fit remains
part of the real-Windows matrix.

Every editable setting, combo box, checkbox, and action button has:

- a stable, non-zero Win32 control ID;
- a visible Japanese or English label with a unique access key;
- standard Win32 keyboard behavior through `IsDialogMessageW`;
- Tab/Shift+Tab focus, Enter-to-save, and Escape-to-close behavior;
- an unsaved-change marker and a close confirmation when edits have not been saved.

The 15 setting IDs occupy 2001–2015. The 11 action IDs include 1001–1004, 1101–1104, 1201, `IDOK`
(1), and `IDCANCEL` (2). A platform-neutral `SettingsAction` mapping is exhaustively tested so every
action ID has exactly one dispatch target. A second platform-neutral contract fixes the Win32
creation/Tab order of all 26 IDs and requires the PowerShell automation literal to match it exactly.

The settings application localizes labels, combo-box choices, status text, validation, folder-picker titles, confirmations, success messages, and operational errors. It follows the Windows user UI language: Japanese selects Japanese, while other UI languages use English. `IROHA_ZIP_LANGUAGE=ja` or `IROHA_ZIP_LANGUAGE=en` is an explicit process-local override for automation and support reproduction; it is not persisted in the configuration file.

## UI Automation smoke test

Windows CI builds the release settings executable and runs
[test-settings-ui.ps1](../scripts/test-settings-ui.ps1) with the native .NET UI Automation API.
The fast Windows job runs the complete contract once in Japanese and once in English. The script creates a temporary configuration path and a long, non-ASCII backend path, then checks
all 26 controls for:

- expected AutomationId and control type;
- enabled and keyboard-focusable state;
- non-empty accessible name, access key, and bounds;
- exact forward Tab and reverse Shift+Tab cycles through all 26 controls, including wraparound;
- successful focus and full visibility after automatic scrolling, including controls initially
  outside the viewport.

The test first requests foreground activation for the Settings HWND, focuses the first edit, and
injects real `VK_TAB`/`VK_SHIFT` keyboard input through Win32 `SendInput`. Hosted Windows runners do
not always report that HWND from `GetForegroundWindow()`, so the evidence records that observation
without treating it as the input-routing oracle. After every key chord, UI Automation must report
the expected process and AutomationId and a non-empty rectangle fully inside the top-level window.
The forward cycle must return from Cancel to the first edit; the reverse cycle
must reach Cancel from the first edit and return through the exact opposite order. This exercises
the production `IsDialogMessageW` path instead of treating independent UIA `SetFocus` calls as Tab
evidence. It remains automated runner evidence rather than a human assistive-technology test.

Before mutating the form, the same process-level test checks the effective Per-Monitor V2 context
and the synthetic 96→144→96 relayout contract described above. This detects a missing embedded
manifest, a handler that ignores the suggested rectangle, one-time-only child geometry, and
round-trip scaling drift without claiming physical-monitor evidence.

The exact-main [Actions run 31868019031](https://github.com/hjosugi/iroha-zip/actions/runs/31868019031)
at commit `5cbc6c27fb67466369b20180a9c5aa2fdd3f6713` produced four independently checked Settings
reports: English on Server 2022 and Server 2025, plus Japanese and English on native Windows 11
ARM64. Every report records 26 controls, effective `PerMonitorV2`, the exact 96→144→96 synthetic
transition, backend diagnosis success, and complete temporary-root removal. New schema-v2 reports
also record the complete observed forward/reverse keyboard cycles, wrap targets, input method, and
focused-control visibility result.

It edits a path and numeric value through `ValuePattern`, toggles a checkbox through
`TogglePattern`, verifies the dirty-title contract without writing the temporary configuration,
and opens/cancels the backend Browse, arbitrary-bundle Import, and MSYS2 Import folder pickers. It
then invokes Restore Defaults twice, requires accessible No/Yes confirmation paths, verifies both
preserved edits and restored defaults, makes the form dirty again, and invokes the native Cancel
button twice to verify both preservation and confirmed discard through the accessible dialog.

When supplied a verified backend and evidence path by the dedicated Windows E2E job, a second disposable English settings process also saves that backend path through the native Save button, dismisses the success dialog, invokes the settings-screen diagnosis, requires the real backend/AppContainer diagnostic success dialog, closes from a clean state, hashes the saved configuration and executable, records the exercised language, and writes a JSON report after removing the temporary tree. This does not exercise backend replacement, file associations, Default Apps, or folder-picker side effects.

The smoke test cancels import before a source is selected, and deliberately does not invoke actual
backend replacement, Default Apps, or the configuration-folder launch because those actions mutate
or open external Windows state. Their ID-to-handler dispatch is covered by the exhaustive
platform-neutral test. Their side effects and rollback must be exercised only on a disposable
Windows worker.

The fast disposable Windows job separately exercises association registration as real per-user
external state. It refuses to run over any pre-existing iroha-zip-owned registry tree, stages the
shell executable under a path containing spaces and Japanese text, and places unique unrelated
sentinel values in all 18 shared `OpenWithProgids` keys and `RegisteredApplications`. It snapshots
each protected `UserChoice`, registers twice to prove idempotence, and requires exact quoted
commands, icon paths, supported types, capabilities, and candidate ProgID values. Unregistration
must remove only iroha-zip-owned keys/values while retaining every sentinel and the exact
`UserChoice` key existence, subkey names, value names, kinds, and data. Final cleanup removes only test-created state. The contract passed in
[Actions run 31768309835](https://github.com/hjosugi/iroha-zip/actions/runs/31768309835).

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
`WAIT_ABANDONED` after an owning thread exits without release. The Windows tests and both fixed
Server settings save/diagnosis paths passed in [Actions run 31768440143](https://github.com/hjosugi/iroha-zip/actions/runs/31768440143).

The replacement transaction writes and flushes a unique temporary file, renames the prior file to
a unique backup, and only then moves the new file into place. A deterministic injected-failure test
requires a failed final rename to restore the byte-identical prior file and remove both staging
artifacts. A second test forces both replacement and restoration to fail; the returned error then
includes the exact recovery-backup path, and that backup must retain the prior bytes instead of
being silently deleted.

Backend replacement has an independent disposable-Windows transaction check. After constructing and
validating a complete staged bundle, CI injects failure immediately after the existing backend is
renamed to its unique backup. The script must restore the byte-identical prior tree, remove every
backend stage/backup artifact, and then complete a normal import successfully in the same job. The
test-only environment hook is honored only when `CI=true` and can only force failure; it cannot skip
source, manifest, evidence, or destination validation.

## Real-Windows evidence still required

UX-001 stays open until disposable Windows 10 and 11 systems record:

1. human visual-fit and keyboard-only traversal at 100, 125, 150, 200, and 300% scaling beyond the
   automated exact forward/reverse keyboard cycles;
2. low-resolution viewport scrolling and repeated movement between physical mixed-DPI monitors;
3. Narrator and at least one independent screen reader;
4. Japanese and English Windows with long and non-ASCII paths;
5. every external-state action, confirmation, progress indication, failure, and rollback;
6. a passing Windows CI execution of the independent-process, mutex timeout/abandonment, and both
   native UI Automation phases for the current branch.

Primary implementation references:

- [Setting the default DPI awareness for a process](https://learn.microsoft.com/en-us/windows/win32/hidpi/setting-the-default-dpi-awareness-for-a-process)
- [`WM_DPICHANGED`](https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-dpichanged)
- [High-DPI desktop application development](https://learn.microsoft.com/en-us/windows/win32/hidpi/high-dpi-desktop-application-development-on-windows)
- [`GetDpiForWindow`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdpiforwindow)
- [UI Automation overview](https://learn.microsoft.com/en-us/dotnet/framework/ui-automation/ui-automation-overview)
- [Supporting UI Automation control types](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-supportinguiautocontroltypes)
