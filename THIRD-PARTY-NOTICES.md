# Third-party notices / 第三者に関する表示

## 日本語

iroha-zipの公式release packageには、libarchive、`bsdtar.exe`、MSYS2 DLL、その他の第三者
backend binaryを含めていません。source treeには、下記の試験専用fixtureを含みます。

### Rust dependencies

`Cargo.toml`で直接利用する主なcrateは次のとおりです。

- `clap`
- `serde`
- `serde_json`
- `sha1`
- `sha2`
- `toml`
- `libc`（Unix上の検証buildのみ）
- `windows`（Windowsビルドのみ）

これらと推移的依存関係には、それぞれのライセンスが適用されます。実際の依存グラフは`Cargo.lock`で固定し、`cargo-deny`で許可ライセンス、脆弱性、取得元を検査しています。

依存crate名、バージョン、適用ライセンス、ライセンス本文は[`THIRD-PARTY-LICENSES.html`](THIRD-PARTY-LICENSES.html)に収録しています。このHTMLは`cargo-about 0.9.1`で次のように再生成できます。

```text
cargo about generate --locked --all-features -o THIRD-PARTY-LICENSES.html about.hbs
```

生成物はリポジトリの改行規則に合わせてLFへ正規化しています。ライセンス本文そのものは変更していません。

### Archive backend

iroha-zipは、利用者が別途用意したlibarchiveの`bsdtar.exe`と依存DLLを実行します。バックエンドを第三者へ再配布する場合、libarchive本体だけでなく、圧縮・文字コード・暗号・ランタイム関連DLLを含む全ファイルのライセンス、著作権表示、ソース提供条件を配布元ごとに確認してください。

`scripts/export-msys2-backend.ps1`は実行に必要なUCRT64／CLANGARM64 DLLを収集しますが、ライセンス文書までは自動収集しません。

### libarchive試験fixture

source treeとE2E harness付きrelease packageの`tests/fixtures/libarchive-v3.8.9/`には、公式
libarchive `v3.8.9`から取得したRAR、RAR5、LHA、ZIPXのUUencoded read fixtureを4つ含みます。
これはbenign test dataであり、backend EXE／DLLではありません。exact tag／commit、
encoded／decoded SHA-256、expected tree、upstream BSD-2-Clause条件は同directoryの
`README.md`と`COPYING`に記録しています。

### No endorsement

iroha-zipはlibarchive、MSYS2、Microsoft、Rust Projectの公式製品ではありません。各名称は識別のために使用しています。

## English

Official iroha-zip release packages do not include libarchive, `bsdtar.exe`, MSYS2 DLLs, or other
third-party backend binaries. The source tree contains only the test fixtures described below.

### Rust dependencies

The primary direct dependencies in `Cargo.toml` are:

- `clap`
- `serde`
- `serde_json`
- `sha1`
- `sha2`
- `toml`
- `libc` (Unix validation builds only)
- `windows` (Windows builds only)

Those crates and their transitive dependencies remain under their respective licenses. `Cargo.lock`
pins the resolved graph, and `cargo-deny` checks allowed licenses, advisories, and sources.

[`THIRD-PARTY-LICENSES.html`](THIRD-PARTY-LICENSES.html) contains dependency names, versions,
declared licenses, and license text. It is generated with `cargo-about 0.9.1` using the command shown
in the Japanese section. Line endings are normalized to LF without modifying the license text.

### Archive backend

iroha-zip runs a user-supplied libarchive `bsdtar.exe` and its dependent DLLs. Anyone redistributing
a backend must review the distributor-specific licenses, notices, and source obligations for every
file, including compression, character-conversion, cryptography, and runtime DLLs.

`scripts/export-msys2-backend.ps1` collects the UCRT64 or CLANGARM64 runtime payload but does not
automatically collect license documents.

### libarchive test fixtures

`tests/fixtures/libarchive-v3.8.9/` in the source tree and E2E-harness release package contains four
UUencoded RAR, RAR5, LHA, and ZIPX read fixtures copied from official libarchive `v3.8.9`. They are
benign test data, not backend executables or DLLs. The exact tag/commit, encoded and decoded SHA-256
values, expected trees, and upstream BSD-2-Clause terms are recorded in that directory's `README.md`
and `COPYING`.

### No endorsement

iroha-zip is not an official product of libarchive, MSYS2, Microsoft, or the Rust Project. Their
names are used only for identification.
