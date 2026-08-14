# iroha-zip threat model

## 1. Protection goal

iroha-zipの中心目標は、攻撃者が作成した書庫を展開するときに、書庫パーサーの侵害や危険なパス構造がユーザーの通常データへ直接到達する可能性を下げることです。書庫作成時も、バックエンドが通常の圧縮元ツリーを直接読まないようにします。

保護対象は次です。

- ユーザーのドキュメント、ソースコード、SSH鍵、ブラウザプロファイル
- 通常ユーザーが利用できるレジストリや資格情報
- 既存ファイルと既存ディレクトリ
- Windows上のネットワーク資源
- iroha-zipが公開する最終出力先の整合性

## 2. 信頼境界

### 信頼するもの

- WindowsカーネルとAppContainer実装
- iroha-zipの実行ファイル
- iroha-zipの設定ファイル
- `backend-manifest.tsv`
- マニフェストで固定されたbsdtarとDLL
- ユーザーが指定した圧縮元／出力先

### 信頼しないもの

- 入力書庫の全バイト
- 書庫内のパス、サイズ、属性、リンク情報
- 書庫の拡張子
- 書庫に含まれる実行ファイルやスクリプト
- libarchiveパーサーが未信頼データを解析している間のプロセス

## 3. 主な攻撃と対策

| 攻撃 | 対策 |
|---|---|
| libarchiveのメモリ破壊からコード実行 | capabilityなしの一時AppContainerでbsdtarを起動。検証環境ではLPACを明示選択可能 |
| パーサーからネットワークへ接続 | network capabilityを付与しない |
| ユーザーファイルや資格情報の読み取り | 入力とバックエンドをAppContainer専用領域へコピーし、通常データへのACLを付与しない |
| 子プロセスによる回避 | Job Objectのactive process limitを1に設定 |
| 長時間処理 | timeout後にJob Object全体を終了 |
| メモリ枯渇 | Job Objectのメモリ上限 |
| ZIP bomb | 展開中と展開後のファイル数・ディレクトリ数・容量制限 |
| Zip Slip / `../` | bsdtarの安全オプションに加え、公開前にRust側で相対パスを再検査 |
| 絶対パス／UNC／ドライブパス | 正常な相対コンポーネント以外を拒否 |
| シンボリックリンク／ジャンクション | symlinkとWindows reparse pointを拒否 |
| ハードリンク | link countとファイルID重複を拒否 |
| NTFS ADS | 展開後ツリーのnamed streamを拒否 |
| Windows予約名や末尾ドット | 各パスコンポーネントをWindows規則で検査 |
| preview表示のmetadata/control文字注入 | 書庫listing文字列をparseせず、完全展開・監査済みfilesystemから型付きentryだけを生成 |
| 選択pathによるoption注入・監査迂回 | selectorをbackendへ渡さず、書庫全体監査後のhandle保持copyと選択tree再監査にだけ使用 |
| バックエンド差し替え／pass間の自己改変 | EXEと全DLLをSHA-256マニフェストで固定し、コピー後に再ハッシュして、sandbox copy全体をPackage SIDからread／execute専用に再帰封印 |
| listing passからextract passへのstate汚染 | sandbox入力書庫を保持handle＋再帰DACLで固定して両pass後にfingerprint再照合し、展開先はlisting子終了・policy通過後にだけ親が`create_new`相当で作成 |
| DLL横取り | バックエンドディレクトリの余分なファイルを拒否し、最小PATHで起動 |
| 既存データ上書き | `create_new`、`-k`、既存出力拒否、最終rename |
| 作成backendから通常の圧縮元treeへアクセス | 親プロセスが通常objectだけを一意な外部stagingへ監査付きcopy・DACL封印し、そこから作った有界PAX streamの固定sandbox-local名だけを渡す |
| Windows版libarchiveのUTF-8名回帰 | manifest固定済みDLL候補だけをzero-capability AppContainer子でloadし、公式UTF-8 pathname APIから有界一覧を得る。ZIP／PAX作成時はUTF-8 headerを明示し、sandbox EXEの固定UTF-8 process manifestもbyte再照合する |
| インターネット由来属性の消失 | 入力書庫のZone.Identifierを正規化し、公開する通常ファイルへ再付与 |

## 4. 展開処理の流れ

