# Windows evidence snapshot / Windows証跡snapshot

## 日本語

このdirectoryは、`main`のexact commit
`71f7b674745bc8446142f4f7dbf71534839ac9fa`を実行した
[GitHub Actions run 31891960603](https://github.com/hjosugi/iroha-zip/actions/runs/31891960603)
から、2026-08-16 JSTに取得して独立検証した11件の機械可読JSONを長期保存します。

取得した3 artifactは次のとおりです。`digest`はGitHub Actions artifact ZIPに対するAPI値です。

| artifact | id | API SHA-256 digest | bytes | expires at |
|---|---:|---|---:|---|
| `windows-arm64-native` | 9248885037 | `aac81bd3efd322b36f5b2d862047e2597dd770f241fb575753463bf58df7261c` | 11,419 | 2026-11-13 15:10:16 UTC |
| `windows-e2e-windows-2022` | 9248818543 | `b96877145d646aa7a3ce959f67c7d61d1e9b3c3edde95c7781b8efb3e0fefbef` | 8,270 | 2026-11-13 15:10:16 UTC |
| `windows-e2e-windows-2025` | 9248825689 | `172f6e3213da365e3bfcd75c8ef36fe9cd669775385aeb1aa7818a86ae5517e0` | 8,287 | 2026-11-13 15:10:16 UTC |

[`SOURCE_SHA256SUMS.txt`](SOURCE_SHA256SUMS.txt)はartifactから展開した元のJSON bytesを固定します。
checked-in JSONは、値を追加・削除せず`jq 1.8.2 -S '.'`でkey順と空白をcanonical化し、LFで終端したものです。
[`SHA256SUMS.txt`](SHA256SUMS.txt)は、このcanonical copyのbytesを固定します。これにより元の
PowerShell生成CRLFを偽装せず、artifactが期限切れになった後も全値を再検査できます。

通常のRust testはexact 11-file inventoryとcanonical SHA-256に加え、schema-v5 archive matrix、
4作成形式、14読取形式、3暗号化方式、password非出力、raw-stream拒否、capability 0、network／
timeout／memory拒否、7 cleanup、19-sample corpus、native ARM64 PE、日英Settings 26 controls、
`PerMonitorV2`、96→144→96遷移、temporary-root削除を再検証します。schema-v3 Settings証跡は、
4環境すべてでproduction backend Browse pickerを1回完了し、残る2つのimport pickerを取消した
ことを記録します。Server 2022/2025 x64は実際の`SendInput`による全26 controlsの正逆Tab巡回、
Enter保存、Escape終了要求も記録します。GitHub-hosted ARM64はforeground focusを公開しないため、
同じ巡回・保存・終了要求を限定fallbackで検証しつつ、real-key flagsを明示的にfalseと記録します。

このsnapshotはdiagnostic evidenceであり、物理keyboard、screen reader、実monitor、Windows
10/11 x64 desktop、Authenticode署名、release attestation、security auditの証明ではありません。
悪性archiveやbackend binaryは含みません。

## English

This directory preserves the 11 machine-readable JSON reports downloaded and independently checked
on 2026-08-16 JST from
[GitHub Actions run 31891960603](https://github.com/hjosugi/iroha-zip/actions/runs/31891960603),
which executed exact `main` commit `71f7b674745bc8446142f4f7dbf71534839ac9fa`.

The table above records each artifact ID, GitHub API SHA-256 digest and size together with its 90-day
expiration. [`SOURCE_SHA256SUMS.txt`](SOURCE_SHA256SUMS.txt) fixes the bytes of the JSON files
extracted from those artifacts. The checked-in JSON files were normalized with `jq 1.8.2 -S '.'`
and a final LF without adding or removing values. [`SHA256SUMS.txt`](SHA256SUMS.txt) fixes these
canonical copies. This distinction preserves every value for review without pretending that
normalized files retain the source PowerShell CRLF bytes.

The ordinary Rust test requires the exact 11-file inventory and canonical hashes, then rechecks the
schema-v5 archive matrix, four create formats, 14 read formats, three encryption modes, password
output absence, raw-stream rejection, zero-capability isolation, network/timeout/memory denial,
seven cleanups, the 19-sample corpus, native ARM64 PE identity, all four Japanese/English 26-control
Settings reports, `PerMonitorV2`, the 96→144→96 transition, and temporary-root removal. The
schema-v3 Settings evidence records one completed production backend Browse picker and two safely
cancelled import pickers in all four reports. Server 2022/2025 x64 additionally record real
`SendInput` traversal, save, and close-request paths. Hosted ARM64 records the bounded focus fallback
and explicitly false real-key flags.

This snapshot is diagnostic evidence, not proof from a physical keyboard, screen reader, real
monitor, Windows 10/11 x64 desktop, Authenticode signature, release attestation, or security audit.
It contains no hostile archive or backend binary.
