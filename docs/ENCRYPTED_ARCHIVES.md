# Encrypted archive input / 暗号化書庫の入力

Updated / 更新: 2026-08-15

## Status / 対応状況

iroha-zip supports one-use password input for encrypted ZIP preview and extraction on Windows 10
and later. Add the boolean `--prompt-password` flag to open a native bilingual password dialog.
There is no CLI option that accepts a password value.

iroha-zipは、Windows 10以降で暗号化ZIPのpreviewと展開に使う一回限りのパスワード入力に対応します。
boolean flagの`--prompt-password`を付けると、日英併記のnative password dialogを表示します。
パスワード値を受け取るCLI optionはありません。

```powershell
iroha-zip.exe preview .\encrypted.zip --prompt-password
iroha-zip.exe extract .\encrypted.zip --prompt-password
iroha-zip.exe extract .\encrypted.zip --output D:\Extracted\archive --prompt-password
```

The pinned libarchive 3.8.9 interface supports ZIP encryption only. The Windows E2E contract covers
ZipCrypto, WinZip AES-128, and WinZip AES-256 using a verified MSYS2 backend. Actual compatibility
still depends on the imported backend build. Double-click extraction does not prompt, encrypted
archive creation is not exposed, and the explicitly dangerous `--allow-unsandboxed` path refuses
password transport.

固定対象のlibarchive 3.8.9 interfaceで暗号化に対応する形式はZIPだけです。Windows E2E契約は、
検証済みMSYS2 backendでZipCrypto、WinZip AES-128、WinZip AES-256を検査します。実際の互換性は
取り込んだbackend buildにも依存します。ダブルクリック展開ではパスワード画面を出さず、暗号化書庫の
作成も公開していません。危険な例外である`--allow-unsandboxed`経路はパスワード転送を拒否します。

## Why iroha-zip does not invoke bsdtar / bsdtarを起動しない理由

The bsdtar manual defines `--passphrase <passphrase>` and warns that it is insecure. The value would
become process-command-line data, so iroha-zip never uses it for a user secret. Without that option,
stock Windows bsdtar requires a console handle. A redirected pipe is not such a handle, and ConPTY
does not provide a compatible input handle to this callback inside the zero-capability AppContainer
on the tested Windows runner.

bsdtar manualは`--passphrase <passphrase>`を定義していますが、安全でないことも明記しています。
値がprocess command lineに残るため、iroha-zipは利用者の秘密にこのoptionを使いません。optionを
省略したstock Windows bsdtarはconsole handleを要求します。redirectしたpipeはconsole handleではなく、
検証したWindows runnerでは、ConPTYもcapability 0件のAppContainer内にあるcallbackへ互換input handleを
提供しませんでした。

Password operations therefore run a byte-identical, sealed copy of iroha-zip as the isolated child.
That child loads only manifest-pinned libarchive DLL candidates, registers the one supplied value
with `archive_read_add_passphrase`, and extracts through the libarchive read API. It accepts only
regular-file and directory entries, validates every UTF-8 path before creation, creates files with
create-new and open-reparse-point semantics, and enforces file, directory, per-file, total-size,
depth, and path-length limits while reading. The existing independent post-extraction tree audit and
atomic publication path still run afterward.

そのためpassword操作では、iroha-zip自身のbyte-identicalかつ封印済みcopyをisolated childとして
実行します。childはmanifest固定済みlibarchive DLL候補だけをloadし、受け取った1値を
`archive_read_add_passphrase`へ登録してlibarchive read APIで展開します。通常fileとdirectory以外を
作成前に拒否し、全UTF-8 pathを検証し、create-new／open-reparse-pointでfileを作り、読取中にも
file数、directory数、単一／合計容量、深さ、path長を制限します。その後も既存の独立した展開後tree
監査とatomic publicationを必ず実行します。

Primary references / 一次資料:

