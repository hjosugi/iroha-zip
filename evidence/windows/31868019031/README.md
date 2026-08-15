# Windows evidence snapshot / Windows証跡snapshot

## 日本語

このdirectoryは、`main`のexact commit
`5cbc6c27fb67466369b20180a9c5aa2fdd3f6713`を実行した
[GitHub Actions run 31868019031](https://github.com/hjosugi/iroha-zip/actions/runs/31868019031)
から、2026-08-15に取得して独立検証した11件の機械可読JSONを長期保存します。

取得した3 artifactは次のとおりです。`digest`はGitHub Actions artifact ZIPに対するAPI値です。

| artifact | API SHA-256 digest | bytes | expires at |
|---|---|---:|---|
| `windows-arm64-native` | `8546d38ffcb450e3c365b56a8f6bb421a54593688a13fa67acdfa8ebbd696d57` | 10,524 | 2026-11-13 05:52:17 UTC |
| `windows-e2e-windows-2022` | `649d6e89ace562dd22e12e51eed5d7cf839e3ee940df94d804f4943fc548cde6` | 7,879 | 2026-11-13 05:52:17 UTC |
| `windows-e2e-windows-2025` | `c107ad239ec9f5503d22e8313f19581165d6f55792aec048483f33e02ea56207` | 7,891 | 2026-11-13 05:52:17 UTC |

[`SOURCE_SHA256SUMS.txt`](SOURCE_SHA256SUMS.txt)はartifactから展開した元のJSON bytesを固定します。
checked-in JSONは、値を追加・削除せず`jq 1.8.2 -S '.'`でkey順と空白をcanonical化し、LFで終端したものです。
[`SHA256SUMS.txt`](SHA256SUMS.txt)は、このcanonical copyのbytesを固定します。これにより元の
PowerShell生成CRLFを偽装せず、artifactが期限切れになった後も全値を再検査できます。

通常のRust testはexact 11-file inventoryとcanonical SHA-256に加え、schema-v5 archive matrix、
4作成形式、14読取形式、3暗号化方式、password非出力、raw-stream拒否、capability 0、network／
timeout／memory拒否、7 cleanup、19-sample corpus、native ARM64 PE、日英Settings 26 controls、
`PerMonitorV2`、96→144→96遷移、temporary-root削除を再検証します。

このsnapshotはdiagnostic evidenceであり、Authenticode署名、release attestation、security audit、
Windows 10/11 x64 desktop実機証明ではありません。悪性archiveやbackend binaryは含みません。

## English

This directory preserves the 11 machine-readable JSON reports downloaded and independently checked
on 2026-08-15 from
[GitHub Actions run 31868019031](https://github.com/hjosugi/iroha-zip/actions/runs/31868019031),
which executed exact `main` commit `5cbc6c27fb67466369b20180a9c5aa2fdd3f6713`.

The table above records the GitHub API SHA-256 digest and size of each source artifact ZIP together
with its 90-day expiration. [`SOURCE_SHA256SUMS.txt`](SOURCE_SHA256SUMS.txt) fixes the bytes of the
JSON files extracted from those artifacts. The checked-in JSON files were normalized with
`jq 1.8.2 -S '.'` and a final LF without adding or removing values. [`SHA256SUMS.txt`](SHA256SUMS.txt)
fixes these canonical copies. This distinction preserves every value for review without pretending
that normalized files retain the source PowerShell CRLF bytes.

The ordinary Rust test requires the exact 11-file inventory and canonical hashes, then rechecks the
schema-v5 archive matrix, four create formats, 14 read formats, three encryption modes, password
output absence, raw-stream rejection, zero-capability isolation, network/timeout/memory denial,
seven cleanups, the 19-sample corpus, native ARM64 PE identity, all four Japanese/English 26-control
Settings reports, `PerMonitorV2`, the 96→144→96 transition, and temporary-root removal.

This snapshot is diagnostic evidence, not an Authenticode signature, release attestation, security
audit, or Windows 10/11 x64 desktop-device result. It contains no hostile archive or backend binary.
