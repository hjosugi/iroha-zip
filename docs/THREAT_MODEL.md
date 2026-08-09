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
| バックエンド差し替え | EXEと全DLLをSHA-256マニフェストで固定し、コピー後に再ハッシュ |
| DLL横取り | バックエンドディレクトリの余分なファイルを拒否し、最小PATHで起動 |
| 既存データ上書き | `create_new`、`-k`、既存出力拒否、最終rename |
| 作成処理から圧縮元全体へアクセス | 監査済み通常ファイルだけをAppContainer領域へ複製してから圧縮 |
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
  3. 検査済みの通常ファイルだけをAppContainer領域へコピー
  4. バックエンドをコピー後に再ハッシュ

AppContainerプロセス
  5. bsdtarが専用outputへ書庫を作成
  6. 親プロセスが出力サイズとオブジェクト数を監視

通常プロセス
  7. 生成物が通常ファイルであり上限内であることを再検査
  8. 新規ファイルとして最終出力先へコピー
  9. AppContainer profileと一時データを削除
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

ディレクトリ列挙そのものを親ディレクトリハンドル相対で固定する実装、監査コピー完了後からbackend読取完了までのstaging tree封印、作成書庫の再展開照合、およびWindows実機でのreparse point競合stress testは未完です。同一ユーザー権限をすでに持つ能動的攻撃者との全競合を排除したとは扱いません。

### 暗号化書庫

安全なパスワード受け渡しは未実装です。現在は標準入力を`NUL`へ固定しているため、対話入力を要求する書庫はfail closedになります。bsdtarの`--passphrase`は秘密をprocess argumentsへ残すため使用しません。Windows版bsdtarの対話callbackはconsole handleを要求するので、単純な匿名pipeへの差し替えも採用しません。

計画中の経路は、一回限りのConPTY input、非継承のcontroller handle、専用threadでのoutput drain、prompt回数・時間・出力量の上限、native password dialog、終了直後のbuffer zeroizationを組み合わせます。cancel、空入力、wrong password、複数prompt、timeout、backend crashの全経路でpartial outputと秘密channelを破棄できるまで有効化しません。詳細は[`ENCRYPTED_ARCHIVES.md`](ENCRYPTED_ARCHIVES.md)で追跡します。

## 7. 将来の強化候補

- LPACの実書庫・ACL・network denial matrixと必要capability 0件の実証
- Authenticode署名と署名済みアップデート
- Windows Attachment Servicesの実OS／Defender／第三者provider matrix
- AppLocker／WDAC向けpublisher rule
- パスワードを保護された匿名パイプで渡す仕組み
- 親ディレクトリハンドル相対の列挙、staging tree封印、作成書庫の再照合、Windows reparse競合stress test
- malicious archive corpus and the remaining Windows 10/11, LPAC, read-format, denial, crash, and race matrix described in [`WINDOWS_E2E.md`](WINDOWS_E2E.md)
- MSYS2 package key rotation、過去archive availability、生成済みbackend SBOM/license証跡の独立レビュー