- [libarchive 3.8.9 `bsdtar(1)` encryption and passphrase contract](https://github.com/libarchive/libarchive/blob/v3.8.9/tar/bsdtar.1)
- [libarchive 3.8.9 password API](https://github.com/libarchive/libarchive/blob/v3.8.9/libarchive/archive_read_add_passphrase.3)
- [libarchive 3.8.9 password-copy implementation](https://github.com/libarchive/libarchive/blob/v3.8.9/libarchive/archive_read_add_passphrase.c)
- [bsdtar Windows passphrase implementation](https://github.com/libarchive/libarchive/blob/v3.8.9/libarchive_fe/passphrase.c)
- [Windows process handle inheritance](https://learn.microsoft.com/en-us/windows/win32/procthread/inheritance)
- [Anonymous pipe security](https://learn.microsoft.com/en-us/windows/win32/ipc/anonymous-pipe-security-and-access-rights)
- [Job Object unhandled-exception behavior](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information)

## One-use anonymous channel / 一回限りの匿名channel

The parent creates an anonymous pipe with both ends initially non-inheritable. It marks only the
child read end inheritable and passes only stdin/stdout/stderr in an explicit
`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`. The controller write end remains non-inheritable and is checked
at runtime. The child is created suspended with the existing zero-capability
`SECURITY_CAPABILITIES` and Job Object attributes. The parent writes nothing until the requested
AppContainer/LPAC mode and zero capabilities have been positively verified.

親は両endを最初から非継承にした匿名pipeを作成します。child側read endだけを継承可能にし、明示的な
`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`にはstdin／stdout／stderrだけを渡します。controller側write endは
非継承のままで、runtimeにも確認します。childは既存のcapability 0件`SECURITY_CAPABILITIES`とJob
Object属性を使ってsuspendedで作成します。要求したAppContainer／LPAC modeとcapability 0件を肯定確認
するまで、親は秘密を書きません。

After verification and while the child is still suspended, the parent converts the bounded value to
UTF-8, writes exactly one delimited value into a dedicated 4 KiB pipe, closes the controller end to
establish EOF, and zeroizes its transport buffer. Only then does it resume the child. Avoiding a
synchronous pipe flush is part of the launch contract: the child cannot be required to read before
it is allowed to run. The child accepts one bounded non-empty UTF-8 value and reads to EOF. The
password never becomes an argument, environment value, file, named object, configuration field, or
diagnostic value. `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` keeps the child-process count at one, and
`JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION`, timeout, memory, and live filesystem limits remain
active.

検証後、childがまだsuspendedの間に、親は上限付きvalueをUTF-8へ変換し、専用4 KiB pipeへ
delimiter付きの1値だけを書き、controller endを閉じてEOFを確定し、transport bufferをzeroize
します。その後だけchildをresumeします。同期pipe flushを行わないこともlaunch contractの一部です。
childは実行許可前にreadを要求されないからです。childは上限内の空でないUTF-8 1値だけをEOFまで読みます。
パスワードをargument、environment value、file、named object、configuration field、diagnostic value
として保存しません。`JOB_OBJECT_LIMIT_ACTIVE_PROCESS`でchild process数を1に保ち、
`JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION`、timeout、memory、live filesystem上限も維持します。

## Secret lifetime and failure behavior / secretの寿命と失敗時動作

The native edit control uses the Windows password style, is limited to 1,022 UTF-16 units, and is
cleared before destruction. Empty strings, NUL, line breaks, invalid UTF-16, more than 1,022 UTF-16
units, or more than 1,022 UTF-8 bytes are rejected. The owning Rust type is not `Clone`, always
redacts `Debug` and `Display`, and keeps UI and transport storage in zeroizing containers.

native edit controlはWindows password styleを使い、1,022 UTF-16 unitに制限し、破棄前に内容を消去
します。空文字、NUL、改行、不正UTF-16、1,022 UTF-16 unit超過、1,022 UTF-8 byte超過を拒否します。
所有するRust typeは`Clone`を実装せず、`Debug`／`Display`を常にredactし、UIとtransportのstorageを
zeroizing containerに保持します。

Cancellation returns without creating a sandbox or child. A wrong password, malformed channel,
unsupported entry, timeout, resource overflow, loader failure, or child crash cleans the sandbox and
publishes no destination. There is no automatic retry; the user must explicitly start a new
operation. The explicitly unsandboxed path and raw compressed streams fail closed before transport.

cancelはsandboxもchildも作らず終了します。wrong password、不正channel、未対応entry、timeout、resource
超過、loader失敗、child crashではsandboxをcleanupし、destinationを公開しません。自動retryはなく、
利用者が明示的に新しい操作を開始します。unsandboxed経路とraw compressed streamはtransport前に
fail closedになります。

```text
NoSecret
  ├─ cancel ─────────────────────> Cancelled (no sandbox/process)
  └─ bounded secret ─────────────> SecretReady
SecretReady
  ├─ pipe/spawn/isolation failure > Failed + zeroize + cleanup
  └─ verified suspended child ───> write once + close + zeroize + resume
ChildRunning
  ├─ parse/decrypt/policy failure > no retry + cleanup + no publication
  └─ success ────────────────────> independent audit + atomic publication
```

## Verification contract / 検証契約

Platform-neutral tests cover boolean-only CLI input, redaction, bounded conversion, invalid input,
cancellation, and command-line/environment absence. Windows integration tests run the anonymous
transport inside a real zero-capability AppContainer and cover a Japanese value, EOF after one
value, timeout, large output, child abort, log absence, and cleanup.

platform-neutral testはboolean-only CLI、redaction、変換上限、不正入力、cancel、command line／environment
非露出を検査します。Windows integration testは実際のcapability 0件AppContainerで匿名transportを
実行し、日本語value、1値後のEOF、timeout、大量output、child abort、log非露出、cleanupを検査します。

The schema-v5 Windows E2E harness generates deterministic ZipCrypto, AES-128, and AES-256 ZIPs,
locates and validates the bilingual dialog through UI Automation, drives its native standard buttons
through bounded synchronous `WM_COMMAND` / `BN_CLICKED` notifications, previews and extracts every
variant, compares the complete SHA-256 tree, rejects a wrong password, cancels before spawn,
requires no destination on
either failure, and checks that its deliberately public sentinel is absent from stdout/stderr. Only
fixture generation uses generator-side `--passphrase`; the product path never does.

schema-v5 Windows E2E harnessは決定的なZipCrypto、AES-128、AES-256 ZIPを生成し、UI Automationで
日英dialogと標準button contractを検証してから、実dialog procedureへ有界な同期
`WM_COMMAND` / `BN_CLICKED`通知を送り、
全variantのpreview／extractと完全SHA-256 treeを比較します。wrong password拒否、
spawn前cancel、両失敗時のdestination不存在、意図的に公開したsentinelのstdout／stderr非露出も検査
します。generator側`--passphrase`はfixture生成だけで使い、製品経路では使いません。

## Residual risks / 残るrisk

libarchive copies a registered passphrase into its reader allocation and frees it when the reader is
destroyed, but upstream does not promise overwriting that allocation first. Zeroization reduces the
lifetime of iroha-zip-owned buffers but cannot promise removal from allocator reuse, CPU copies, OS
paging, hibernation, privileged debugging, or external crash/minidump capture. The imported
libarchive DLL and Windows kernel remain trust boundaries. CI evidence is not desktop certification
or an independent security audit.

libarchiveは登録passphraseをreader allocationへcopyしてreader破棄時にfreeしますが、upstreamはfree前の
上書きを保証していません。zeroizationはiroha-zip所有bufferの寿命を減らしますが、allocator再利用、
CPU copy、OS paging、hibernation、privileged debug、外部crash／minidumpからの消去は保証できません。
取り込んだlibarchive DLLとWindows kernelは信頼境界に残ります。CI証跡はdesktop認証や独立security
auditの代替ではありません。
