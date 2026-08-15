# Windows evidence snapshot / Windows証跡snapshot

## 日本語

このdirectoryは、`main`のexact commit
`9debd02e819899f8dbdfdd5281d3d0b2a68a89db`を実行した
[GitHub Actions run 31875638650](https://github.com/hjosugi/iroha-zip/actions/runs/31875638650)
から、2026-08-15に取得して独立検証した11件の機械可読JSONを長期保存します。

取得した3 artifactは次のとおりです。`digest`はGitHub Actions artifact ZIPに対するAPI値です。

| artifact | API SHA-256 digest | bytes | expires at |
|---|---|---:|---|
| `windows-arm64-native` | `1cbecf963e2546aa6993d8d995c396b9aa72d11b6695f73d24c1fc22393410fd` | 11,424 | 2026-11-13 08:54:23 UTC |
| `windows-e2e-windows-2022` | `fcdb9809f254c0f76f4002148dec91e027d57855398eeb8700e26f051d44476c` | 8,278 | 2026-11-13 08:54:23 UTC |
| `windows-e2e-windows-2025` | `24531e4c7e1e26da3e0ad7e933e3af1012cd7d63a161bb242e6fe6ec2f5511f6` | 8,269 | 2026-11-13 08:54:23 UTC |

[`SOURCE_SHA256SUMS.txt`](SOURCE_SHA256SUMS.txt)はartifactから展開した元のJSON bytesを固定します。
checked-in JSONは、値を追加・削除せず`jq 1.8.2 -S '.'`でkey順と空白をcanonical化し、LFで終端したものです。
[`SHA256SUMS.txt`](SHA256SUMS.txt)は、このcanonical copyのbytesを固定します。これにより元の
PowerShell生成CRLFを偽装せず、artifactが期限切れになった後も全値を再検査できます。

通常のRust testはexact 11-file inventoryとcanonical SHA-256に加え、schema-v5 archive matrix、
4作成形式、14読取形式、3暗号化方式、password非出力、raw-stream拒否、capability 0、network／
timeout／memory拒否、7 cleanup、19-sample corpus、native ARM64 PE、日英Settings 26 controls、
`PerMonitorV2`、96→144→96遷移、temporary-root削除を再検証します。schema-v2 Settings証跡は、
Server 2022/2025 x64で実際の`SendInput`による全26 controlsの正逆Tab巡回とwrap、Enter保存、
保存messageのEnter終了、dirty状態のEscape終了要求と取消を記録します。GitHub-hosted ARM64は
foreground focusを公開しないため、同じ巡回・保存・終了要求を限定fallbackで検証しつつ、
`realKeyInput`、`enterKeyVerified`、`escapeKeyVerified`を明示的に`false`と記録します。

このsnapshotはdiagnostic evidenceであり、物理keyboard、screen reader、実monitor、Windows
10/11 x64 desktop、Authenticode署名、release attestation、security auditの証明ではありません。
悪性archiveやbackend binaryは含みません。

## English

This directory preserves the 11 machine-readable JSON reports downloaded and independently checked
on 2026-08-15 from
[GitHub Actions run 31875638650](https://github.com/hjosugi/iroha-zip/actions/runs/31875638650),
which executed exact `main` commit `9debd02e819899f8dbdfdd5281d3d0b2a68a89db`.

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
Settings reports, `PerMonitorV2`, the 96→144→96 transition, and temporary-root removal. The
schema-v2 Settings evidence records real `SendInput` forward/reverse Tab traversal and wrap across
all 26 controls on Server 2022/2025 x64, plus real Enter save, Enter saved-message dismissal, and
Escape dirty-close request/cancellation. Because the GitHub-hosted ARM64 image exposes no foreground
focus for the spawned window, it verifies the same order and actions through the bounded fallback
while explicitly recording `realKeyInput`, `enterKeyVerified`, and `escapeKeyVerified` as false.

This snapshot is diagnostic evidence, not proof from a physical keyboard, screen reader, real
monitor, Windows 10/11 x64 desktop, Authenticode signature, release attestation, or security audit.
It contains no hostile archive or backend binary.
