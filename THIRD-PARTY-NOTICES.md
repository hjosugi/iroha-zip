# Third-party notices

iroha-zipのソース配布には、libarchive、`bsdtar.exe`、MSYS2 DLL、その他の第三者バイナリを含めていません。

## Rust dependencies

`Cargo.toml`で直接利用する主なcrateは次のとおりです。

- `clap`
- `serde`
- `sha2`
- `toml`
- `windows`（Windowsビルドのみ）

これらと推移的依存関係には、それぞれのライセンスが適用されます。実際の依存グラフは`Cargo.lock`で固定し、`cargo-deny`で許可ライセンス、脆弱性、取得元を検査しています。

依存crate名、バージョン、適用ライセンス、ライセンス本文は[`THIRD-PARTY-LICENSES.html`](THIRD-PARTY-LICENSES.html)に収録しています。このHTMLは`cargo-about 0.9.1`で次のように再生成できます。

```text
cargo about generate --locked --all-features -o THIRD-PARTY-LICENSES.html about.hbs
```

生成物はリポジトリの改行規則に合わせてLFへ正規化しています。ライセンス本文そのものは変更していません。

## Archive backend

iroha-zipは、利用者が別途用意したlibarchiveの`bsdtar.exe`と依存DLLを実行します。バックエンドを第三者へ再配布する場合、libarchive本体だけでなく、圧縮・文字コード・暗号・ランタイム関連DLLを含む全ファイルのライセンス、著作権表示、ソース提供条件を配布元ごとに確認してください。

`scripts/export-msys2-backend.ps1`は実行に必要なUCRT64 DLLを収集しますが、ライセンス文書までは自動収集しません。

## No endorsement

iroha-zipはlibarchive、MSYS2、Microsoft、Rust Projectの公式製品ではありません。各名称は識別のために使用しています。
