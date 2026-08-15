# iroha-zip 0.6.1

## 日本語

iroha-zip 0.6.1は、0.6.0の暗号化ZIPとsandboxの境界を維持しながら、Settingsの
Per-Monitor V2 DPI対応とWindows実測証跡の長期保存を強化するpatch安定版です。Windows x64と
native Windows ARM64を別々のnative runnerでbuildし、意図的にAuthenticode未署名のまま、
exact 11 assetのimmutable Releaseとして公開します。

### ダウンロードの選び方

- **一般的なIntel/AMD PC**: `iroha-zip-0.6.1-windows-x64.zip`
- **Windows on ARM PC**: `iroha-zip-0.6.1-windows-arm64.zip`
- CLIだけが必要な場合は、対応する`iroha-zip-0.6.1-windows-<arch>.exe`を選べます。
- native設定画面と関連付けlauncherも、各architecture向けの個別EXEがあります。
- `SHA256SUMS.txt`は2つのZIPと6つの個別EXEをまとめて対象にします。各ZIPには専用の
  `.zip.sha256`もあります。

配布ZIPにはlibarchive、`bsdtar.exe`、backend DLLを同梱しません。初回利用時に設定画面から
自分が信頼するbackendを取り込み、`doctor`を成功させてください。公式EXEが使用する
`VCRUNTIME140.dll`がない場合は、対象architectureのMicrosoft公式Visual C++ v14
Redistributableを導入してください。

### 暗号化ZIP

```powershell
iroha-zip.exe preview .\encrypted.zip --prompt-password
iroha-zip.exe extract .\encrypted.zip --prompt-password
```

- `--prompt-password`は値を受け取らないboolean flagです。日英native dialogの保護editへ入力し、
  cancel時はsandboxも出力先も作りません。
- passwordは最大1,022 UTF-8 byteへ制限し、non-`Clone`・常時redactの型がUTF-16／UTF-8
  bufferを所有して、利用後にzeroizeします。dialog controlも破棄前に消去します。
- 親processはbyte同一・封印済みの内部childをsuspendedで作成し、そのAppContainer／LPAC tokenと
  capability 0件を外部確認します。childも処理前に同じ条件を自己確認します。
- 明示的handle listはchild stdinのread endだけを継承します。controller write endは非継承です。
  検証済みchildをsuspendしたまま、親が専用4 KiB匿名pipeへ値を一度だけ書いてcloseし、その後だけ
  childを一度resumeします。同期pipe flushや安全性を弱めるfallbackはありません。
- 内部childはmanifest固定済みlibarchive DLLだけをloadし、public password APIで読取ります。
  通常file／directory以外、非canonical path、重複・大小文字・separator alias、設定した
  file／directory数、単一／合計byte、深さ、path長の超過を作成前と読取中に拒否します。
- 正しいpasswordはZipCrypto、WinZip AES-128、AES-256でpreview／展開tree一致を確認しています。
  wrong password、timeout、EOF、overflow、異常終了、cancel、cleanup failureは非公開でfail closedします。

### v0.6.0からの変更

- 3 EXEをPer-Monitor V2 DPI awareとし、Settingsは`WM_DPICHANGED`のsuggested rectangleを適用して、
  96-DPI基準から全control・scroll・system fontを再計算します。
- native UIA証跡は実効Per-Monitor V2 contextと日英それぞれの96→144→96 synthetic transitionを必須にし、
  suggested geometry、150%拡大、往復後のdrift-free復帰を検証します。
- native ARM64と固定Windows Serverのevidence artifactを90日保持し、exact-main runの11 JSONを
  raw/canonical SHA-256とActions artifact API digest付きのcanonical snapshotとして保存します。
- `main`はLinux・Windows x64・native ARM64・dependency policy・3言語CodeQLを必須checkとし、
  adminを含むforce push／削除防止、conversation resolution、linear history、squash-onlyを適用します。

### 既知の境界

- password入力はCLIの暗号化ZIP `preview`／`extract`だけです。ダブルクリック、暗号化書庫の作成、
  ZIP以外の暗号化形式、自動retryには対応しません。
- 現在の公式binaryはAuthenticode未署名です。SmartScreenを無効化せず、公開Release自身の
  `SHA256SUMS.txt`とtag-ref GitHub artifact attestationを確認してください。
- セキュリティ監査済み製品ではありません。Windows 10/11 x64 desktop実機、実験的LPAC、
  screen reader／mixed DPI、さらに広いmalformed format／race試験は継続課題です。

