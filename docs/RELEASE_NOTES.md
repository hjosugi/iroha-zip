# iroha-zip 0.4.1

## 日本語

iroha-zip 0.4.1 は、Windows x64 向け安定版のセキュリティ境界・回帰証跡を更新する
maintenance releaseです。`v0.4.0`を利用している場合は、この版へ置き換えてください。

### ダウンロードの選び方

- **通常は `iroha-zip-0.4.1-windows-x64.zip`**: 3つの実行ファイル、設定・関連付け
  スクリプト、日英README、ライセンス、設計文書を含む完全版です。
- **`iroha-zip-0.4.1-windows-x64.exe`**: CLI本体だけが必要な場合。
- **`iroha-zip-settings-0.4.1-windows-x64.exe`**: ネイティブ設定画面。
- **`iroha-zip-shell-0.4.1-windows-x64.exe`**: Windowsのファイル関連付け用ランチャー。
- **`SHA256SUMS.txt`**: ZIPと3つのEXEのSHA-256一覧。

### 重要

- 3つのEXEは **Authenticode未署名** です。SmartScreenの警告を無効化せず、
  `SHA256SUMS.txt`とGitHub artifact attestationで出所を確認してください。
- libarchive / `bsdtar.exe` は同梱していません。設定画面から自分が信頼するbackendを
  取り込み、`iroha-zip.exe doctor`が成功することを確認してください。
- このReleaseはWindows x64専用です。native ARM64 CIはありますが、ARM64 backend・書庫matrix・
  Release assetは未完成です。
- セキュリティ監査済み製品ではありません。Windows 10/11 desktop実機matrixや未完の形式・
  race試験は、同梱のBuild Statusに正確に記録しています。

### v0.4.0からの主な変更

- Windowsの全sandbox子processを`CREATE_SUSPENDED`で生成し、Job Objectへ割り当て、要求した
  AppContainer/LPAC tokenとcapability 0を確認した後だけ1回resumeします。検証失敗時はbackend codeを
  実行する前にJobを終了します。2秒間の強制検証失敗でも子stdoutが空である回帰試験を追加しました。
- 正常なtoken検証後の異常終了と、破損PEのloader拒否をisolation reportへ追加し、network、timeout、
  memory、temp、DACLと合わせた7つのprofile/root cleanupを必須化しました。
- 固定Windows Server 2022/2025で通常AppContainerの完全matrixを再実行しました。LPACは両環境で
  `TokenIsLessPrivilegedAppContainer` queryが`ERROR_INVALID_PARAMETER`となり、exact failure class、
  exit code 2、空stdout、backend未実行、完全cleanupを要求してfail closedにしました。
- ファイル関連付け登録が共有`OpenWithProgids`／`RegisteredApplications` keyの既存値を消さないよう修正し、
  18拡張子のunrelated値と保護された`UserChoice` stateを登録2回・解除後も完全保持するWindows試験を追加しました。
- native `windows-11-arm`上で3つのARM64 PE、Rust tests/Clippy、通常AppContainer isolationを検証します。
  ARM64 binaryはこのReleaseへ混在させません。
- 将来Release向けimmutable policy、draft-first upload、公開前後のexact 6-asset name/length/digest検証を
  強化しました。既存Releaseを上書きしません。

実測範囲は [Windows E2E](https://github.com/hjosugi/iroha-zip/blob/v0.4.1/docs/WINDOWS_E2E.md)、
[ARM64 status](https://github.com/hjosugi/iroha-zip/blob/v0.4.1/docs/ARM64.md)、
[Build Status](https://github.com/hjosugi/iroha-zip/blob/v0.4.1/docs/BUILD_STATUS.md)を確認してください。

---

## English

iroha-zip 0.4.1 is a maintenance release that updates the security boundary and regression evidence
for the stable Windows x64 build. Replace `v0.4.0` with this version if you are currently using it.

### Which download to choose

- **Normally choose `iroha-zip-0.4.1-windows-x64.zip`**: the complete package with all three
  executables, setup and association scripts, Japanese and English guides, licenses, and design docs.
- **`iroha-zip-0.4.1-windows-x64.exe`**: the CLI only.
- **`iroha-zip-settings-0.4.1-windows-x64.exe`**: the native Settings application.
- **`iroha-zip-shell-0.4.1-windows-x64.exe`**: the Windows file-association launcher.
- **`SHA256SUMS.txt`**: SHA-256 digests for the ZIP and all three executables.

### Important

- The three executables are **not Authenticode-signed**. Do not disable SmartScreen warnings;
  establish provenance with `SHA256SUMS.txt` and the GitHub artifact attestation.
- libarchive / `bsdtar.exe` is not bundled. Import a backend you trust in Settings and require
  `iroha-zip.exe doctor` to pass.
- This release is Windows x64 only. Native ARM64 CI exists, but its backend, archive matrix, and
  release assets are incomplete.
- This is not a security-audited product. The packaged Build Status precisely records the missing
  Windows 10/11 desktop, format, and race validation.

### Main changes from v0.4.0

- Every sandboxed Windows child is created with `CREATE_SUSPENDED`, assigned to its Job Object, and
  resumed exactly once only after the requested AppContainer/LPAC token and zero capabilities are
  verified. A verification failure terminates the Job before backend code can run. A regression test
  forces a two-second verification failure and requires empty child stdout.
- The isolation report now covers abnormal termination after positive token verification and loader
  rejection of a corrupt PE. Seven network/timeout/memory/crash/loader/temp/DACL profiles and roots
  must all be explicitly removed.
- The complete normal-AppContainer matrix was rerun on fixed Windows Server 2022/2025. LPAC token
  queries returned `ERROR_INVALID_PARAMETER` on both; the harness requires that exact failure class,
  exit code 2, empty stdout, no reported backend execution, and complete cleanup.
- Association registration no longer clears existing values from shared `OpenWithProgids` or
  `RegisteredApplications` keys. A Windows test registers all 18 extensions twice and unregisters,
  while preserving unrelated values and the exact protected `UserChoice` state.
- Native `windows-11-arm` CI validates all three ARM64 PEs, Rust tests/Clippy, and normal-AppContainer
  isolation. No ARM64 binary is mixed into this release.
- Future releases use the repository immutable policy, draft-first upload, and exact six-asset
  name/length/digest verification before and after publication. Existing releases are never overwritten.

See [Windows E2E](https://github.com/hjosugi/iroha-zip/blob/v0.4.1/docs/WINDOWS_E2E.md),
[ARM64 status](https://github.com/hjosugi/iroha-zip/blob/v0.4.1/docs/ARM64.md), and
[Build Status](https://github.com/hjosugi/iroha-zip/blob/v0.4.1/docs/BUILD_STATUS.md) for the measured boundary.
