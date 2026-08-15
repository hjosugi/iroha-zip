# Encrypted archive input / 暗号化書庫の入力

Updated / 更新: 2026-08-15

## Status / 対応状況

iroha-zip supports one-use password input for encrypted ZIP preview and extraction on Windows 10
version 1809 and later. Use the boolean `--prompt-password` flag; a native bilingual password
dialog obtains the secret before the sandbox is created. No CLI option accepts the password value.

iroha-zipは、Windows 10 version 1809以降で暗号化ZIPのpreviewと展開に使う一回限りの
パスワード入力に対応します。boolean flagの`--prompt-password`を指定すると、sandbox作成前に
日英併記のnative password dialogを表示します。パスワード値を受け取るCLI optionはありません。

```powershell
iroha-zip.exe preview .\encrypted.zip --prompt-password
iroha-zip.exe extract .\encrypted.zip --prompt-password
iroha-zip.exe extract .\encrypted.zip --output D:\Extracted\archive --prompt-password
```

The pinned libarchive 3.8.9 interface supports ZIP encryption only. The Windows E2E contract
generates and exercises ZipCrypto, WinZip AES-128, and WinZip AES-256 archives with a verified MSYS2
backend. Backend build capabilities still control actual compatibility. Double-click extraction does
not open a password prompt, encrypted archive creation is not exposed, and the explicitly dangerous
`--allow-unsandboxed` path rejects password transport.

固定対象のlibarchive 3.8.9 interfaceで暗号化に対応する形式はZIPだけです。Windows E2E契約は、
検証済みMSYS2 backendでZipCrypto、WinZip AES-128、WinZip AES-256を生成して検査します。実際の
互換性は取り込んだbackend buildにも依存します。ダブルクリック展開ではパスワード画面を出さず、
暗号化書庫の作成も公開していません。危険な例外である`--allow-unsandboxed`経路はパスワード転送を
拒否します。

## Why a normal pipe is not enough / 通常pipeを使わない理由

The libarchive bsdtar manual documents `--passphrase <passphrase>`, but explicitly warns that it is
insecure. The value becomes process-command-line data, so iroha-zip never uses that option for user
secrets. Without the option, stock Windows bsdtar installs a callback that requires console input and
temporarily disables console echo. A redirected anonymous pipe is not a compatible console handle.

libarchiveのbsdtar manualは`--passphrase <passphrase>`を定義していますが、安全でないことも
明記しています。値がprocess command lineに残るため、iroha-zipは利用者の秘密にこのoptionを
使いません。optionを省略したstock Windows bsdtarは、console inputを要求して一時的にechoを
無効化するcallbackを使います。redirectした通常の匿名pipeは互換console handleではありません。

Primary references / 一次資料:

