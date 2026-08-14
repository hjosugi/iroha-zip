# 未署名リリースについて / About unsigned releases

## 日本語

iroha-zip の現在の公式 Windows バイナリは Authenticode 未署名です。Windows SmartScreen やブラウザーが警告を表示する場合があります。警告を隠したり無効化したりせず、次を確認してから利用してください。

1. `https://github.com/hjosugi/iroha-zip/releases` から取得したことを確認する。
2. リリース添付の `SHA256SUMS.txt` とダウンロードしたファイルの SHA-256 を比較する。
3. 必要に応じて GitHub CLI の `gh attestation verify <file> --repo hjosugi/iroha-zip` で artifact attestation を確認する。
4. バックエンドは同梱されていない。設定画面から自分が信頼する libarchive / `bsdtar.exe` を取り込み、`doctor` が成功することを確認する。

PowerShell で SHA-256 を表示する例:

```powershell
Get-FileHash .\iroha-zip-0.5.1-windows-x64.zip -Algorithm SHA256
# Windows on ARMの場合 / For Windows on ARM:
Get-FileHash .\iroha-zip-0.5.1-windows-arm64.zip -Algorithm SHA256
```

未署名であることは、ファイルが安全であることも危険であることも単独では証明しません。リポジトリ、ハッシュ、GitHub artifact attestation、公開ソースを組み合わせて出所を確認してください。

## English

The current official iroha-zip Windows binaries are not Authenticode-signed. Windows SmartScreen or your browser may display a warning. Do not hide or disable that warning; verify the following before use:

1. Confirm that the file came from `https://github.com/hjosugi/iroha-zip/releases`.
2. Compare the file's SHA-256 digest with the release's `SHA256SUMS.txt`.
3. When desired, verify the GitHub artifact attestation with `gh attestation verify <file> --repo hjosugi/iroha-zip`.
4. The archive backend is not bundled. Import a libarchive / `bsdtar.exe` bundle you trust in Settings and require `doctor` to pass.

Example SHA-256 check in PowerShell:

```powershell
Get-FileHash .\iroha-zip-0.5.1-windows-x64.zip -Algorithm SHA256
# Windows on ARM:
Get-FileHash .\iroha-zip-0.5.1-windows-arm64.zip -Algorithm SHA256
```

The absence of an Authenticode signature does not, by itself, prove that a file is safe or unsafe. Establish provenance by combining the repository URL, SHA-256 digest, GitHub artifact attestation, and published source.
