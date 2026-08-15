# 未署名リリースについて / About unsigned releases

## 日本語

iroha-zip の現在の公式 Windows バイナリは Authenticode 未署名です。Windows SmartScreen やブラウザーが警告を表示する場合があります。警告を隠したり無効化したりせず、次を確認してから利用してください。

1. `https://github.com/hjosugi/iroha-zip/releases` から取得したことを確認する。
2. リリース添付の `SHA256SUMS.txt` とダウンロードしたファイルの SHA-256 を比較する。
3. `gh release verify v0.6.2 --repo hjosugi/iroha-zip`で、tagと11 assetを固定するGitHub release attestationを確認する。
4. `gh release verify-asset v0.6.2 <file> --repo hjosugi/iroha-zip`で、downloadしたfileがそのReleaseに含まれることを確認する。
5. さらに、`gh attestation verify <file> --repo hjosugi/iroha-zip`でrelease workflowが発行したartifact attestationを確認する。
6. 公式EXEが使用する`VCRUNTIME140.dll`がない場合は、対象architectureの[Microsoft公式Visual C++ v14 Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)を導入する。第三者siteからDLL単体を取得しない。
7. バックエンドは同梱されていない。設定画面から自分が信頼する libarchive / `bsdtar.exe` を取り込み、`doctor` が成功することを確認する。

PowerShell で SHA-256 を表示する例:

```powershell
Get-FileHash .\iroha-zip-0.6.2-windows-x64.zip -Algorithm SHA256
# Windows on ARMの場合 / For Windows on ARM:
Get-FileHash .\iroha-zip-0.6.2-windows-arm64.zip -Algorithm SHA256
```

dry runで得たSHA-256を、後から作る正式Releaseの期待値として使わないでください。dry runとtag公開は
別々のnative buildです。v0.5.2の両buildを比較すると、6 EXEはすべて同じbyte長でしたが、各23–24
byteのCOFF／debug timestampとCodeView PDB GUIDが異なりました。現状はbit-reproducibleなPE／PDB
buildを主張しません。依存crateのpanic位置には、一般的なGitHub-hosted runnerのCargo registry
source prefix（`C:\Users\runneradmin\.cargo\registry\src\...`）も残ります。6 EXEのASCII／UTF-16
文字列検査では、repository workspace path、runner temporary-directory path、明白なsecret-value
markerは検出されませんでしたが、build-path-independentとも主張しません。必ず公開Release自身の
`SHA256SUMS.txt`とtag-ref attestationを確認してください。

未署名であることは、ファイルが安全であることも危険であることも単独では証明しません。リポジトリ、ハッシュ、GitHub artifact attestation、公開ソースを組み合わせて出所を確認してください。

## English

The current official iroha-zip Windows binaries are not Authenticode-signed. Windows SmartScreen or your browser may display a warning. Do not hide or disable that warning; verify the following before use:

1. Confirm that the file came from `https://github.com/hjosugi/iroha-zip/releases`.
2. Compare the file's SHA-256 digest with the release's `SHA256SUMS.txt`.
3. Verify the GitHub release attestation binding the tag and all 11 assets with `gh release verify v0.6.2 --repo hjosugi/iroha-zip`.
4. Confirm that the downloaded file belongs to that Release with `gh release verify-asset v0.6.2 <file> --repo hjosugi/iroha-zip`.
5. Also verify the artifact attestation issued by the release workflow with `gh attestation verify <file> --repo hjosugi/iroha-zip`.
6. If `VCRUNTIME140.dll`, which the official EXEs import, is absent, install the architecture-matching [official Microsoft Visual C++ v14 Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist). Do not download an individual DLL from a third-party site.
7. The archive backend is not bundled. Import a libarchive / `bsdtar.exe` bundle you trust in Settings and require `doctor` to pass.

Example SHA-256 check in PowerShell:

```powershell
Get-FileHash .\iroha-zip-0.6.2-windows-x64.zip -Algorithm SHA256
# Windows on ARM:
Get-FileHash .\iroha-zip-0.6.2-windows-arm64.zip -Algorithm SHA256
```

Do not use a dry-run SHA-256 value as the expected digest for a later published Release. Dry runs
and tag publication are separate native builds. Comparing both v0.5.2 builds found equal byte
lengths for all six EXEs, but 23–24 differing bytes per file in COFF/debug timestamps and CodeView
PDB GUIDs. Dependency panic locations also retain the generic GitHub-hosted runner Cargo-registry
source prefix (`C:\Users\runneradmin\.cargo\registry\src\...`). An ASCII/UTF-16 string scan of all
six EXEs found no repository-workspace path, runner temporary-directory path, or obvious
secret-value marker. Bit-reproducible or build-path-independent PE/PDB output is not currently
claimed. Always verify the published Release's own `SHA256SUMS.txt` and tag-ref attestation.

The absence of an Authenticode signature does not, by itself, prove that a file is safe or unsafe. Establish provenance by combining the repository URL, SHA-256 digest, GitHub artifact attestation, and published source.
