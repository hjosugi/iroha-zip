# iroha-zip

[日本語](README.md) | [English](README.en.md) | [公式サイト / Website](https://hjosugi.github.io/iroha-zip/)

iroha-zipは、未信頼の圧縮ファイルをWindows上でできるだけ小さい権限で展開し、検査済みの通常ファイルから書庫を作成するRust製ラッパーです。

目的は「Rustで全書庫形式を再実装する」ことではありません。最新版のlibarchive/`bsdtar.exe`を独立プロセスとして使い、そのプロセスを一時的なAppContainerに閉じ込め、展開前後と圧縮元をRust側で検査します。展開と作成のどちらでも、`bsdtar.exe`を通常ユーザー権限で直接実行しません。

これはセキュリティ監査済み製品ではありません。現段階は、設計を検証しながら実用化するための`v0.6.2`です。

## ダウンロード

[GitHub Releases](https://github.com/hjosugi/iroha-zip/releases/latest) から Windows x64／native ARM64 ZIP、または各architectureの個別EXEをダウンロードできます。現在の公式バイナリは未署名です。`SHA256SUMS.txt`とGitHub release／workflow attestationsで出所を確認してください。詳しい確認手順は[未署名リリースについて](docs/UNSIGNED_RELEASE.md)にあります。

配布物にはlibarchive / `bsdtar.exe`を同梱していません。初回起動後、設定画面から自分が信頼するバックエンドを取り込む必要があります。

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
設定時だけWindows Attachment Servicesへ引き渡し、内容とMotWを再検査
    ↓
書庫の隣に同名フォルダとして公開
```


### 作成時の流れ

```text
圧縮元フォルダを指定
    ↓
リンク、リパースポイント、ADS、ハードリンク、容量を監査
    ↓
監査済みの通常ファイルだけを隔離stagingへコピー
    ↓
全file／directoryのDACLをPackage SIDからread／execute専用に封印
    ↓
信頼側で封印treeを有界PAX streamへ直列化し、identity・長さ・SHA-256を固定
    ↓
SHA-256固定済みbsdtarがsandbox内の固定`@source.pax.tar`だけを書庫形式へ変換
    ↓
7z writerだけは専用scratchを使用し、終了時に空であることを検証して削除
    ↓
生成中の書庫サイズを監視
    ↓
封印したstaging treeが変わっていないことをfingerprintで再確認
    ↓
生成書庫を別のAppContainerでlisting検査・再展開し、元treeと照合
    ↓
検証時と同じidentity・時刻・長さ・SHA-256のhandleから新規出力へコピー
```

圧縮元の監査済みcopy、有界PAX stream、生成書庫の再展開treeを別々の一時領域に置き、7z writerの一時出力も専用領域に限定するため、作成時の追加ディスク容量は保守的に見て圧縮元のおよそ3倍と生成書庫のおよそ2倍が必要です。7z scratchはresource monitorの対象で、backend終了時に空でなければ公開前にfail closedします。これは、侵害されたバックエンドから通常の圧縮元ツリーを切り離し、AppContainerから拒否されるドライブ直下の照会を不要にし、壊れた生成物や残留物を公開前に検出するための意図的なコストです。

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

単体のGZ／BZ2／XZ／Zstandard／`.Z` streamは、外側の拡張子を1つ外した安全な名前の通常fileとして展開します。たとえば`logs.txt.gz`の出力は`logs.txt`です。Windowsではmanifest固定DLLを専用AppContainer子processだけがloadし、事前検査でもstream全体を復号して、拡張子から期待したfilter、raw形式、単一file容量、timeout、memory、process数を確認します。実展開は別の新規AppContainer passで同じ検査を繰り返します。stream内の元filenameは出力名に使わず、拡張子と実データのfilterが違う場合、libarchiveが報告した形式／復号error、容量超過、既存出力は公開せずfail closedになります。圧縮形式は暗号学的な内容真正性を提供しないため、出力内容が信頼済みであるとは扱いません。`tar.gz`などの複合書庫は従来どおり書庫として処理します。

### 作成

- ZIP
- 7z
- TAR
- TAR.GZ

RARとLZHの作成は実装していません。RARは独自形式で、libarchiveは読み取り用途が中心です。LZHもこのバックエンドでは作成対象にしません。

## 文字化け対策

通常は書庫内のフラグとlibarchiveの自動判定を使います。古い日本語ZIP/LZHが文字化けする場合は、CP932を明示できます。

```powershell
iroha-zip.exe extract .\old-japanese.zip --encoding cp932
```

選択肢は次の4つです。

```text
auto
utf8
cp932
cp437
```

ZIPに文字コード情報が正しく保存されていない場合、完全な自動判定は不可能です。そのため手動指定を残しています。ダブルクリック展開で使う既定値は設定画面から選択できます。Windows版libarchive 3.8.6以降にあるUTF-8名の[上流回帰](https://github.com/libarchive/libarchive/issues/3063)に対しては、通常processで`bsdtar -t`の現在code page向け表示を解釈しません。検証済みmanifestから選んだDLLだけを専用のAppContainer子processでloadし、libarchive公式のUTF-8 pathname APIから有界一覧を取得します。子は起動後にもAppContainerかつcapability 0を自己検査します。作成時のZIP／PAXにはUTF-8 header charsetを明示します。検証済みbackendのsandbox copyには固定UTF-8 process code page manifestも埋め込み、resource bytesを実行前に再照合します。取り込んだ原本とDLLは変更しません。

## セキュリティ設計

iroha-zipは次をfail-closedで拒否します。

- AppContainerの作成に失敗した状態での暗黙の展開・作成
- sandbox内の事前一覧で検出した`..`、絶対／drive／UNCプレフィックス
- シンボリックリンク、ジャンクション、その他のリパースポイント
- ハードリンク、重複したファイルID
- 重複した書庫member、大小文字やseparator表記だけが異なるpath alias
- NTFS Alternate Data Stream
- `CON`、`NUL`、`COM1`などのWindows予約名
- 末尾のドット／スペース、コロン、Windowsで無効な文字
- ファイル数、ディレクトリ数、単一ファイル容量、合計展開容量、深さの超過
- SHA-256マニフェストと一致しないバックエンドEXE/DLL
- バックエンドフォルダ内の余分なファイル、欠落ファイル、リンク
- 既存の展開先や既存の出力書庫の上書き
- 単体圧縮streamで、拡張子から期待したfilterと実データが一致しない場合

さらに、Job Objectで子プロセス数を1、メモリ上限を設定し、指定時間を超えた処理を終了します。Windowsの子processはsuspended状態で生成し、要求したAppContainer／LPAC tokenとcapability 0を親が確認した後だけ実行を開始します。照合やresumeに失敗した場合は、backendの実行開始前にJobごと終了します。展開完了後は、一時領域から直接利用せず、検査済みの通常ファイルだけを新しいフォルダへコピーしてからrenameします。Windowsのツリー監査は、rename／delete共有を許さず開いた親directory handleからmember名を有界列挙し、directory identityも監査時とコピー時に照合します。sandboxへコピーしたbackend treeと入力書庫copyは再帰的にread／execute専用へ封印し、入力書庫は保持handleでも固定します。展開先directoryはlisting process終了・一覧検証・入力再照合の後に親が新規作成するため、侵害されたlisting processは書庫差替え、backend自己改変、展開先への置き土産を次のpassへ残せません。作成時は圧縮元を一意な外部staging treeへ監査付きで複製し、全file／directoryのDACLを継承から保護してPackage SIDへread／executeだけを個別付与します。信頼する親プロセスがこのtreeを有界PAX streamへ変換し、backendには通常の圧縮元pathやtree operandを渡さず、sandbox内の固定`@source.pax.tar`だけを渡します。7z writerが必要とする削除時close付き一時fileには、sandbox内の専用scratchだけをread／write／delete可能にし、全使用量を監視し、process終了後に空であることを必須検証して削除します。PAXとstaging treeは保持中のhandleとfingerprintで再照合し、生成書庫は別sandboxへ再展開して元treeと一致するまで公開しません。明示的なunsandboxed検証経路ではDACL封印を行わず、前後fingerprintによる検出だけです。

詳細は[脅威モデル](docs/THREAT_MODEL.md)を参照してください。通常AppContainerと実験的LPACの差、fail-closed条件、未完の検証matrixは[LPAC評価](docs/LPAC_EVALUATION.md)、Windows自動E2Eの証跡項目と限界は[Windows E2E](docs/WINDOWS_E2E.md)、生成型の攻撃書庫と非公開方針は[悪性コーパス](docs/MALICIOUS_CORPUS.md)に分離しています。

## 動作環境

- Windows 10 version 1903以降。x64とnative ARM64を配布し、ARM64自動実機証拠はWindows 11 ARMで取得
- 対象architectureの[Microsoft Visual C++ v14 Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)。公式EXEは`VCRUNTIME140.dll`を使用します。DLL不足時はMicrosoft公式packageを導入し、第三者DLL配布siteから単体fileを取得しないでください
- libarchive 3.8.9系の`bsdtar.exe`と実行に必要なDLL
- PowerShell 5.1以降

ソースからビルドする場合は、追加でRust 1.97.1とVisual Studio Build ToolsのMSVC C++ build toolsが必要です。`rust-toolchain.toml`でRust 1.97.1を固定しています。

## なぜbsdtarをZIPに同梱していないか

第三者が作った実行ファイルを出所不明のまま再配布することを避けるためです。iroha-zipのソースと公式リリースZIPにはバックエンドバイナリを含めません。

ユーザー自身が信頼するlibarchiveビルドを用意し、設定画面の「bundleを取り込む」または「MSYS2から取り込む」を使います。取り込み時にEXEと全DLLのSHA-256マニフェストを生成し、取り込み後に完全検証します。付属スクリプトは自動化用にも残しています。

Windows x64ではMSYS2 UCRT64、Windows ARM64ではMSYS2 CLANGARM64のlibarchiveを利用します。

```powershell
# MSYS2 UCRT64シェルで実行
pacman -S mingw-w64-ucrt-x86_64-libarchive

# native ARM64のMSYS2 CLANGARM64シェルで実行
pacman -S mingw-w64-clang-aarch64-libarchive
```

その後、設定画面で「MSYS2から取り込む」を選び、`C:\msys64`を指定します。設定画面はx64版でUCRT64、ARM64版でCLANGARM64を自動指定します。PowerShellで自動化する場合は次のコマンドでも同じ処理を実行できます。

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\export-msys2-backend.ps1 -Msys2Root C:\msys64

# Windows ARM64
.\scripts\export-msys2-backend.ps1 `
  -Msys2Root C:\msys64 `
  -Environment CLANGARM64
```

exporter内の各MSYS2 commandは既定で180秒に制限されます。mirrorやpackage database、
`ldd`が停止しても外側のCI timeoutまで待ち続けません。低速な管理環境でだけ
`-CommandTimeoutSeconds`を30～1800秒の範囲で明示変更できます。timeout時は未完成の
bundleを公開せず、失敗した境界を表示してfail closedになります。

すでに最小構成のbsdtarフォルダを持っている場合は、設定画面の「bundleを取り込む」から直接取り込めます。PowerShellで自動化する場合は次のとおりです。

```powershell
.\scripts\install-backend.ps1 `
  -SourceDirectory C:\path\to\minimal-bsdtar-bundle `
  -AllowUnsupportedSource
```

任意bundleは未対応の取得元であり、配布元署名を検証できないため、設定画面では専用警告への明示確認、CLIでは`-AllowUnsupportedSource`が必須です。`SourceDirectory`直下またはその配下にある全payloadファイルがバックエンドとして固定されます。不要なEXEやDLLを混ぜないでください。

`backend-manifest.tsv`の形式、入力上限、パス規則、検証範囲は[backend manifest仕様](docs/BACKEND_MANIFEST.md)に記載しています。MSYS2 UCRT64／CLANGARM64の署名必須export、任意bundleの警告、machine-readable provenance、SPDX 2.3 SBOM、license inventory、private packageのfail-closed条件は[backend証跡仕様](docs/BACKEND_EVIDENCE.md)に記載しています。

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
cargo test --features fuzzing --test fuzz_regressions
cargo clippy --all-targets
cargo build --release
```

成功すると、配布用フォルダとZIPが生成されます。

```text
dist\iroha-zip\
   dist\iroha-zip-0.6.2-windows-x64.zip
```

この通常実行はx64 packageを作成します。ARM64をローカルbuildする場合は`-Target aarch64-pc-windows-msvc`を指定します。tag-driven workflowはnative x64／ARM64 runnerで両packageを別々に作成します。公式リリースには2つのZIP、6つの個別EXE、2つのZIP sidecar、全体SHA-256一覧と、GitHub release／workflow attestationsがあります。未署名であること、SmartScreen警告、独立検証手順は[未署名リリースについて](docs/UNSIGNED_RELEASE.md)を参照してください。将来Authenticode署名を有効にするための厳格な検証経路は[リリース検証仕様](docs/RELEASE_VERIFICATION.md)に保持し、最新の独立検証済み公開結果は[v0.6.2 release snapshot](https://github.com/hjosugi/iroha-zip/tree/main/evidence/releases/v0.6.2)へ固定しています。

初回ビルド時に`Cargo.lock`がない場合は生成されます。以後は`Cargo.lock`をバージョン管理し、`--locked`でビルドしてください。

通常の公式ZIPは第三者backendを含めません。信頼済みbackendも含む私的な配布物を作る場合に限り、`-IncludeBackend`を明示できます。

```powershell
.\scripts\build-release.ps1 -IncludeBackend
```

`-IncludeBackend`はsupported sourceの証跡を既定で必須にします。独立に確認済みの未対応bundleを意図的に含める場合だけ、追加の`-AllowUnsupportedBackendSource`も同時に指定します。

## 設定画面と初期設定

配布フォルダで設定画面を開きます。

```powershell
.\iroha-zip.exe settings
# または .\iroha-zip-settings.exe
```

設定画面はWindowsのユーザーUI言語が日本語なら日本語、それ以外なら英語で表示します。サポート時や自動試験では、process環境変数`IROHA_ZIP_LANGUAGE=ja`または`en`で明示できます。この指定は設定ファイルへ保存されません。

設定画面から次のすべてを実行できます。

| 分類 | 設定・操作 |
|---|---|
| Backend | 保存先の選択、未対応bundleの明示警告付き取り込み、MSYS2署名検証付き収集、SHA-256／provenance／SPDX／license inventory検証、AppContainer診断 |
| AppContainer | 通常AppContainer／実験的LPAC、timeout、Job Object memory limit |
| Resource limits | 入力書庫容量、ファイル数、ディレクトリ数、合計容量、単一ファイル容量、パス深さ、パス長 |
| 展開動作 | Mark-of-the-Web伝播、ダブルクリック後に開く、既定のfilename encoding |
| Windows信頼連携 | 無効（既定）／best-effort／必須。Attachment Servicesへの引き渡しでありclean判定ではない |
| Windows統合 | 関連付け候補の登録・解除、既定のアプリ画面、設定フォルダ |
| 設定管理 | 入力検証、既定値復元、rollback-safe保存 |

`--allow-unsandboxed`は管理者が検証時に毎回明示する危険な例外であり、永続設定にはできません。

容量欄は`16 GiB`、`512 MiB`のような読みやすい二進単位で入力できます。入力エラー時は該当欄へフォーカスし、未保存の変更はタイトルの`*`と終了確認で通知します。Tab／Shift+Tab、アクセスキー、Enterで保存、Escapeで閉じる操作にも対応しています。SettingsはPer-Monitor V2 DPI awareで、monitor DPI変更時にWindowsのsuggested rectangle、全control、scroll、system fontを96-DPI基準から再計算します。画面内に収まらない場合は縦横スクロールとフォーカス追従で全項目へ到達できます。バックエンド置換・関連付け解除・既定値復元には確認が入り、診断・取り込み中は状態を画面下部へ表示します。設定保存は同時実行を直列化してからrollback-safeに置換します。実装済みのアクセシビリティ契約と未完の実機matrixは[`SETTINGS_ACCESSIBILITY.md`](docs/SETTINGS_ACCESSIBILITY.md)に記録しています。

設定ファイルは通常、次に作成されます。

```text
%LOCALAPPDATA%\iroha-zip\config.toml
```

設定例は[`config.example.toml`](config.example.toml)です。

Windows信頼連携を有効にすると、公開前のpartialツリーに対して`IAttachmentExecute::Save`を呼び、完了後に通常データのSHA-256、ツリー構造、リンク／reparse point／ADS、Mark-of-the-Webを再検査します。`best-effort`はサービス不在を明示して続行し、`required`は最終フォルダを公開しません。どちらでも内容変更や削除はfail closedです。詳細は[`ANTIMALWARE_HANDOFF.md`](docs/ANTIMALWARE_HANDOFF.md)を参照してください。

CLIだけで初期化・診断する場合は従来のコマンドも使用できます。

```powershell
.\iroha-zip.exe init-config
.\iroha-zip.exe doctor
```

Windowsは既定アプリの変更をユーザー操作で確定する仕組みです。設定画面でiroha-zipを候補として登録し、「既定のアプリを開く」からZIP、7z、RARなどをiroha-zipへ割り当てると、ダブルクリックで同名フォルダへ展開します。

## CLI

### プレビューと選択展開

`preview`は通常展開と同じsandbox・timeout・容量・個数・パス監査で書庫全体を一時展開し、公開せずに監査済みtreeだけを一覧表示します。書庫のlisting文字列をmain processでparseしません。

```powershell
iroha-zip.exe preview .\archive.zip
iroha-zip.exe extract .\archive.zip --select "docs\readme.txt"
iroha-zip.exe extract .\archive.zip --select "写真" --select "資料\index.txt"
```

`--select`はpreviewに表示された相対pathを複数指定できます。選択はbackendへ渡さず、書庫全体の監査後に適用し、選択treeも再監査してから通常のpartial/atomic publish経路へ渡します。安全でない未選択entryがあっても書庫全体を拒否します。詳細は[`ARCHIVE_PREVIEW.md`](docs/ARCHIVE_PREVIEW.md)を参照してください。

### 展開

```powershell
iroha-zip.exe extract .\archive.zip
iroha-zip.exe extract .\archive.zip --encoding cp932
iroha-zip.exe extract .\archive.7z --output D:\Extracted\archive
iroha-zip.exe extract .\archive.tar.gz --open
iroha-zip.exe extract .\encrypted.zip --prompt-password
```

既存の出力先は上書きしません。出力先を省略すると、書庫の隣に衝突しない名前を作ります。

### 暗号化ZIP

`preview`と`extract`に`--prompt-password`を付けると、日英併記のnative dialogで一回だけ
パスワードを入力できます。パスワード値をCLI option、環境変数、設定、fileへ渡す経路はありません。
封印済みの内部抽出子は、検証済みAppContainer内でtokenとcapability 0件を確認した後だけ、明示的な
handle listに含めた匿名pipeから1値を受け取ります。manifest固定済みlibarchive DLLへ値を登録し、
通常file／directory以外を作成前に拒否して展開します。wrong password、timeout、policy違反、cancelは
いずれも出力先を公開しません。

```powershell
iroha-zip.exe preview .\encrypted.zip --prompt-password
iroha-zip.exe extract .\encrypted.zip --prompt-password
```

対象はWindows 10以降と、取り込んだlibarchive buildが読めるZipCrypto／WinZip AESの
暗号化ZIPです。ダブルクリックではパスワード画面を出さず、暗号化書庫の作成、password valueの
command-line指定、unsandboxed password処理には対応しません。詳細なsecret lifetime、fail-closed条件、
検証範囲は[暗号化書庫](docs/ENCRYPTED_ARCHIVES.md)を参照してください。

### 作成

```powershell
iroha-zip.exe create zip .\folder .\folder.zip
iroha-zip.exe create seven-zip .\folder .\folder.7z
iroha-zip.exe create tar .\folder .\folder.tar
iroha-zip.exe create tar-gz .\folder .\folder.tar.gz
```

圧縮元の中に出力書庫を作る操作、リンクやADSを含む圧縮元、既存書庫の上書きは拒否します。

### 診断

```powershell
iroha-zip.exe doctor
```

設定、バックエンドの全ハッシュ、`bsdtar --version`、AppContainer作成可否を確認します。

## 現在の制約

- パスワード入力はCLIの暗号化ZIP `preview`／`extract`だけです。ダブルクリック、暗号化書庫の作成、ZIP以外の暗号化形式、自動retryには対応しません。[暗号化書庫の境界](docs/ENCRYPTED_ARCHIVES.md)を参照してください。
- 自動updaterは未実装です。未署名版から自己更新を有効にせず、署名・downgrade・rollback・backend分離の条件を[署名付きupdater設計](docs/UPDATER.md)で固定しています。
- CLIのpolicy-safe previewと選択展開はありますが、書庫内容を閲覧・検索・選択するネイティブGUIは未実装です。
- ウイルス対策エンジンではありません。展開後の実行ファイルが安全であることは保証しません。
- AppContainerやWindowsカーネル、libarchive自体の未知の脆弱性を防げる保証はありません。
- 既定は通常AppContainerです。実験的LPACは設定画面から選べますが、対象backendで`doctor`が成功した環境だけで使用してください。互換モードへ暗黙に降格しません。
- 同一ユーザー権限をすでに奪取した攻撃者との競合を完全には防げません。
- `v0.6.2`はWindows x64とnative ARM64を別assetで配布します。ARM64の実測範囲と未検証device境界は[ARM64対応状況](docs/ARM64.md)にあります。
- Rust／GitHub Actions／JavaScriptのCodeQL `extended`解析を有効化しています。local pathを扱うdesktop CLIとしての初回233件のsink/source確認と、その後のtest限定alert #234の判定は[CodeQL baseline](docs/CODEQL.md)に記録しています。2026-08-15時点のopen alertは0件です。
- Linuxでの全テスト、Clippy、Windows MSVC targetの型検査に加え、manifest、Windows path、書庫名、Windows command line、設定往復の5つのbounded fuzz targetを実行済みです。schema-v5 Windows E2Eは14追加読取形式、生成型の悪性コーパス、ZipCrypto／AES-128／AES-256の暗号化ZIPをnative Windows 11 ARMとWindows Server 2022/2025 x64の全環境で[Actions run 31875638650](https://github.com/hjosugi/iroha-zip/actions/runs/31875638650)により合格しました。Server 2022/2025では全26 controlsの正逆Tab巡回、Enter保存、Escape終了要求も実key入力で合格し、hosted ARM64の限定fallbackは実key入力ではないことを証跡内で明示しています。11 JSONはraw/canonical SHA-256とartifact API digest付きの[長期証跡snapshot](https://github.com/hjosugi/iroha-zip/tree/main/evidence/windows/31875638650)として保存しています。ただし、これはWindows 10/11 x64 desktop実機検証やセキュリティ監査の代替ではありません。再現可能な定期fuzzingは[`docs/FUZZING.md`](docs/FUZZING.md)、E2Eの正確な範囲は[`docs/WINDOWS_E2E.md`](docs/WINDOWS_E2E.md)、コーパス範囲は[`docs/MALICIOUS_CORPUS.md`](docs/MALICIOUS_CORPUS.md)、全体状況は[`docs/BUILD_STATUS.md`](docs/BUILD_STATUS.md)に記録しています。

残作業は、優先度・依存関係・受け入れ条件を付けた[`docs/ISSUE_BACKLOG.md`](docs/ISSUE_BACKLOG.md)で追跡します。変更を提案する場合は[`CONTRIBUTING.md`](CONTRIBUTING.md)も確認してください。

## ライセンス

iroha-zip本体はMITまたはApache-2.0のデュアルライセンスです。

取り込むlibarchiveおよびDLLのライセンスは、それぞれの配布元に従います。バイナリを第三者へ再配布する場合は、依存DLLを含むライセンス表示を別途確認してください。