```text
通常プロセス
  1. 入力を書庫としてではなく通常ファイルとして検査
  2. バックエンドの全ファイルを検証
  3. capabilityなしの選択済みAppContainerモードを作成
  4. 入力とバックエンドをAppContainer領域へコピー

AppContainerプロセス
  5. bsdtarが書庫を専用outputへ展開
  6. 親プロセスが容量・個数を監視

通常プロセス
  7. outputツリーを再帰監査
  8. preview時は監査済みtreeをfingerprintして一覧化し、再fingerprint後に公開せず削除
  9. 選択展開時は監査済みtreeから選択treeを作り、source/selection双方を再fingerprint
 10. 通常ファイル／ディレクトリだけをpartialへコピー
 11. Mark-of-the-Webを付与
 12. 設定時はWindows Attachment Servicesへ引き渡す
 13. 内容・ツリー・リンク・ADS・Mark-of-the-Webを再監査
 14. 新規名へrenameして公開
 15. AppContainer profileと一時データを削除
```


## 5. 作成処理の流れ

```text
通常プロセス
  1. 圧縮元ツリーを再帰監査
  2. capabilityなしの選択済みAppContainerモードを作成
  3. 検査済みの通常ファイルだけを外部staging treeへコピー
  4. copy後の完全tree fingerprintを元sourceと照合
  5. backend原本をSHA-256再検証してsandboxへcopy
  6. Windowsではsandbox EXEへ固定UTF-8 process manifestを埋め込み、resourceをbyte単位で再照合
  7. backend treeとstaging root／全childを、Package SIDからread／execute専用に再帰封印
  8. staging treeを有界PAX streamへ直列化し、保持handleでidentity・長さ・SHA-256を固定

AppContainerプロセス
  9. backendが固定sandbox-local `@source.pax.tar`だけを専用output形式へ変換
 10. 親プロセスが出力サイズとオブジェクト数を監視

通常プロセス
 11. staging treeとPAX streamのfingerprintを再照合
 12. 生成書庫をidentity・時刻・長さ・SHA-256付きhandleで固定
 13. 別AppContainerへhandleからcopyし、manifest固定DLLのUTF-8 APIでraw listingを事前検査して再展開
 14. 再展開した完全rootと元sourceのtree fingerprintを照合
 15. staging tree、PAX stream、生成書庫を再照合
 16. 同じ生成書庫handleからcreate-newで最終出力へcopy
 17. 両AppContainer profileと一時データを削除
```

## 6. 残るリスク

### AppContainer escape

WindowsカーネルやAppContainerの脆弱性は本プロジェクトだけでは防げません。既定は互換性を確認済みの通常AppContainerで、設定画面から実験的LPACを明示選択できます。どちらもcapabilityは0件で、ネットワークcapabilityを付けません。

通常AppContainerは、そのPackage SID／capability SIDに明示されたアクセスに加え、Windowsの多くのシステム資源が`ALL APPLICATION PACKAGES`へ与えているアクセスを利用できます。LPACは`PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT`を指定するため、その暗黙アクセスを利用できず、通常AppContainerより狭い境界になります。iroha-zipはLPACで`registryRead`、`lpacCom`などの補完capabilityを追加しません。

LPAC指定時はprocess attributeを付けた後、生成した子プロセスの`TokenIsAppContainer`と`TokenIsLessPrivilegedAppContainer`を検査します。属性設定、process生成、token照合のいずれかが失敗しても通常AppContainerとして再試行しません。OS build番号による推測ではなくruntime検査でfail closedにします。`--allow-unsandboxed`は利用者がコマンドごとに明示した場合だけ存在する別の危険な診断経路です。

対象のlibarchive bundleがLPACで必要形式を処理できるか、通常AppContainerが持つACL差分のどれを実際に必要とするかはWindows実機matrixで未検証です。設定画面の診断は、選択したモードで`bsdtar --version`を実行してtoken照合も通過した場合だけ成功します。詳細は[`LPAC_EVALUATION.md`](LPAC_EVALUATION.md)で追跡します。

### 悪意ある展開後ファイル

展開成功は安全判定ではありません。`.exe`、`.lnk`、`.js`、Office文書などをユーザーが開けば、そのファイル固有の危険があります。Mark-of-the-Webを保つ理由は、Windowsや各アプリの警告・保護機能を継続させるためです。

`IAttachmentExecute::Save`によるWindows信頼連携もclean判定ではありません。Windowsはvirus scanner等を呼ぶ可能性があるとだけ規定しており、実際のprovider実行を成功値から証明できません。連携は既定で無効です。best-effort時のサービス失敗は明示して公開を続け、required時は公開しません。いずれも連携後の削除、通常データ変更、tree変更、reparse point、hardlink、予期しないADS、MotW消失を検出した場合はpartial全体を破棄します。詳細は[`ANTIMALWARE_HANDOFF.md`](ANTIMALWARE_HANDOFF.md)で追跡します。

### 同一ユーザーの能動的攻撃者

すでに同一ユーザー権限で動く攻撃者は、設定やマニフェストを変更できます。コード署名、インストールディレクトリACL、更新署名は今後の課題です。

### 供給網

