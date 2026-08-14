# iroha-zip 0.5.1

## 日本語

iroha-zip 0.5.1は、単体の圧縮streamを専用の二段階AppContainer経路で扱い、legacy書庫の
実backend証拠を拡張した安定版です。Windows x64とnative Windows ARM64を別々のnative runnerで
buildし、ZIP内外の全EXEをPE machine値で照合してから、1つのimmutable Releaseへ公開します。

### ダウンロードの選び方

- **一般的なIntel/AMD PC**: `iroha-zip-0.5.1-windows-x64.zip`
- **Windows on ARM PC**: `iroha-zip-0.5.1-windows-arm64.zip`
- CLIだけが必要な場合は、対応する`iroha-zip-0.5.1-windows-<arch>.exe`を選べます。
- native設定画面と関連付けlauncherも、各architecture向けの個別EXEがあります。
- `SHA256SUMS.txt`は2つのZIPと6つの個別EXEをまとめて対象にします。各ZIPには専用の
  `.zip.sha256`もあります。

### 重要

- 6つのEXEは **Authenticode未署名** です。SmartScreenの警告を無効化せず、
  `SHA256SUMS.txt`とGitHub artifact attestationで出所を確認してください。
- libarchive / `bsdtar.exe`は同梱していません。設定画面の「MSYS2から取り込む」は、
  x64版ではUCRT64、ARM64版ではCLANGARM64を自動選択します。取り込み後は
  `iroha-zip.exe doctor`が成功することを確認してください。
- セキュリティ監査済み製品ではありません。Windows 10/11 x64 desktop実機、実験的LPAC、
  screen reader／mixed DPI、さらに広いmalformed format／race試験は未完です。

### v0.5.0からの主な変更

- 単体GZ／BZ2／XZ／Zstandard／`.Z`を、外側の拡張子から導いた安全な1 fileとして展開します。
  manifest固定libarchive DLLをloadする専用子processがstream全体を事前に復号し、別の新規
  AppContainerで実展開を繰り返します。filter不一致、復号error、単一file容量超過、既存出力は
  fail closedです。`tar.gz`などの複合書庫は従来の書庫経路を使います。
- stream内の元filenameは出力名に使いません。圧縮形式は暗号学的な内容真正性を提供しないため、
  展開成功を「信頼済み内容」の判定には使いません。
- 公式libarchive 3.8.9 tag由来のRAR、RAR5、LHA level 3、BZIP2-compressed ZIPX fixtureを、
  upstream license／tag object／commit／encoded・decoded hash付きで固定しました。有界UU decoderは
  encoded hash、envelope、decoded length/hashを要求し、3種類の改変自己testも実行します。
- Windows Server 2022、Server 2025、native Windows 11 ARMで、4作成形式と14追加読取形式を
  preview／extractし、path、type、length、SHA-256 treeを完全照合しました。単体streamの
  filter不一致／32-byte limit超過／payload破損、単体gzipのdouble-click shell経路も含みます。
- backend置換は、旧treeをbackupへrenameした直後の失敗をCIで注入し、byte同一の復元、
  stage／backup残留ゼロ、その後の正常importを要求します。

実測範囲は [Windows E2E](https://github.com/hjosugi/iroha-zip/blob/v0.5.1/docs/WINDOWS_E2E.md)、
[ARM64 status](https://github.com/hjosugi/iroha-zip/blob/v0.5.1/docs/ARM64.md)、
[Build Status](https://github.com/hjosugi/iroha-zip/blob/v0.5.1/docs/BUILD_STATUS.md)を確認してください。

---

## English

iroha-zip 0.5.1 is a stable release that adds a dedicated two-pass AppContainer path for standalone
compressed streams and broadens real-backend evidence for legacy archives. Windows x64 and native
Windows ARM64 are built on separate native runners. Every executable inside and outside both ZIPs is
checked by PE machine value before one immutable Release is published.

### Which download to choose

- **Typical Intel/AMD PC**: `iroha-zip-0.5.1-windows-x64.zip`
- **Windows on ARM PC**: `iroha-zip-0.5.1-windows-arm64.zip`
- If you need only the CLI, choose the matching `iroha-zip-0.5.1-windows-<arch>.exe`.
- Separate native Settings and file-association launcher executables are available for each architecture.
- `SHA256SUMS.txt` covers both ZIPs and all six standalone executables. Each ZIP also has its own
  `.zip.sha256` sidecar.

### Important

- All six executables are **not Authenticode-signed**. Do not disable SmartScreen warnings;
  establish provenance with `SHA256SUMS.txt` and the GitHub artifact attestation.
- libarchive / `bsdtar.exe` is not bundled. **Import from MSYS2** in Settings automatically selects
  UCRT64 in the x64 build and CLANGARM64 in the ARM64 build. Require `iroha-zip.exe doctor` to pass
  after import.
- This is not a security-audited product. Windows 10/11 x64 desktop devices, experimental LPAC,
  screen-reader/mixed-DPI validation, and broader malformed-format/race testing remain open.

### Main changes from v0.5.0

- Standalone GZ/BZ2/XZ/Zstandard/`.Z` inputs extract as one safely named file derived from the outer
  extension. A dedicated child loads manifest-pinned libarchive DLLs, drains the full stream during
  preflight, then repeats extraction in a fresh AppContainer. Filter mismatch, decode error,
  single-file overflow, and existing output fail closed. Compound archives such as `tar.gz` retain
  the normal archive path.
- Embedded stream filenames never select the output name. Compression provides no cryptographic
  content authenticity, so successful decompression is not treated as a trust verdict.
- RAR, RAR5, LHA level 3, and BZIP2-compressed ZIPX fixtures from the official libarchive 3.8.9 tag
  are pinned with upstream license, tag object, commit, and encoded/decoded hashes. The bounded UU
  decoder requires the exact encoded hash, envelope, decoded length/hash, and three tamper self-tests.
- Windows Server 2022, Server 2025, and native Windows 11 ARM preview/extract four create formats and
  14 additional read formats with exact path/type/length/SHA-256 tree comparison. Coverage includes
  raw filter-mismatch, 32-byte-limit, and payload-corruption rejection plus standalone-gzip
  double-click shell dispatch.
- Backend replacement CI injects failure immediately after renaming the prior tree to its backup and
  requires byte-identical restoration, zero stage/backup residue, and a subsequent successful import.

See [Windows E2E](https://github.com/hjosugi/iroha-zip/blob/v0.5.1/docs/WINDOWS_E2E.md),
[ARM64 status](https://github.com/hjosugi/iroha-zip/blob/v0.5.1/docs/ARM64.md), and
[Build Status](https://github.com/hjosugi/iroha-zip/blob/v0.5.1/docs/BUILD_STATUS.md) for the measured boundary.
