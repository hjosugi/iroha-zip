# iroha-zip 0.6.3

## 日本語

iroha-zip 0.6.3は、0.6.2のarchive／sandbox／Per-Monitor V2境界を維持しながら、ダブルクリック
shellの失敗・警告を日英対応し、Release／Pages／repository automationの検証境界を強化する
patch安定版です。Windows x64とnative Windows ARM64を別々のnative runnerでbuildし、意図的に
Authenticode未署名のまま、exact 11 assetのimmutable Releaseとして公開します。

### ダウンロードの選び方

- **一般的なIntel/AMD PC**: `iroha-zip-0.6.3-windows-x64.zip`
- **Windows on ARM PC**: `iroha-zip-0.6.3-windows-arm64.zip`
- CLIだけが必要な場合は、対応する`iroha-zip-0.6.3-windows-<arch>.exe`を選べます。
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

### v0.6.2からの変更

- ダブルクリックshellの全8 error分類とWindows信頼連携warningに日英の状況説明を追加し、既存の
  技術詳細を保持します。通常testは各分類とwarningの正確な日英contractを検証します。
- `v0.6.2`のexact 11 asset、hash、x64／ARM64 PE identity、未署名Certificate Table、tag、
  workflow／Release attestationを独立検証し、恒久snapshotと通常Rust regressionへ固定しました。
- 将来の公開workflowはimmutable/latest readback後にRelease attestationと全11 local assetを
  有界retryで照合します。PagesはRelease APIのasset名、正のbyte長、SHA-256 digestも必須にします。
- 全workflow jobのtimeout、全checkoutのcredential非保持、full-SHA action、write permission、
  repository内Markdown link、日英contribution template／Pages metadataを通常testで固定しました。
- backend exporterの署名済みpackageにlicense一覧を取る`bsdtar`も、`ldd`やpacmanと同じ
  個別180秒timeoutのlauncherを必ず通し、直接起動の再導入を通常CIで拒否します。

### 既知の境界

- password入力はCLIの暗号化ZIP `preview`／`extract`だけです。ダブルクリック、暗号化書庫の作成、
  ZIP以外の暗号化形式、自動retryには対応しません。
- 現在の公式binaryはAuthenticode未署名です。SmartScreenを無効化せず、公開Release自身の
  `SHA256SUMS.txt`とtag-ref GitHub artifact attestationを確認してください。
- セキュリティ監査済み製品ではありません。Windows 10/11 x64 desktop実機、実験的LPAC、
  screen reader／mixed DPI、さらに広いmalformed format／race試験は継続課題です。

実測範囲は[暗号化書庫の境界](https://github.com/hjosugi/iroha-zip/blob/v0.6.3/docs/ENCRYPTED_ARCHIVES.md)、
[Windows E2E](https://github.com/hjosugi/iroha-zip/blob/v0.6.3/docs/WINDOWS_E2E.md)、
[ARM64 status](https://github.com/hjosugi/iroha-zip/blob/v0.6.3/docs/ARM64.md)、
[Build Status](https://github.com/hjosugi/iroha-zip/blob/v0.6.3/docs/BUILD_STATUS.md)を確認してください。

---

## English

iroha-zip 0.6.3 is a stable patch release that retains the archive, sandbox, and Per-Monitor V2
boundaries from 0.6.2 while making double-click shell failures and warnings bilingual and strengthening
the verification boundaries around Releases, Pages, and repository automation. Windows x64 and native
Windows ARM64 are built on separate native runners and published intentionally without Authenticode
signatures as an immutable Release with exactly 11 assets.

### Which download to choose

- **Typical Intel/AMD PC**: `iroha-zip-0.6.3-windows-x64.zip`
- **Windows on ARM PC**: `iroha-zip-0.6.3-windows-arm64.zip`
- If you need only the CLI, choose the matching `iroha-zip-0.6.3-windows-<arch>.exe`.
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

### Changes from v0.6.2

- Add Japanese and English outcome summaries to all eight double-click shell error categories and the
  incomplete Windows trust-handoff warning while retaining the existing technical details. Ordinary
  tests enforce the exact bilingual contract for every category and the warning.
- Independently verify and retain the exact 11 `v0.6.2` assets, hashes, x64/ARM64 PE identities,
  unsigned Certificate Tables, tag, and workflow/Release attestations in a durable snapshot and
  ordinary Rust regression.
- After immutable/latest readback, require future publication workflows to verify the Release
  attestation and all 11 local assets with bounded retries. Pages also requires every Release API
  asset's exact name, positive byte length, and SHA-256 digest.
- Lock every workflow job timeout, discarded checkout credential, full-SHA action, write permission,
  repository Markdown link, and bilingual contribution-template/Pages-metadata contract into
  ordinary tests.
- Route the backend exporter's `bsdtar` inventory of signed-package licenses through the same
  per-command 180-second launcher as `ldd` and pacman, and make ordinary CI reject any reintroduced
  direct invocation.

### Known boundaries

- Password input is limited to CLI `preview`/`extract` for encrypted ZIPs. Double-click prompting,
  encrypted archive creation, encrypted non-ZIP formats, and automatic retries are not supported.
- Current official binaries are not Authenticode-signed. Do not disable SmartScreen; verify the
  published Release's own `SHA256SUMS.txt` and tag-ref GitHub artifact attestation.
- This is not a security-audited product. Windows 10/11 x64 desktop devices, experimental LPAC,
  screen readers/mixed DPI, and broader malformed-format/race testing remain open.

See [Encrypted archives](https://github.com/hjosugi/iroha-zip/blob/v0.6.3/docs/ENCRYPTED_ARCHIVES.md),
[Windows E2E](https://github.com/hjosugi/iroha-zip/blob/v0.6.3/docs/WINDOWS_E2E.md),
[ARM64 status](https://github.com/hjosugi/iroha-zip/blob/v0.6.3/docs/ARM64.md), and
[Build Status](https://github.com/hjosugi/iroha-zip/blob/v0.6.3/docs/BUILD_STATUS.md) for the measured boundary.