SHA-256マニフェストは「取り込み後の変更」を検出しますが、最初から悪意あるbsdtarを取り込んだ場合は防げません。取得元の署名、パッケージ署名、ハッシュを利用者が確認する必要があります。

### TOCTOU

入力書庫と圧縮元の通常ファイルは、検査からコピー完了まで同じファイルハンドルを維持します。ハンドルから取得したファイルidentity、長さ、作成・更新時刻、SHA-256をコピー前後で照合し、コピー先も保持中のハンドルから再ハッシュします。Windowsでは読み取り共有だけを許可して書き込み・削除・renameをブロックし、`FILE_FLAG_OPEN_REPARSE_POINT`で開いたうえでreparse pointと複数リンクを拒否します。Unixでは`O_NOFOLLOW`で開き、複数リンクを拒否します。

圧縮元ツリーは相対パス、種別、長さ、各ファイルのSHA-256を決定的にfingerprintします。実コピー時には各ファイルのidentity・時刻・長さ・内容を監査時の値と照合し、コピー後のツリーfingerprintも再比較します。同一サイズの改変、同じ内容を持つ別ファイルへの置換、rename、hardlink、symlink、およびroot外へ解決されるファイルはfail closedになります。

作成backendへ通常の圧縮元pathは渡しません。信頼する親プロセスが、通常file／directoryだけを一意な外部staging treeへ監査付きでcopyし、path深さ・長さ、file数、directory数、単一file size、合計sizeとtree fingerprintを元sourceに対して再検査します。全entryをDACL封印した後、親が決定的で有界なPAX streamへ直列化します。各fileは保持中の監査handleからcopyされ、PAX自体もidentity・長さ・SHA-256付きhandleでbackend実行中まで固定します。backend processには通常source、staging tree operand、絶対pathを渡さず、sandbox rootをcurrent directoryとして固定`@source.pax.tar`だけを渡します。これにより、Windows版libarchiveのdisk readerがAppContainerから許可されないvolume rootへ`GetDiskFreeSpaceW`を行う経路を使用しません。以前の日本語名での`@archive` access violationは、sandbox EXEへ固定UTF-8 process manifestを適用する現在の互換境界で回避します。作成終了後はstaging treeとPAX streamを再照合し、生成書庫をhandleから別sandboxへ渡します。libarchiveが付ける単一の`./` prefix、Windows native表記の単一`.\` prefix、または同じroot markerだけを作成物専用のraw listing policyで正規化し、二重prefix、その他のbackslash形式、親参照、絶対pathなどは拒否します。外部書庫のpolicyはこの例外を持ちません。再展開した完全rootがsourceと一致し、さらに書庫identity・時刻・長さ・SHA-256が検証時と一致するhandleからだけ最終出力へcopyします。内容不一致、同一サイズ改変、identity置換、危険なlistingはいずれも出力前にfail closedになります。

Windowsの作成経路では、AppContainerが本来書き込めるPackage profile storageからstaging sourceを分離し、通常の一時領域へ複製します。AppContainerにはその一意な親directoryへの非継承read accessだけを用意します。親が完全監査・fingerprint照合した後、rootを含む全file／directoryの既存DACLを継承から個別に保護し、Package SIDにはread／execute専用ACEだけを設定します。file data／append／EA／attribute書込、child削除、delete、DACL変更、owner変更は与えません。通常ユーザー側のallow ACEは維持するため親プロセスはPAX生成、監査、cleanupを続けられます。MicrosoftのAppContainer dual-principal modelどおり、ユーザー側が許可されていてもPackage SID側にread／executeしか許可しないことでcreate passの実効書込権限を止めます。同じ再帰封印を各sandbox内のbackend EXE／DLL／directoryにも適用し、listing processがbackend bytesや補助DLLを変更して後続のextract passへ残すことを予防します。実行ファイルを同じAppContainerへbyte-identical copyしたprobeが親／root／nestedの列挙、root／nested内容の読取成功と、overwrite、append、親／rootでの作成、rename、delete、attribute、DACL、owner各write accessの拒否を測定します。API根拠は[AppContainer isolation](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation)、[`SetEntriesInAclW`](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-setentriesinaclw)、[Launch an AppContainer](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer)です。明示的unsandboxed経路ではWindows DACL封印を適用せず、前後fingerprint検査で変更を検出します。

MSYS2が配布するlibarchive 3.8.9の`bsdtar.exe`は長path manifestを持ちますが、Windows版libarchive 3.8.6以降にはUTF-8 member名を現在の非UTF-8 code pageへ損失なく変換できず拒否する[上流回帰](https://github.com/libarchive/libarchive/issues/3063)があります。iroha-zipのWindows事前一覧は`bsdtar -t`のnarrow文字列へ依存しません。親は完全検証済みmanifestからDLL pathだけを有界fileへ出力し、byte-identicalな専用子EXEとともにread／execute専用へ封印します。子は起動直後に自身がAppContainerかつcapability 0であることを再検査し、候補DLLをそのdirectoryとSystem32だけの探索でloadして、`archive_read_open_filename_w`と`archive_entry_pathname_utf8`からUTF-8名を取得します。候補数、一覧file、member数、1 path、標準出力、時間、memory、process数はすべて上限付きです。作成側はZIP／PAX header charsetをUTF-8へ固定します。

さらにiroha-zipはbundle原本と全DLLをSHA-256で検証し、sandbox copyもcopy直後に同じhashで照合した後、その一時backend EXEのresource tableだけを`BeginUpdateResourceW`／`UpdateResourceW`で固定manifestへ置換します。manifestは`asInvoker`、long-path aware、UTF-8 `activeCodePage`だけを宣言し、`LoadLibraryExW`／`FindResourceW`で実行前にbyte単位で再照合します。取り込み原本とDLLは変更せず、変更済みEXEを再配布もしません。この互換処理後のsandbox EXEは原本のSHA-256やAuthenticode署名とは一致しないため、供給元検証は必ず処理前の原本に対して行い、実行copyの信頼は原本検証・固定変換・resource再照合の連鎖として扱います。[MicrosoftのUTF-8 process code page](https://learn.microsoft.com/en-us/windows/apps/design/globalizing/use-utf8-code-page)はWindows 10 version 1903以降が対象です。

Windowsのtree member列挙は、`FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT`かつread共有だけで開いたdirectory handleに対し、`GetFileInformationByHandleEx(FileIdBothDirectoryInfo)`を使用します。固定64 KiB bufferを検査し、設定のfile＋directory上限を超えて名前を蓄積しません。handleと現在pathのvolume serial／file indexを列挙前後で照合し、各directory identityを初回監査と実コピーの間でも比較します。これにより列挙対象directory自身のrename／deleteと同名の空directory差替えを検出します。根拠は[`FILE_ID_BOTH_DIR_INFO`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_both_dir_info)、[`GetFileInformationByHandle`](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileinformationbyhandle)、[`CreateFileW`](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)です。非Windowsの明示的検証経路はdirectory handleとidentityを保持・照合しますが、member名の取得自体は`read_dir`です。

child file／directoryを開く操作自体はWin32 path APIを使うため、親handleをrootにしたnative relative openではありません。開いたchild handleのidentity、reparse状態、内容と、最終tree fingerprintでfail closedにしますが、Windows実機でのreparse point競合stress testは未完です。DACL封印はbackend自身によるstaging tree変更を予防しますが、親の通常ユーザー権限は意図的に残すため、同一ユーザー権限をすでに持つ別プロセスとの全競合を排除したとは扱いません。事後fingerprintと別sandboxでの再展開照合は引き続き必要です。

### 暗号化書庫

安全なパスワード受け渡しは未実装です。現在の全backend processは標準入力を`NUL`へ固定しているため、対話入力を要求する書庫はfail closedになります。bsdtarの`--passphrase`は秘密をprocess argumentsへ残すため使用しません。Windows版bsdtarの対話callbackはconsole handleを要求するので、単純な匿名pipeへの差し替えも採用しません。

計画中の経路は、一回限りのConPTY input、非継承のcontroller handle、専用threadでのoutput drain、prompt回数・時間・出力量の上限、native password dialog、終了直後のbuffer zeroizationを組み合わせます。cancel、空入力、wrong password、複数prompt、timeout、backend crashの全経路でpartial outputと秘密channelを破棄できるまで有効化しません。詳細は[`ENCRYPTED_ARCHIVES.md`](ENCRYPTED_ARCHIVES.md)で追跡します。

## 7. 将来の強化候補

- LPACの実書庫・ACL・network denial matrixと必要capability 0件の実証
- 最初の実環境Authenticode/SLSA/immutable-release証跡と独立レビュー、および署名済みアップデート
- Windows Attachment Servicesの実OS／Defender／第三者provider matrix
- AppLocker／WDAC向けpublisher rule
- パスワードを保護された匿名パイプで渡す仕組み
- 親handleをrootにしたnative child open、Windows staging DACL probe／handle列挙の初回実機証跡、Windows reparse競合stress test
- first passing review of the generated malicious corpus plus its remaining format/control-byte/CPU-bomb/crash/race matrix described in [`MALICIOUS_CORPUS.md`](MALICIOUS_CORPUS.md), and the Windows 10/11, LPAC, read-format, denial, crash, and race matrix in [`WINDOWS_E2E.md`](WINDOWS_E2E.md)
- MSYS2 package key rotation、過去archive availability、生成済みbackend SBOM/license証跡の独立レビュー
