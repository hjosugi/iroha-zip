# iroha-zip 0.4.0

## 日本語

iroha-zip 0.4.0 は、Windows x64 向けの最初の安定版扱いリリースです。ZIPに加えて、3つの実行ファイルを個別にダウンロードできます。

### ダウンロードの選び方

- **通常は `iroha-zip-0.4.0-windows-x64.zip`**: 実行ファイル、設定・関連付けスクリプト、日英README、ライセンス、設計文書をまとめて含みます。
- **`iroha-zip-0.4.0-windows-x64.exe`**: CLI本体だけが必要な場合。
- **`iroha-zip-settings-0.4.0-windows-x64.exe`**: ネイティブ設定画面。
- **`iroha-zip-shell-0.4.0-windows-x64.exe`**: Windowsのファイル関連付け用ランチャー。
- **`SHA256SUMS.txt`**: ZIPと3つのEXEのSHA-256一覧。

### 重要

- バイナリは **Authenticode未署名** です。SmartScreenが警告する場合があります。警告を無効化せず、`SHA256SUMS.txt`とGitHub artifact attestationで出所を確認してください。
- libarchive / `bsdtar.exe` は同梱していません。設定画面から自分が信頼するバックエンドを取り込み、`iroha-zip.exe doctor`が成功することを確認してください。
- セキュリティ監査済み製品ではありません。未信頼ファイルの実行前には、OS・Defender・組織のセキュリティ手順も併用してください。

### 主な変更

- 生成書庫を別のsandboxで再展開し、監査済み圧縮元と完全一致するまで公開しない作成経路。
- 監査済み通常objectだけの外部staging treeを全entry単位でread-only封印し、相対`.`だけから作成する経路と、handleを保持したTOCTOU対策。
- 検証済みsandbox copyへ固定UTF-8 process manifestを埋め込み・再照合する、Windows版libarchive 3.8.6以降の日本語名回帰対策。
- raw member名を検査するbounded preflight、悪性書庫回帰コーパス、5つのbounded fuzz target。
- policy-safe previewと、全書庫監査後に行う選択展開。
- backend provenance、SPDX SBOM、license evidenceの厳格な検証。
- Windows Attachment Servicesへの明示的なbest-effort／required handoff。
- 日本語／英語、高DPI、キーボード操作、rollback-safe保存に対応した設定画面。
- 日本語／英語のGitHub Pagesと完全な日英利用ガイド。

既知の制約と検証状況は、同梱の `README.md` / `README.en.md` と `docs/BUILD_STATUS.md` を確認してください。

---

## English

iroha-zip 0.4.0 is the first release presented as a stable Windows x64 download. In addition to the complete ZIP, all three executables are available separately.

### Which download to choose

- **Normally choose `iroha-zip-0.4.0-windows-x64.zip`**: includes the executables, setup and association scripts, Japanese and English guides, licenses, and design documents.
- **`iroha-zip-0.4.0-windows-x64.exe`**: the CLI only.
- **`iroha-zip-settings-0.4.0-windows-x64.exe`**: the native Settings application.
- **`iroha-zip-shell-0.4.0-windows-x64.exe`**: the Windows file-association launcher.
- **`SHA256SUMS.txt`**: SHA-256 digests for the ZIP and all three executables.

### Important

- The binaries are **not Authenticode-signed**. SmartScreen may display a warning. Do not disable the warning; establish provenance with `SHA256SUMS.txt` and the GitHub artifact attestation.
- libarchive / `bsdtar.exe` is not bundled. Import a backend you trust in Settings and require `iroha-zip.exe doctor` to pass.
- This is not a security-audited product. Continue to use Windows, Defender, and your organization's normal security controls before running extracted files.

### Highlights

- Creation re-extracts every generated archive in a second sandbox and refuses publication unless it exactly reproduces the audited source.
- A creation path that copies only audited regular objects into external staging, seals every entry read-only to the Package SID, archives only relative `.`, and retains handles across TOCTOU-sensitive boundaries.
- A fixed UTF-8 process manifest embedded into and read back from each verified sandbox executable copy to avoid the Windows libarchive 3.8.6+ Unicode-name regression.
- Bounded raw-member preflight, a generated hostile-archive regression corpus, and five bounded fuzz targets.
- Policy-safe preview and selective extraction only after auditing the complete archive.
- Strict backend provenance, SPDX SBOM, and license-evidence verification.
- Explicit best-effort or required Windows Attachment Services handoff.
- A Japanese/English, high-DPI, keyboard-accessible Settings application with rollback-safe saves.
- Japanese and English GitHub Pages plus complete bilingual usage guidance.

See the packaged `README.md` / `README.en.md` and `docs/BUILD_STATUS.md` for known limitations and current validation evidence.
