# セキュリティポリシー / Security policy

[日本語](#日本語) | [English](#english)

iroha-zip 0.6.x はセキュリティ指向の実用版ですが、第三者監査済み製品ではありません。未署名リリースの意味と検証方法は [未署名リリースについて](docs/UNSIGNED_RELEASE.md) を参照してください。

iroha-zip 0.6.x is a security-oriented usable release, but it is not a third-party-audited product. See [About unsigned releases](docs/UNSIGNED_RELEASE.md#english) for the current signing status and verification procedure.

## 日本語

### サポート対象

最新の `main` と `0.6.x` を対象とします。`0.5.x` 以前はソースレビュー用の旧版であり、セキュリティ修正の対象外です。

### 脆弱性の報告

[GitHub Security Advisories](https://github.com/hjosugi/iroha-zip/security/advisories/new) から、再現手順、影響、対象バージョンを非公開で報告してください。公開 Issue へ、実際に悪用可能な書庫、秘密情報、未修正の詳細を添付しないでください。

受付後は影響範囲と再現可否を確認し、修正・回帰試験・公開方法を報告者と調整します。応答時間や修正期限は保証できませんが、受信した報告を公開 Issue へ無断転記しません。

### セキュリティ上重要なファイル

- `src/platform/windows_impl.rs`
- `src/policy.rs`
- `src/backend.rs`
- `src/extract.rs`
- `src/create.rs`
- `src/monitor.rs`
- `src/transfer.rs`
- `scripts/install-backend.ps1`
- `scripts/export-msys2-backend.ps1`
- `scripts/build-release.ps1`
- `scripts/verify-release-signatures.ps1`
- `.github/workflows/release.yml`

### 「監査済み・production-ready」と表明するための条件

通常の安定版番号や未署名配布を妨げる条件ではありません。ただし「第三者監査済み」「production-ready」「安全性を保証」と表明する前には、最低限次を完了し、証拠を公開する必要があります。

- Windows CI の test / Clippy と、Windows 10/11 実機の形式・失敗経路 matrix
- 作成した ZIP / 7z / TAR / TAR.GZ の再展開・完全 tree 照合
- traversal、link、junction、ADS、resource bomb、race の回帰試験
- locked build、依存関係・backend DLL・配布元署名の確認
- owner が明示的に管理する Authenticode 署名、publisher/EKU/timestamp 検証、独立 provenance、immutable release
- 独立したセキュリティレビューと、既知の限界の公開

現行の未署名 asset 契約と将来の署名経路は [リリース検証仕様](docs/RELEASE_VERIFICATION.md) に記録しています。

## English

### Supported versions

The latest `main` branch and `0.6.x` are supported. Versions `0.5.x` and earlier are historical releases retained for source review and do not receive security fixes.

### Reporting a vulnerability

Use [GitHub Security Advisories](https://github.com/hjosugi/iroha-zip/security/advisories/new) to report reproduction steps, impact, and affected versions privately. Do not attach an exploitable archive, secrets, or unpatched details to a public issue.

After receipt, the maintainer will assess scope and reproducibility and coordinate the fix, regression evidence, and disclosure with the reporter. No response or remediation deadline is guaranteed, but a private report will not be copied to a public issue without coordination.

### Security-sensitive files

The files listed in the Japanese section above are trust-boundary code and require particularly careful review.

### Requirements for an “audited” or “production-ready” claim

These requirements do not block an ordinary stable version or an explicitly unsigned download. Before claiming that the project is third-party audited, production-ready, or safety-guaranteed, however, the project must complete and publish evidence for at least:

- passing Windows tests/Clippy plus a Windows 10/11 physical or disposable-VM format and failure-path matrix;
- complete re-extraction and tree comparison for created ZIP, 7z, TAR, and TAR.GZ archives;
- traversal, link, junction, ADS, resource-bomb, and race regressions;
- locked builds and review of Rust dependencies, backend DLLs, and distributor signatures;
- owner-authorized Authenticode signing, publisher/EKU/timestamp checks, independent provenance, and immutable releases; and
- independent security review with publicly documented residual limitations.

The current unsigned asset contract and the future signing path are recorded in the [release verification specification](docs/RELEASE_VERIFICATION.md).