- [libarchive 3.8.9 `bsdtar(1)` encryption and passphrase contract](https://github.com/libarchive/libarchive/blob/v3.8.9/tar/bsdtar.1)
- [bsdtar reader installs the passphrase callback](https://github.com/libarchive/libarchive/blob/v3.8.9/tar/read.c)
- [bsdtar Windows passphrase implementation](https://github.com/libarchive/libarchive/blob/v3.8.9/libarchive_fe/passphrase.c)
- [Windows `CreatePseudoConsole`](https://learn.microsoft.com/en-us/windows/console/createpseudoconsole)
- [Microsoft pseudoconsole session lifecycle](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session)
- [Anonymous pipe security](https://learn.microsoft.com/en-us/windows/win32/ipc/anonymous-pipe-security-and-access-rights)
- [Job Object unhandled-exception behavior](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information)

## One-use ConPTY channel / 一回限りのConPTY channel

The password path creates a hidden pseudoconsole dynamically. The host gives only the
pseudoconsole-facing synchronous pipe ends to `CreatePseudoConsole`; controller ends have handle
inheritance explicitly disabled. The child receives `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` together
with the existing zero-capability `SECURITY_CAPABILITIES` and Job Object attributes. As with every
normal backend launch, it starts suspended and is resumed only after AppContainer/LPAC mode and a
zero capability count are positively verified.

パスワード経路はhidden pseudoconsoleを動的に作成します。hostはpseudoconsole側の同期pipe endだけを
`CreatePseudoConsole`へ渡し、controller endのhandle継承を明示的に無効化します。childには既存の
capability 0件の`SECURITY_CAPABILITIES`、Job Objectとともに
`PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`を渡します。通常backend起動と同様にsuspendedで作成し、
AppContainer／LPAC modeとcapability 0件を肯定確認した後だけresumeします。

A dedicated thread drains the merged terminal output concurrently. Its incremental monitor:

- caps raw pseudoconsole output at 1 MiB;
- removes ANSI CSI/OSC sequences and escapes other control bytes before logging;
- recognizes only the pinned `Enter passphrase:` prompt at the start of a logical line;
- accepts exactly one prompt and treats every additional prompt as fatal;
- suppresses every log byte after the prompt, including any unexpected terminal echo;
- terminates the Job on output overflow, retry, timeout, monitor failure, or backend failure.
- applies `JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION` so an unhandled child fault does not enter
  the normal interactive Windows fault-reporting path.

専用threadが統合terminal outputを並行してdrainします。incremental monitorは次を必須にします。

- raw pseudoconsole outputを1 MiBで制限する
- logへ出す前にANSI CSI／OSCを除去し、その他のcontrol byteをescapeする
- logical line先頭の固定文字列`Enter passphrase:`だけをpromptとして認識する
- promptは1回だけ許し、追加promptをfatal errorにする
- 予期しないterminal echoを含め、prompt以後の全byteをlogへ出さない
- output超過、retry、timeout、monitor失敗、backend失敗時はJobを終了する
- `JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION`を適用し、childのunhandled faultを通常の対話的な
  Windows fault-reporting経路へ進めない

After the first exact prompt, the controller converts the bounded secret to UTF-8, writes one line,
closes the input controller, and zeroizes the transport buffer. The password never becomes an
argument, environment value, file, named object, configuration field, or diagnostic value.

最初の正確なpromptを受け取ると、controllerは上限付きsecretをUTF-8へ変換し、1行だけ書き、input
controllerを閉じてtransport bufferをzeroizeします。パスワードをargument、environment value、
file、named object、configuration field、diagnostic valueとして保存しません。

## Secret lifetime and UI / secretの寿命とUI

The native edit control uses the Windows password style, is limited to 1,022 UTF-16 units, and is
cleared before the dialog is destroyed. Empty strings, NUL, line breaks, invalid UTF-16, more than
1,022 UTF-16 units, or more than 1,022 UTF-8 bytes are rejected. The owning Rust type is not
`Clone`, always redacts `Debug` and `Display`, and stores both UI and transport buffers in zeroizing
containers. Cancellation returns successfully without constructing a sandbox or child process.

native edit controlはWindows password styleを使い、1,022 UTF-16 unitに制限し、dialog破棄前に
内容を消去します。空文字、NUL、改行、不正UTF-16、1,022 UTF-16 unit超過、1,022 UTF-8 byte超過を
拒否します。所有するRust typeは`Clone`を実装せず、`Debug`／`Display`を常にredactし、UI bufferと
transport bufferをzeroizing containerに保持します。cancelはsandboxもchild processも作らず正常終了
します。

Automatic retries are forbidden. A wrong password or second prompt terminates the one-use channel,
cleans the sandbox, and publishes no destination. The user may explicitly start a new operation and
enter a new secret.

自動retryは禁止です。wrong passwordまたは2回目のpromptでは一回限りのchannelを終了し、sandboxを
cleanupしてdestinationを公開しません。利用者が明示的に新しい操作とsecret入力を開始できます。

## Fail-closed state machine / fail-closed状態遷移

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
  ├─ nonzero/wrong password ─────> no retry + cleanup + no publication
  └─ success ────────────────────> existing tree audit/publication path
```

`CreatePseudoConsole` and `ClosePseudoConsole` are resolved dynamically so Windows versions without
ConPTY can still launch iroha-zip for ordinary unencrypted operations. A requested password operation
fails closed when ConPTY is unavailable; it never falls back to command-line input, an ordinary pipe,
or an unsandboxed child.

`CreatePseudoConsole`／`ClosePseudoConsole`は動的に解決するため、ConPTYがないWindowsでも通常の
非暗号化操作ではiroha-zipを起動できます。パスワード操作を要求してConPTYがない場合はfail closedに
なり、command line入力、通常pipe、unsandboxed childへfallbackしません。

## Verification contract / 検証契約

Platform-neutral regressions cover redaction, bounded UTF-16/UTF-8 conversion, invalid input,
cancellation, fragmented and spoofed prompts, retry rejection, control-sequence filtering, output
limits, and suppression after the prompt. Windows integration tests run the one-use transport inside
a real zero-capability AppContainer and cover a correct Japanese secret, retry, timeout, output
overflow, backend abort, log absence, and explicit cleanup.

platform-neutral regressionはredaction、UTF-16／UTF-8上限、不正入力、cancel、分割prompt、spoof prompt、
retry拒否、control sequence除去、output上限、prompt後のlog抑止を検査します。Windows integration testは
実際のcapability 0件AppContainer内でone-use transportを実行し、正しい日本語secret、retry、timeout、
output overflow、backend abort、log非露出、明示cleanupを検査します。

The schema-v5 Windows E2E harness additionally creates ZipCrypto, AES-128, and AES-256 ZIPs from
deterministic public fixture data. It drives the native bilingual dialog through UI Automation,
previews and extracts every variant, compares the complete SHA-256 tree, rejects a wrong password,
cancels before spawn, requires no destination on either failure path, and asserts that its public
sentinel password is absent from stdout/stderr. The generator uses `--passphrase` only with a
deliberately public test sentinel; product handling never does so.

schema-v5 Windows E2E harnessは決定的な公開fixtureからZipCrypto、AES-128、AES-256 ZIPも作成します。
UI Automationで日英native dialogを操作し、全variantをpreview／extractして完全SHA-256 treeを比較し、
wrong passwordを拒否し、spawn前cancelを検査し、両failureでdestinationがなく、公開sentinel passwordが
stdout／stderrにないことを要求します。generatorは意図的に公開したtest sentinelに限り
`--passphrase`を使用し、製品の処理経路では使用しません。

## Residual risks / 残るrisk

Zeroization reduces ordinary lifetime but cannot promise removal from CPU copies, operating-system
paging, hibernation, privileged debuggers, or crash/minidump capture outside iroha-zip's control.
ConPTY and libarchive remain trusted platform/backend boundaries. The hosted Server and Windows 11
ARM evidence is not a Windows 10/11 x64 desktop certification or an independent security audit.

zeroizationは通常の保持時間を減らしますが、CPU copy、OS paging、hibernation、privileged debugger、
iroha-zipの管理外にあるcrash／minidumpからの消去は保証できません。ConPTYとlibarchiveも信頼境界に
残ります。hosted ServerとWindows 11 ARMの証跡はWindows 10/11 x64 desktop認証や独立security auditの
代替ではありません。
