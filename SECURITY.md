# Security policy

iroha-zip v0.3.1はセキュリティ指向のpreview実装であり、第三者監査済みではありません。

## Supported versions

現時点では最新の`main`と`0.3.x`を対象とします。`0.1.x`と`0.2.x`はソースレビュー用の旧previewであり、セキュリティ修正の対象外です。

## Reporting a vulnerability

[GitHub Security Advisories](https://github.com/hjosugi/iroha-zip/security/advisories/new)から、脆弱性の再現手順、影響、対象バージョンを非公開で報告してください。

公開Issueへ、実際に悪用可能な書庫や未修正の詳細を直接添付しないでください。

## Security-sensitive files

変更時に特に注意が必要です。

- `src/platform/windows_impl.rs`
- `src/policy.rs`
- `src/backend.rs`
- `src/extract.rs`
- `src/create.rs`
- `src/monitor.rs`
- `src/transfer.rs`
- `scripts/install-backend.ps1`
- `scripts/export-msys2-backend.ps1`
- `scripts/build-release.ps1`
- `scripts/verify-release-signatures.ps1`
- `.github/workflows/release.yml`

## Production release requirements

監査済み・production-readyと表明するリリースの前に最低限、次を満たしてください。`v0.3.x`はこれらを完了するためのpreviewです。

- Windows CIで`cargo test`と`cargo clippy`が成功
- 実際のWindows 11でZIP、7z、RAR、LZH、TAR.GZ、`.Z`を展開
- ZIP、7z、TAR、TAR.GZを作成し、再展開して内容を照合
- パストラバーサル、symlink、hardlink、junction、ADS、ZIP bombの回帰試験
- `Cargo.lock`を含むlocked build
- Rust依存関係と同梱DLLの脆弱性確認
- 3つのEXEのAuthenticode署名、publisher/EKU/timestamp検証、独立に検証可能なSLSA provenance、GitHub immutable release
- バックエンド配布元の署名／ハッシュ確認

正式リリースの証明書custody、GitHub/Azure設定、asset契約、利用者側の検証手順は[`docs/RELEASE_VERIFICATION.md`](docs/RELEASE_VERIFICATION.md)を参照してください。
