# Signed updater design / 署名付きupdater設計

Updated: 2026-08-15

## 日本語

### 現在の状態

iroha-zipはupdaterを出荷しておらず、backgroundでnetworkへ接続しません。`v0.6.0`を含む公式版は
未署名なので、digestやGitHub artifact attestationだけを根拠に自己更新を有効化しません。
Authenticode publisher identityとcustodyが独立に確立され、署名付きimmutable Releaseが成功するまで
OPS-001は実装開始不可です。

### 固定する信頼境界

- update checkは既定無効で、利用者が設定画面から明示的に有効化する。
- channelは`stable`と明示的な`preview`を分離し、stableからpreviewへ自動移動しない。
- GitHub HTTPS、Release metadata、同じ場所から取得したchecksumだけをpublisher identityとして扱わない。
- downloadしたmanifest、package、展開後の3 EXEを、実行や置換の前にSHA-256とWindows trust APIで検証する。
- publisher subject、Code Signing EKU、RFC 3161 timestampを現在のrelease verifierと同じpolicyで照合する。
- version、architecture、asset名、byte長、digest、3 EXE inventoryが1件でも不一致ならfail closedにする。
- downgradeは既定で禁止する。recovery downgradeは、利用者がexact versionを明示し、その古いpackageも
  現在信頼するpublisherで有効に署名され、明示したdigestと一致する場合だけ別操作として許可する。
- backend directory、backend manifest、provenance、SPDX、license evidenceには一切触れない。backendの更新は
  既存の独立した利用者承認付きimportだけで行う。

### 置換とrollback

候補は設定・backend・現在のinstall directoryとは別の一意なdirectoryへcreate-newで取得します。検証後、
独立した最小updater processへ、秘密を含まない固定manifest handleとinstall targetだけを渡します。updaterは
現在の3 EXEを同一volume上のbackup名へ移し、候補3 EXEを同一volume renameで配置し、配置後にもう一度
identity、長さ、SHA-256、Authenticodeを検証します。

起動確認前の失敗では旧3 EXEをbyte-identicalに戻し、候補を隔離します。復元にも失敗した場合はbackupを
削除せず、正確なrecovery pathを表示して自動再試行を止めます。成功後も旧版は次回正常起動が確認されるまで
保持し、無期限には残しません。immutable Release assetを同じversionで修復・置換することはありません。

### 有効化前の必須試験

1. 正常更新、同一version、preview channel、明示recovery downgrade。
2. manifest/package/展開後EXEのdigest、名前、大小文字、長さ、architecture改変。
3. 未署名、期限切れ、publisher違い、EKU違い、timestampなし／不正。
4. download中断、disk full、AV quarantine、rename拒否、同時更新、起動失敗、rollback失敗。
5. 更新前後でconfigとbackend treeのidentity/hashが不変であること。
6. Windows x64とARM64 assetを相互に受理しないこと。
7. 全失敗で未検証binaryを実行せず、回復可能な旧版または明示されたbackupを残すこと。

## English

### Current state

iroha-zip ships no updater and makes no background network connection. Official builds, including
`v0.6.0`, are unsigned, so a digest or GitHub artifact attestation alone cannot authorize
self-update. OPS-001
must remain disabled until an Authenticode publisher identity and custody process are independently
established and a signed immutable release succeeds.

### Fixed trust boundary

- Update checks are off by default and require an explicit Settings choice.
- `stable` and explicit `preview` channels are distinct; stable never moves to preview automatically.
- GitHub HTTPS, release metadata, and checksums from the same transport are not publisher identity.
- The downloaded manifest/package and all three expanded executables are checked by SHA-256 and
  Windows trust APIs before execution or replacement.
- Publisher subject, Code Signing EKU, and RFC 3161 timestamp follow the existing release verifier.
- Version, architecture, asset name, byte length, digest, and exact executable inventory all fail closed.
- Downgrades are denied by default. A recovery downgrade is a separate, explicit exact-version action;
  the older package must still have a valid signature from the currently trusted publisher and match
  the explicitly selected digest.
- The backend directory, manifest, provenance, SPDX, and license evidence are never touched. Backend
  changes continue to use the independent, user-approved import flow.

### Replacement and rollback

A candidate is created in a unique directory separate from configuration, backend, and the current
installation. After verification, a minimal updater process receives only a non-secret pinned
manifest handle and install target. It moves the current three executables to same-volume backup
names, installs the three candidates by same-volume rename, and rechecks identity, length, SHA-256,
and Authenticode after placement.

Any failure before launch confirmation restores the byte-identical old executables and quarantines
the candidate. If restoration also fails, the backup is preserved, its exact recovery path is shown,
and automatic retries stop. A successful update retains the old version only until the next healthy
launch. An immutable asset is never repaired or replaced under the same version.

### Tests required before enablement

1. Normal update, same version, preview channel, and explicit recovery downgrade.
2. Manifest/package/expanded-EXE digest, name, case, length, and architecture tampering.
3. Unsigned, expired, wrong-publisher, wrong-EKU, missing-timestamp, and invalid-timestamp inputs.
4. Interrupted download, disk full, AV quarantine, denied rename, concurrent update, launch failure,
   and rollback failure.
5. Identical configuration and backend-tree identities/hashes before and after every path.
6. Mutual rejection of Windows x64 and ARM64 assets.
7. No unverified binary execution on any failure, with either a recoverable old version or an exact
   preserved-backup path.

Primary references:

- [Microsoft WinVerifyTrust](https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-winverifytrust)
- [Microsoft Authenticode](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/authenticode)
- [GitHub immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)
- [iroha-zip release verification](RELEASE_VERIFICATION.md)
