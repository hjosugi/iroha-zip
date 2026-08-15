# v0.6.2 release evidence / v0.6.2公開証跡

## 日本語

このdirectoryは、未署名のimmutable
[`v0.6.2`](https://github.com/hjosugi/iroha-zip/releases/tag/v0.6.2)を2026-08-15に
公開後、公開APIと全assetを独立に再取得して確認した固定事実を保存します。

[`release.json`](release.json)にはannotated tag object、exact commit、release API状態、11 assetの
ID・byte長・API SHA-256、dry run／release／tag CI／source CI／CodeQL／Pages、期限付きworkflow
artifact、9件のattestation subject、独立再検証結果を記録しています。
[`RELEASE_ASSETS_SHA256SUMS.txt`](RELEASE_ASSETS_SHA256SUMS.txt)は、公開Releaseから実際に
downloadした11 assetすべてのbyteを固定します。release内の`SHA256SUMS.txt`は、そのうち2 ZIPと
6 standalone EXEの8件を対象にしています。

独立検証では、exact 11 asset、API digest／length、8 checksum subject、2 sidecar、ZIP内外の
実行ファイル同一性、x64 `0x8664`／ARM64 `0xAA64`、backend非同梱、日英package documentと
sourceの一致、release bodyとsourceの一致、公開Pagesとsourceの一致、およびtag-ref・公式workflow・
hosted runner限定の9 attestationを確認しました。6つのdistinct EXEはいずれもPE Certificate Tableが
空です。このsnapshotはGitHub provenanceと公開bytesの証拠であり、Authenticode publisher署名、
SmartScreen reputation、security auditの代替ではありません。

workflow artifactには期限がありますが、GitHub immutable releaseの11 asset、tag、attestationと
このmetadata snapshotは通常のrepository historyから引き続き検査できます。公開binaryそのものは
大きいため、このdirectoryには複製していません。

## English

This directory preserves fixed facts independently checked on 2026-08-15 after publication of the
unsigned, immutable
[`v0.6.2`](https://github.com/hjosugi/iroha-zip/releases/tag/v0.6.2) release.

[`release.json`](release.json) records the annotated tag object, exact commit, Release API state,
IDs, byte lengths, and API SHA-256 digests for all 11 assets, the dry run, release, tag CI, source CI,
CodeQL and Pages runs, expiring workflow artifacts, nine attestation subjects, and independent
verification results. [`RELEASE_ASSETS_SHA256SUMS.txt`](RELEASE_ASSETS_SHA256SUMS.txt) fixes the bytes
of all 11 assets actually downloaded from the public Release. The Release's own `SHA256SUMS.txt`
covers eight of them: two ZIPs and six standalone executables.

Independent verification checked the exact 11-asset inventory, API digests and lengths, eight
checksum subjects, two sidecars, byte identity between ZIP-contained and standalone executables,
x64 `0x8664` and ARM64 `0xAA64`, backend non-inclusion, bilingual package/source-document equality,
release-body/source equality, public Pages/source equality, and all nine tag-ref attestations with
the official workflow and hosted-runner enforcement. Every one of the six distinct executables had
an empty PE Certificate Table. This snapshot is evidence of GitHub provenance and published bytes,
not an Authenticode publisher signature, SmartScreen reputation, or a security audit.

Workflow artifacts expire. The 11 assets in the immutable GitHub Release, tag, attestations, and
this metadata snapshot remain reviewable through ordinary repository history. The public binaries
are intentionally not duplicated in this directory because of their size.
