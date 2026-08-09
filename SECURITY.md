# Security policy

SafeArc v0.1.0はセキュリティ指向の試作実装であり、第三者監査済みではありません。

## Supported versions

現時点では最新の`main`と`0.2.x`を対象とします。`0.1.x`はソースレビュー用の初期版であり、セキュリティ修正の対象外です。

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

## Production release requirements

監査済み・production-readyと表明するリリースの前に最低限、次を満たしてください。`v0.2.x`はこれらを完了するためのpreviewです。

- Windows CIで`cargo test`と`cargo clippy`が成功
- 実際のWindows 11でZIP、7z、RAR、LZH、TAR.GZ、`.Z`を展開
- ZIP、7z、TAR、TAR.GZを作成し、再展開して内容を照合
- パストラバーサル、symlink、hardlink、junction、ADS、ZIP bombの回帰試験
- `Cargo.lock`を含むlocked build
- Rust依存関係と同梱DLLの脆弱性確認
- 生成物のAuthenticode署名
- バックエンド配布元の署名／ハッシュ確認
