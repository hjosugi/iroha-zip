# SafeArc

SafeArcは、未信頼の圧縮ファイルをWindows上でできるだけ小さい権限で展開し、検査済みの通常ファイルから書庫を作成するRust製ラッパーです。

目的は「Rustで全書庫形式を再実装する」ことではありません。最新版のlibarchive/`bsdtar.exe`を独立プロセスとして使い、そのプロセスを一時的なAppContainerに閉じ込め、展開前後と圧縮元をRust側で検査します。展開と作成のどちらでも、`bsdtar.exe`を通常ユーザー権限で直接実行しません。

これはセキュリティ監査済み製品ではありません。現段階は、設計を検証しながら実用化するための`v0.2.0`です。

## 主な動作

```text
archive.zipをダブルクリック
    ↓
一時AppContainerを作成（capabilityなし、ネットワークなし）
    ↓
SHA-256マニフェストで固定したbsdtar一式をコピー
    ↓
AppContainer内で書庫を一時領域へ展開
    ↓
ファイル数・ディレクトリ数・容量を展開中も監視
    ↓
リパースポイント、リンク、ADS、予約名などを再検査
    ↓
Mark-of-the-Webを各ファイルへ伝播
    ↓
書庫の隣に同名フォルダとして公開
```


### 作成時の流れ

```text
圧縮元フォルダを指定
    ↓
リンク、リパースポイント、ADS、ハードリンク、容量を監査
    ↓
監査済みの通常ファイルだけをAppContainer領域へコピー
    ↓
SHA-256固定済みbsdtarをAppContainer内で実行
    ↓
生成中の書庫サイズを監視
    ↓
新規ファイルとして最終出力先へコピー（既存書庫は上書きしない）
```

圧縮元を丸ごと一時領域へ複製するため、作成時には元データと同程度の追加ディスク容量が必要です。これは、侵害されたバックエンドから通常の圧縮元ツリーを切り離すための意図的なコストです。

## 対応形式

### 展開

libarchive/bsdtarが読み取れる形式を自動判定します。主な対象は次です。

- ZIP / ZIPX
- 7z
- LHA / LZH
- RAR / RAR5
- TAR
- GZ / BZ2 / XZ / Zstandard
- UNIX compress `.Z`
- CABなど、使用するlibarchiveビルドが有効にしている形式

### 作成

- ZIP
- 7z
- TAR
- TAR.GZ

RARとLZHの作成は実装していません。RARは独自形式で、libarchiveは読み取り用途が中心です。LZHもこのバックエンドでは作成対象にしません。

## 文字化け対策

通常は書庫内のフラグとlibarchiveの自動判定を使います。古い日本語ZIP/LZHが文字化けする場合は、CP932を明示できます。

```powershell
safearc.exe extract .\old-japanese.zip --encoding cp932
```

選択肢は次の4つです。

```text
auto
utf8
cp932
cp437
```

ZIPに文字コード情報が正しく保存されていない場合、完全な自動判定は不可能です。そのため手動指定を残しています。ダブルクリック展開で使う既定値は設定画面から選択できます。

## セキュリティ設計

SafeArcは次をfail-closedで拒否します。

- AppContainerの作成に失敗した状態での暗黙の展開・作成
- `..`、絶対パス、Windowsドライブプレフィックス
- シンボリックリンク、ジャンクション、その他のリパースポイント
- ハードリンク、重複したファイルID
- NTFS Alternate Data Stream
- `CON`、`NUL`、`COM1`などのWindows予約名
- 末尾のドット／スペース、コロン、Windowsで無効な文字
- ファイル数、ディレクトリ数、単一ファイル容量、合計展開容量、深さの超過
- SHA-256マニフェストと一致しないバックエンドEXE/DLL
- バックエンドフォルダ内の余分なファイル、欠落ファイル、リンク
- 既存の展開先や既存の出力書庫の上書き

さらに、Job Objectで子プロセス数を1、メモリ上限を設定し、指定時間を超えた処理を終了します。展開完了後は、一時領域から直接利用せず、検査済みの通常ファイルだけを新しいフォルダへコピーしてからrenameします。作成時も圧縮元を監査・複製してからAppContainer内で処理します。

詳細は[脅威モデル](docs/THREAT_MODEL.md)を参照してください。

## 必要環境

- Windows 10以降。通常利用はWindows 11 x64を想定
- Rust 1.97.1
- Visual Studio Build ToolsのMSVC C++ build tools
- libarchive 3.8.9系の`bsdtar.exe`と実行に必要なDLL
- PowerShell 5.1以降

`rust-toolchain.toml`でRust 1.97.1を固定しています。

## なぜbsdtarをZIPに同梱していないか

第三者が作った実行ファイルを出所不明のまま再配布することを避けるためです。SafeArcのソースと公式リリースZIPにはバックエンドバイナリを含めません。

ユーザー自身が信頼するlibarchiveビルドを用意し、設定画面の「bundleを取り込む」または「MSYS2から取り込む」を使います。取り込み時にEXEと全DLLのSHA-256マニフェストを生成し、取り込み後に完全検証します。付属スクリプトは自動化用にも残しています。

MSYS2 UCRT64のlibarchiveを利用する場合の例です。

```powershell
# MSYS2 UCRT64シェルで実行
pacman -S mingw-w64-ucrt-x86_64-libarchive
```

