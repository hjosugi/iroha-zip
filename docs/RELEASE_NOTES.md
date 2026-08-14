# iroha-zip 0.5.0

## 日本語

iroha-zip 0.5.0は、Windows x64に加えてnative Windows ARM64を正式な配布対象にした
安定版です。両アーキテクチャを別々のnative GitHub runnerでbuildし、ZIP内外の全EXEを
PE machine値で照合してから、1つの不変Releaseへ公開します。

### ダウンロードの選び方

- **一般的なIntel/AMD PC**: `iroha-zip-0.5.0-windows-x64.zip`
- **Windows on ARM PC**: `iroha-zip-0.5.0-windows-arm64.zip`
- CLIだけが必要な場合は、対応する`iroha-zip-0.5.0-windows-<arch>.exe`を選べます。
- ネイティブ設定画面と関連付けランチャーも、各arch向けの個別EXEがあります。
- `SHA256SUMS.txt`は2つのZIPと6つの個別EXEをまとめてカバーします。各ZIPには専用の
  `.zip.sha256`もあります。

### 重要

- 6つのEXEは **Authenticode未署名** です。SmartScreenの警告を無効化せず、
  `SHA256SUMS.txt`とGitHub artifact attestationで出所を確認してください。
- libarchive / `bsdtar.exe`は同梱していません。設定画面の「MSYS2から取り込む」は、
  x64版ではUCRT64、ARM64版ではCLANGARM64を自動選択します。取り込み後は
  `iroha-zip.exe doctor`が成功することを確認してください。
- セキュリティ監査済み製品ではありません。Windows 10/11 desktop実機matrix、
  実験的LPAC、未完の形式・race試験はBuild Statusに正確に記録しています。

### v0.4.1からの主な変更

- 署名を必須にするMSYS2 exporter、provenance、SPDX、license inventoryをCLANGARM64へ拡張し、
  native `windows-11-arm`でbackendを含む全matrixを実行しました。ZIP/7z/TAR/TAR.GZ作成、
  BZ2/XZ/Zstandard/compress/CAB読取、悪性コーパス、shell、日英Settings、通常AppContainer、
  LPAC fail-closed境界が同じrunで合格しています。
- x64=`0x8664`、ARM64=`0xAA64`を、build出力、3つの個別EXE、両ZIP内の3 EXEで機械的に
  再検証する二系統Releaseを追加しました。公開前後に11 assetの名前・長さ・SHA-256を完全照合します。
- Microsoft署名済み`makecab.exe`から制御されたLZX CABを生成し、Server 2022/2025でpreviewと
  extractionの完全tree hash一致を検証しました。
- 監査済みdirectoryを実際のNTFS junctionへ置き換える決定的race回帰を追加し、外部fileを保ったまま
  post-audit link拒否となり、出力が公開されないことをx64/ARM64で確認しました。
- ARM64設定画面のMSYS2 importがCLANGARM64を自動指定するようにし、x64 backendとの取り違えを防ぎました。

実測範囲は [Windows E2E](https://github.com/hjosugi/iroha-zip/blob/v0.5.0/docs/WINDOWS_E2E.md)、
[ARM64 status](https://github.com/hjosugi/iroha-zip/blob/v0.5.0/docs/ARM64.md)、
[Build Status](https://github.com/hjosugi/iroha-zip/blob/v0.5.0/docs/BUILD_STATUS.md)を確認してください。

---

## English

iroha-zip 0.5.0 is the stable release that adds native Windows ARM64 distribution alongside
Windows x64. Each architecture is built on a separate native GitHub runner. Every executable inside
and outside both ZIPs is checked by its PE machine value before one immutable Release is published.

### Which download to choose

- **Typical Intel/AMD PC**: `iroha-zip-0.5.0-windows-x64.zip`
- **Windows on ARM PC**: `iroha-zip-0.5.0-windows-arm64.zip`
- If you need only the CLI, choose the matching `iroha-zip-0.5.0-windows-<arch>.exe`.
- Separate native Settings and file-association launcher executables are available for each arch.
- `SHA256SUMS.txt` covers both ZIPs and all six standalone executables. Each ZIP also has its own
  `.zip.sha256` sidecar.

### Important

- All six executables are **not Authenticode-signed**. Do not disable SmartScreen warnings;
  establish provenance with `SHA256SUMS.txt` and the GitHub artifact attestation.
- libarchive / `bsdtar.exe` is not bundled. **Import from MSYS2** in Settings automatically selects
  UCRT64 in the x64 build and CLANGARM64 in the ARM64 build. Require `iroha-zip.exe doctor` to pass
  after import.
- This is not a security-audited product. Build Status precisely records the remaining Windows
  10/11 desktop-device, experimental-LPAC, format, and race validation.

### Main changes from v0.4.1

- The signature-enforcing MSYS2 exporter, provenance, SPDX, and license inventory now support
  CLANGARM64. One native `windows-11-arm` run passes the complete backend matrix: ZIP/7z/TAR/TAR.GZ
  creation; BZ2/XZ/Zstandard/compress/CAB reads; the hostile corpus; shell; Japanese and English
  Settings; normal AppContainer; and the fail-closed LPAC boundary.
- A dual-architecture Release rechecks x64=`0x8664` and ARM64=`0xAA64` across build outputs, all six
  standalone executables, and all three executables inside each ZIP. Exact name, size, and SHA-256
  verification covers all 11 assets before and after publication.
- Fixed Server 2022/2025 jobs generate a controlled LZX CAB with Microsoft-signed `makecab.exe` and
  require exact preview/extraction tree-hash equality.
- A deterministic race regression replaces an audited directory with a real NTFS junction, requires
  post-audit link rejection on x64 and ARM64, preserves the outside file, and publishes no output.
- ARM64 Settings now selects CLANGARM64 automatically for MSYS2 import, preventing an x64-backend
  mix-up in the normal setup path.

See [Windows E2E](https://github.com/hjosugi/iroha-zip/blob/v0.5.0/docs/WINDOWS_E2E.md),
[ARM64 status](https://github.com/hjosugi/iroha-zip/blob/v0.5.0/docs/ARM64.md), and
[Build Status](https://github.com/hjosugi/iroha-zip/blob/v0.5.0/docs/BUILD_STATUS.md) for the measured boundary.
