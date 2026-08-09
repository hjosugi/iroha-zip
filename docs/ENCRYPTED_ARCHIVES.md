# Encrypted archive input design

Updated: 2026-08-09

Encrypted archive support is intentionally disabled. This document fixes the SAFE-007 trust boundary before implementation so that compatibility work cannot silently move a password into process arguments, environment variables, logs, crash reports, or persistent configuration.

## Upstream constraints

The libarchive bsdtar manual documents `--passphrase <passphrase>`, but also explicitly warns that this option is insecure. The current bsdtar source assigns that argument directly to its passphrase field and passes it to the archive reader or writer. iroha-zip will not use this option.

When no argument is supplied, bsdtar installs a passphrase callback. In the Windows implementation, that callback obtains standard input/output, requires `GetConsoleMode` to succeed on stdin, disables echo, and reads a line. An ordinary redirected anonymous pipe is not a console handle, so replacing `NUL` with a pipe is not a compatible secure transport for stock Windows bsdtar.

Primary references:

- [libarchive `bsdtar(1)` passphrase warning](https://github.com/libarchive/libarchive/blob/master/tar/bsdtar.1)
- [bsdtar reader installs the passphrase callback](https://github.com/libarchive/libarchive/blob/master/tar/read.c)
- [bsdtar Windows passphrase implementation](https://github.com/libarchive/libarchive/blob/master/libarchive_fe/passphrase.c)
- [Windows `CreatePseudoConsole`](https://learn.microsoft.com/en-us/windows/console/createpseudoconsole)
- [Microsoft pseudoconsole session lifecycle](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session)
- [Anonymous pipe security](https://learn.microsoft.com/en-us/windows/win32/ipc/anonymous-pipe-security-and-access-rights)

## Proposed one-use channel

The compatible design is a hidden pseudoconsole (ConPTY), available on Windows 10 version 1809 and newer. The host creates synchronous input/output pipe pairs, gives only the pseudoconsole-facing ends to `CreatePseudoConsole`, and retains non-inheritable controller ends. The child receives `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`; the plaintext password is never an argument, environment value, file, named object, or configuration field.

The controller writes exactly one UTF-8 password line only after the native UI has returned a confirmed secret. It then zeroizes and drops the write buffer and closes the input controller. A dedicated thread must continuously drain pseudoconsole output until child exit and teardown. Microsoft warns that synchronous single-threaded teardown can deadlock because closing a pseudoconsole may emit a final output frame.

ConPTY changes stdout/stderr into one terminal stream. The controller must therefore:

- cap captured output independently of the archive extraction limits;
- recognize only bounded backend prompts and never copy terminal output into a secret buffer;
- strip or escape terminal control sequences before presenting diagnostics;
- never include user input in diagnostics;
- terminate the Job Object if the prompt count, output limit, or timeout is exceeded;
- join the drain thread before closing the pseudoconsole and pipes.

## Secret lifetime

The password UI must use a password edit control with clipboard/context-menu policy reviewed separately. No password may be persisted in `Config` or represented by a CLI value. A CLI or shell command may request a prompt with a boolean action, but it cannot accept the password text as an option.

The secret type must own a bounded UTF-16 UI buffer and a bounded UTF-8 transport buffer, redact `Debug` and `Display`, avoid `Clone`, and use a zeroization primitive designed not to be optimized away. Every early return, cancellation, spawn failure, timeout, wrong-password result, and panic boundary must drop the buffers. Crash dumps and paging remain residual operating-system risks and must be documented rather than claimed away.

## Fail-closed state machine

```text
NoSecret
  ├─ user cancels ───────────────> Cancelled (no sandbox/process)
  └─ confirmed bounded secret ───> SecretReady
SecretReady
  ├─ ConPTY setup/spawn fails ───> Failed + zeroize + cleanup
  └─ child token verified ───────> AwaitingPrompt
AwaitingPrompt
  ├─ unexpected/multiple prompt ─> terminate + zeroize + cleanup
  ├─ timeout/output overflow ────> terminate + zeroize + cleanup
  └─ expected prompt ────────────> write once + close input + zeroize
ChildRunning
  ├─ nonzero/wrong password ─────> no retry, cleanup, return typed error
  └─ success ────────────────────> existing tree audit/publication path
```

Automatic password retries are forbidden. They multiply secret exposure and make prompt parsing ambiguous. A wrong-password result returns to the caller with no published output; the user may explicitly start a fresh operation and fresh channel.

## Required tests before enablement

Platform-neutral tests:

- secret values are redacted from formatting and error context;
- bounded UTF-16/UTF-8 conversion and empty/oversized rejection;
- cancellation never constructs `ProcessSpec`;
- state transitions allow at most one write and one prompt;
- all failure objects drop owned secret buffers.

Disposable Windows tests with a pinned backend and redistributable corpus:

- correct and wrong passwords for ZipCrypto and AES ZIP variants actually supported by the pinned libarchive build;
- password characters outside ASCII, including Japanese text;
- cancellation before spawn and during prompt wait;
- ConPTY unavailable/attribute rejected, child loader failure, backend crash, timeout, and forced Job termination;
- inherited-handle inventory proves controller ends and unrelated handles are absent from the child;
- process command line, environment, stdout/stderr logs, config, temporary tree, and release diagnostics contain no sentinel secret;
- output drain and pseudoconsole teardown complete without deadlock on supported Windows 10 and Windows 11 workers;
- wrong-password and every failure path publish no destination and leave no AppContainer profile or temporary tree.

Until this matrix passes, encrypted archives remain unsupported and no password UI is shown.