その後、設定画面で「MSYS2から取り込む」を選び、`C:\msys64`を指定します。PowerShellで自動化する場合は次のコマンドでも同じ処理を実行できます。

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\export-msys2-backend.ps1 -Msys2Root C:\msys64
```

すでに最小構成のbsdtarフォルダを持っている場合は、設定画面の「bundleを取り込む」から直接取り込めます。PowerShellで自動化する場合は次のとおりです。

```powershell
.\scripts\install-backend.ps1 -SourceDirectory C:\path\to\minimal-bsdtar-bundle
```

`SourceDirectory`直下またはその配下にある全ファイルがバックエンドとして固定されます。不要なEXEやDLLを混ぜないでください。

## ビルド

Developer PowerShell for VS 2022、またはMSVC環境が利用できるPowerShellで実行します。

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\build-release.ps1
```

スクリプトは次を実行します。

```text
   cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets
cargo build --release
```

成功すると、配布用フォルダとZIPが生成されます。

```text
dist\SafeArc\
   dist\SafeArc-0.2.0-windows-x64.zip
```

初回ビルド時に`Cargo.lock`がない場合は生成されます。以後は`Cargo.lock`をバージョン管理し、`--locked`でビルドしてください。

通常の公式ZIPは第三者backendを含めません。信頼済みbackendも含む私的な配布物を作る場合に限り、`-IncludeBackend`を明示できます。

```powershell
.\scripts\build-release.ps1 -IncludeBackend
```

## 設定画面と初期設定

配布フォルダで設定画面を開きます。

```powershell
.\safearc.exe settings
# または .\safearc-settings.exe
```

設定画面から次のすべてを実行できます。

| 分類 | 設定・操作 |
|---|---|
| Backend | 保存先の選択、既存bundleの取り込み、MSYS2からの収集、SHA-256検証、AppContainer診断 |
| AppContainer | timeout、Job Object memory limit |
| Resource limits | 入力書庫容量、ファイル数、ディレクトリ数、合計容量、単一ファイル容量、パス深さ、パス長 |
| 展開動作 | Mark-of-the-Web伝播、ダブルクリック後に開く、既定のfilename encoding |
| Windows統合 | 関連付け候補の登録・解除、既定のアプリ画面、設定フォルダ |
| 設定管理 | 入力検証、既定値復元、rollback-safe保存 |

`--allow-unsandboxed`は管理者が検証時に毎回明示する危険な例外であり、永続設定にはできません。

容量欄は`16 GiB`、`512 MiB`のような読みやすい二進単位で入力できます。入力エラー時は該当欄へフォーカスし、未保存の変更はタイトルの`*`と終了確認で通知します。Tab／Shift+Tab、アクセスキー、Enterで保存、Escapeで閉じる操作にも対応しています。バックエンド置換・関連付け解除・既定値復元には確認が入り、診断・取り込み中は状態を画面下部へ表示します。

設定ファイルは通常、次に作成されます。

```text
%LOCALAPPDATA%\SafeArc\config.toml
```

設定例は[`config.example.toml`](config.example.toml)です。

CLIだけで初期化・診断する場合は従来のコマンドも使用できます。

```powershell
.\safearc.exe init-config
.\safearc.exe doctor
```

Windowsは既定アプリの変更をユーザー操作で確定する仕組みです。設定画面でSafeArcを候補として登録し、「既定のアプリを開く」からZIP、7z、RARなどをSafeArcへ割り当てると、ダブルクリックで同名フォルダへ展開します。

## CLI

### 展開

```powershell
safearc.exe extract .\archive.zip
safearc.exe extract .\archive.zip --encoding cp932
safearc.exe extract .\archive.7z --output D:\Extracted\archive
safearc.exe extract .\archive.tar.gz --open
```

既存の出力先は上書きしません。出力先を省略すると、書庫の隣に衝突しない名前を作ります。

### 作成

```powershell
safearc.exe create zip .\folder .\folder.zip
safearc.exe create seven-zip .\folder .\folder.7z
safearc.exe create tar .\folder .\folder.tar
safearc.exe create tar-gz .\folder .\folder.tar.gz
```

圧縮元の中に出力書庫を作る操作、リンクやADSを含む圧縮元、既存書庫の上書きは拒否します。

### 診断

```powershell
safearc.exe doctor
```

設定、バックエンドの全ハッシュ、`bsdtar --version`、AppContainer作成可否を確認します。

## 現在の制約

- パスワード付き書庫は未対応です。パスワードをコマンドラインへ露出させない入力経路を設計してから追加します。
- 書庫内容を閲覧するファイラーUIはありません。v0.2は「ダブルクリックで安全側に即展開」が中心です。
- ウイルス対策エンジンではありません。展開後の実行ファイルが安全であることは保証しません。
- AppContainerやWindowsカーネル、libarchive自体の未知の脆弱性を防げる保証はありません。
- 通常のAppContainerを使用しており、LPACではありません。
- 同一ユーザー権限をすでに奪取した攻撃者との競合を完全には防げません。
- Linuxでの全テスト、Clippy、Windows MSVC targetの型検査は実行済みです。実際のWindowsカーネルを使うAppContainer統合試験と実書庫corpus試験の状況は[`docs/BUILD_STATUS.md`](docs/BUILD_STATUS.md)に記録しています。

残作業は、優先度・依存関係・受け入れ条件を付けた[`docs/ISSUE_BACKLOG.md`](docs/ISSUE_BACKLOG.md)で追跡します。変更を提案する場合は[`CONTRIBUTING.md`](CONTRIBUTING.md)も確認してください。

## ライセンス

SafeArc本体はMITまたはApache-2.0のデュアルライセンスです。

取り込むlibarchiveおよびDLLのライセンスは、それぞれの配布元に従います。バイナリを第三者へ再配布する場合は、依存DLLを含むライセンス表示を別途確認してください。