実測範囲は[暗号化書庫の境界](https://github.com/hjosugi/iroha-zip/blob/v0.6.1/docs/ENCRYPTED_ARCHIVES.md)、
[Windows E2E](https://github.com/hjosugi/iroha-zip/blob/v0.6.1/docs/WINDOWS_E2E.md)、
[ARM64 status](https://github.com/hjosugi/iroha-zip/blob/v0.6.1/docs/ARM64.md)、
[Build Status](https://github.com/hjosugi/iroha-zip/blob/v0.6.1/docs/BUILD_STATUS.md)を確認してください。

---

## English

iroha-zip 0.6.1 is a stable patch release that retains the encrypted-ZIP and sandbox boundaries from
0.6.0 while strengthening Per-Monitor V2 DPI behavior in Settings and durable preservation of measured
Windows evidence. Windows x64 and native Windows ARM64 are built on separate native runners and
published intentionally without Authenticode signatures as an immutable Release with exactly 11 assets.

### Which download to choose

- **Typical Intel/AMD PC**: `iroha-zip-0.6.1-windows-x64.zip`
- **Windows on ARM PC**: `iroha-zip-0.6.1-windows-arm64.zip`
- If you need only the CLI, choose the matching `iroha-zip-0.6.1-windows-<arch>.exe`.
- Separate native Settings and file-association launcher executables are available for each architecture.
- `SHA256SUMS.txt` covers both ZIPs and all six standalone executables. Each ZIP also has its own
  `.zip.sha256` sidecar.

The ZIP does not bundle libarchive, `bsdtar.exe`, or backend DLLs. Import a backend you trust in
Settings and require `doctor` to pass. If `VCRUNTIME140.dll`, which the official EXEs import, is
absent, install Microsoft's architecture-matching Visual C++ v14 Redistributable.

### Encrypted ZIPs

```powershell
iroha-zip.exe preview .\encrypted.zip --prompt-password
iroha-zip.exe extract .\encrypted.zip --prompt-password
```

- `--prompt-password` is a value-free boolean flag. Input goes to the protected edit in a bilingual
  native dialog; cancellation creates neither a sandbox nor a destination.
- The password is bounded to 1,022 UTF-8 bytes. A non-`Clone`, always-redacted type owns and zeroizes
  its UTF-16/UTF-8 buffers, and the dialog control is cleared before destruction.
- The parent creates a byte-identical sealed internal child suspended, externally verifies its
  AppContainer/LPAC token and zero capabilities, and the child independently rechecks those conditions.
- The explicit inherited-handle list admits only the child's stdin read end; the controller write end
  is non-inheritable. While the verified child remains suspended, the parent writes the value once to
  a dedicated 4 KiB anonymous pipe, closes it, and only then performs the sole resume. There is no
  synchronous pipe flush or weaker fallback.
- The internal child loads only manifest-pinned libarchive DLLs and reads through the public password
  API. Before creation and while reading, it rejects non-file/directory entries, noncanonical paths,
  duplicate/case/separator aliases, and configured file/directory, per-file/total-byte, depth, and
  path-length limit violations.
- Correct-password preview and extraction trees match for ZipCrypto, WinZip AES-128, and AES-256.
  Wrong password, timeout, EOF, overflow, crash, cancellation, and cleanup failure all fail closed
  without publication.

### Changes from v0.6.0

- Declare all three executables Per-Monitor V2 DPI aware. Settings applies the `WM_DPICHANGED`
  suggested rectangle and recomputes every control, scroll extent, and system font from a stable
  96-DPI baseline.
- Require native UIA evidence for the effective Per-Monitor V2 context and bilingual 96→144→96
  synthetic transitions, including suggested geometry, 150% scaling, and drift-free restoration.
- Retain native ARM64 and fixed Windows Server evidence artifacts for 90 days and preserve the exact-main
  run's 11 JSON reports as a canonical snapshot with raw/canonical SHA-256 and Actions artifact API digests.
- Protect `main` with required Linux, Windows x64, native ARM64, dependency-policy, and three-language
  CodeQL checks, admin enforcement, force-push/deletion denial, conversation resolution, linear history,
  and squash-only merges.

### Known boundaries

- Password input is limited to CLI `preview`/`extract` for encrypted ZIPs. Double-click prompting,
  encrypted archive creation, encrypted non-ZIP formats, and automatic retries are not supported.
- Current official binaries are not Authenticode-signed. Do not disable SmartScreen; verify the
  published Release's own `SHA256SUMS.txt` and tag-ref GitHub artifact attestation.
- This is not a security-audited product. Windows 10/11 x64 desktop devices, experimental LPAC,
  screen readers/mixed DPI, and broader malformed-format/race testing remain open.

See [Encrypted archives](https://github.com/hjosugi/iroha-zip/blob/v0.6.1/docs/ENCRYPTED_ARCHIVES.md),
[Windows E2E](https://github.com/hjosugi/iroha-zip/blob/v0.6.1/docs/WINDOWS_E2E.md),
[ARM64 status](https://github.com/hjosugi/iroha-zip/blob/v0.6.1/docs/ARM64.md), and
[Build Status](https://github.com/hjosugi/iroha-zip/blob/v0.6.1/docs/BUILD_STATUS.md) for the measured boundary.
