# Windows ARM64 status / Windows ARM64 対応状況

Updated: 2026-08-15

## 日本語

### 検証済みのnative境界

CIはGitHubのnative `windows-11-arm` runner上で、OS／process architectureが`Arm64`、Rust hostが
`aarch64-pc-windows-msvc`であることを必須確認してから次を実行します。

- format、全targetのtest、Clippy、release build
- 3つのiroha-zip EXEと、検証済みbackendの全EXE/DLLがPE machine `0xAA64`であること
- MSYS2 CLANGARM64の署名必須package database、exact package version/archive hash、payload byte、
  provenance、SPDX 2.3、license inventoryの完全検証
- ZIP、7z、TAR、TAR.GZの作成と別sandboxでの再展開照合
- TAR.BZ2、TAR.XZ、TAR.Zstandard、UNIX compress、単体GZ／BZ2／XZ／Zstandard／`.Z`、
  Microsoft LZX CAB、libarchive 3.8.9由来のRAR／RAR5／LHA level 3／ZIPXのpreview／展開照合
- 単体streamのfilter不一致、展開容量超過、圧縮payload破損を非公開で拒否
- 18拒否例＋1 controlとnative hardlink／ADS／junctionを含む悪性コーパス
- 通常ZIPと単体gzipのshell経路、日本語／英語それぞれ26 controlのSettings UI／backend診断
- ZipCrypto／WinZip AES-128／AES-256のnative password dialog、preview／extract tree一致、
  wrong-password／cancel非公開、秘密output非露出、一回限りchannel
- 通常AppContainerのcapability 0、loopback、timeout、memory、異常終了、loader、temp、DACL、
  7 profile/root cleanup
- LPAC成功時は同じschema-v4契約、非対応時はexact failure class、exit 2、空stdout、完全cleanup

`v0.5.0`の基準契約はcommit `947b693b96857992bf32a73366c57f537daf0aa5`の
[Actions run 31774280671](https://github.com/hjosugi/iroha-zip/actions/runs/31774280671)で合格しました。
その後、上記14追加読取形式と3拒否ケースを含む拡張契約がcommit
`27610e69f21bf85709f70a68695acc1113d22dca`の
[Actions run 31778764604](https://github.com/hjosugi/iroha-zip/actions/runs/31778764604)でServer 2環境と同時に合格し、
[push run 31778405711](https://github.com/hjosugi/iroha-zip/actions/runs/31778405711)でも独立に再実行されました。
downloadした5つのJSONは、全matrixが実行済みであること、全PEが`0xAA64`であること、通常
AppContainerがcapability 0であること、7 profile/rootと全一時rootが削除されたことを記録しています。
このWindows 11 ARM環境ではLPAC queryは`ERROR_INVALID_PARAMETER`となり、通常AppContainerへ降格せず
fail closedしました。

暗号化ZIPを含むschema-v5拡張はexact `main` commit
`9debd02e819899f8dbdfdd5281d3d0b2a68a89db`の
[Actions run 31875638650](https://github.com/hjosugi/iroha-zip/actions/runs/31875638650)で合格しました。
downloadした5 JSONを独立に確認し、3暗号方式すべてで保護された日英control、一回限りchannel、
passwordのstdout／stderr非露出、sourceと同じpreview／extract tree、wrong-password／cancel時の
destination非公開、全PEの`0xAA64` identity、完全cleanupを確認しました。
全5 JSONはraw/canonical SHA-256とartifact API digest付きの
[長期証跡snapshot](https://github.com/hjosugi/iroha-zip/tree/main/evidence/windows/31875638650/windows-arm64-native)
にも保存しています。
schema-v2 Settingsの2 reportは全26 controlsの正逆順、wrap、可視性、保存、dirty close取消を
保持します。GitHub-hosted ARM64 desktopではspawnしたprocessのforeground/global UIA focusを
取得できないため、限定した`AttachThreadInputSetFocus`／`UIAutomationFallback`経路を明記し、
実Tab／Enter／Escape検証は`false`と記録します。これは物理keyboard証跡とは主張しません。

### 配布境界

`v0.6.3`は`windows-arm64`と
`windows-x64`を別名で扱います。release workflowはnative ARM64 runnerでbuild/packageし、
次のtag-driven境界で取り違えを拒否します。

1. build直後の3 EXEをPE machine `0xAA64`で検査する。
2. ARM64名の3つの個別EXEと、ARM64 ZIP内の3 EXEを再検査する。
3. x64側は同じ箇所で`0x8664`を要求する。
4. 2 ZIP、2 sidecar、6個別EXE、全体`SHA256SUMS.txt`のexact 11 assetだけを許可する。
5. 2 ZIPと6 EXEへarch別attestation、`SHA256SUMS.txt`へ集約attestationを発行する。
6. draft時と公開後に全11 assetの大小文字を含む名前、byte長、SHA-256を照合する。

`v0.6.2`公開では[Actions run 31877964740](https://github.com/hjosugi/iroha-zip/actions/runs/31877964740)が
成功しました。事前の[非公開dry run 31877701103](https://github.com/hjosugi/iroha-zip/actions/runs/31877701103)と、
tag側[CI run 31877964744](https://github.com/hjosugi/iroha-zip/actions/runs/31877964744)も合格しています。
GitHub Releaseから11 assetを独立に再取得し、API digest／byte長、
8 checksum対象、2 sidecar、ZIP内外のx64／ARM64 PE identity、ZIPと個別EXEのbyte一致、
日英package内容とsource文書の一致、backend非同梱、annotated tag object
`d9fb686b830e24798c822b2d00e1a4d8bdbc8f37`、exact tag commit
`4c53bb0b2666486c541b47606e54cf1345f672c3`、hosted-runner限定の
9つのtag-ref attestationを確認しました。6種類のEXEはすべてPE Certificate Tableが空でした。
Release APIはlatest／stable／immutableと、exact 11 assetの公開状態を返しています。固定metadata、
全asset hash、workflow artifact digest、独立検証結果は
[v0.6.2 release snapshot](https://github.com/hjosugi/iroha-zip/tree/main/evidence/releases/v0.6.2)に保存しています。

### ARM64での導入

1. `iroha-zip-0.6.3-windows-arm64.zip`を取得し、SHA-256とattestationを確認します。
2. native ARM64版MSYS2で`mingw-w64-clang-aarch64-libarchive`を導入します。
3. ARM64版の設定画面で「MSYS2から取り込む」を選びます。設定画面はCLANGARM64を自動指定します。
4. CLI自動化では次を使用します。

```powershell
.\scripts\export-msys2-backend.ps1 `
  -Msys2Root C:\msys64 `
  -Environment CLANGARM64
.\iroha-zip.exe doctor
```

backend binaryは公式ZIPへ同梱しません。利用者が信頼するMSYS2 installから取り込みます。

### 残る範囲

- 自動ARM64実機証拠はGitHubのWindows 11 ARM runnerです。Windows 10 ARM64や複数の市販機種、
  endpoint security製品、mixed-DPI／screen readerは未検証です。
- 実験的LPACはこのrunnerでも利用可能と確認できていません。通常AppContainerが既定で、暗黙に降格しません。
- ARM64対応はセキュリティ監査やlibarchive自体の安全性を意味しません。

## English

### Validated native boundary

CI first requires `Arm64` for both OS and process architecture and
`aarch64-pc-windows-msvc` for the Rust host on GitHub's native `windows-11-arm` runner. It then runs:

- formatting, every-target tests, Clippy, and a release build;
- PE machine `0xAA64` checks for all three iroha-zip executables and every verified backend EXE/DLL;
- complete MSYS2 CLANGARM64 verification of required-signature databases, exact package versions and
  archive hashes, payload bytes, provenance, SPDX 2.3, and the license inventory;
- ZIP, 7z, TAR, and TAR.GZ creation followed by independent-sandbox re-extraction comparison;
- preview/extraction comparison for TAR.BZ2, TAR.XZ, TAR.Zstandard, UNIX compress, standalone
  GZ/BZ2/XZ/Zstandard/`.Z`, a Microsoft LZX CAB, and libarchive 3.8.9 RAR/RAR5/LHA-level-3/ZIPX fixtures;
- non-publication for standalone-stream filter mismatch, expanded-byte-limit overflow, and compressed-payload corruption;
- the hostile corpus with 18 rejects, one control, and native hardlink/ADS/junction cases;
- normal-ZIP and standalone-gzip shell handling, plus both Japanese and English 26-control
  Settings/backend-diagnosis paths;
- native password-dialog, preview/extract tree matching, wrong-password/cancel non-publication,
  secret output absence, and one-use-channel checks for ZipCrypto, WinZip AES-128, and AES-256;
- normal AppContainer with zero capabilities, loopback/timeout/memory/crash/loader/temp/DACL checks,
  and seven profile/root cleanups; and
- the same schema-v4 contract on LPAC success, or exact failure class, exit 2, empty stdout, and full
  cleanup when LPAC is unsupported.

The baseline contract passed for the `v0.5.0` commit
`947b693b96857992bf32a73366c57f537daf0aa5` in
[Actions run 31774280671](https://github.com/hjosugi/iroha-zip/actions/runs/31774280671).
The expanded contract above, including all 14 additional reads and three negative cases, then passed
for commit `27610e69f21bf85709f70a68695acc1113d22dca` alongside both Server environments in
[Actions run 31778764604](https://github.com/hjosugi/iroha-zip/actions/runs/31778764604) and independently
repeated in [push run 31778405711](https://github.com/hjosugi/iroha-zip/actions/runs/31778405711).
The five downloaded JSON files record every matrix as executed, `0xAA64` for every PE, normal
AppContainer with zero capabilities, and removal of all seven profiles/roots and every temporary root.
The LPAC query returned `ERROR_INVALID_PARAMETER` on that Windows 11 ARM environment and failed closed
without a normal-AppContainer fallback.

The schema-v5 encrypted-ZIP expansion passed for exact `main` commit
`9debd02e819899f8dbdfdd5281d3d0b2a68a89db` in
[Actions run 31875638650](https://github.com/hjosugi/iroha-zip/actions/runs/31875638650). Independent
checks of all five downloaded JSON reports confirmed the protected bilingual control, one-use
channel, password absence from stdout/stderr, source-identical preview/extraction trees for all three
encryption modes, wrong-password/cancel destination absence, `0xAA64` identity for every PE, and
complete cleanup.
All five reports are also retained with raw/canonical SHA-256 values and the artifact API digest in the
[durable evidence snapshot](https://github.com/hjosugi/iroha-zip/tree/main/evidence/windows/31875638650/windows-arm64-native).
The two schema-v2 Settings reports preserve the exact 26-control forward/reverse order, wrap,
visibility, save, and dirty-close cancellation actions. Because the GitHub-hosted ARM64 desktop does
not expose foreground/global UIA focus for the spawned process, they identify the bounded
`AttachThreadInputSetFocus`/`UIAutomationFallback` path and record real Tab, Enter, and Escape
verification as false; this is not claimed as physical-keyboard evidence.

### Distribution boundary

`v0.6.3` uses separately named
`windows-arm64` and `windows-x64` assets. The release workflow builds/packages on a native ARM64
runner and rejects architecture confusion at these tag-driven boundaries:

1. All three direct build outputs must have PE machine `0xAA64`.
2. The three ARM64 standalone assets and all three executables inside the ARM64 ZIP are rechecked.
3. The corresponding x64 boundaries require `0x8664`.
4. Only the exact 11-asset set is accepted: two ZIPs, two sidecars, six standalone EXEs, and one
   combined `SHA256SUMS.txt`.
5. Per-architecture attestations cover both ZIPs and six EXEs; a combined attestation covers the inventory.
6. Draft and published readback compare exact case-sensitive names, byte lengths, and SHA-256 for all assets.

For the `v0.6.2` publication,
[Actions run 31877964740](https://github.com/hjosugi/iroha-zip/actions/runs/31877964740)
succeeded after [non-publishing dry run 31877701103](https://github.com/hjosugi/iroha-zip/actions/runs/31877701103)
and alongside passing tag [CI run 31877964744](https://github.com/hjosugi/iroha-zip/actions/runs/31877964744).
All 11 assets were independently downloaded from the GitHub Release. API digests and byte lengths,
eight checksum subjects, two sidecars, x64/ARM64 PE
identities inside and outside the ZIPs, ZIP-to-standalone byte matches, bilingual package content,
checked-in source documents, backend non-inclusion, annotated tag object
`d9fb686b830e24798c822b2d00e1a4d8bdbc8f37`, exact tag commit
`4c53bb0b2666486c541b47606e54cf1345f672c3`, and nine hosted-runner-only tag-ref attestations all
matched. All six distinct executables had empty PE Certificate Tables. The Release
API reports it as latest, stable, immutable, and complete with the exact 11-asset inventory. Fixed
metadata, all asset hashes, workflow artifact digests, and independent results are retained in the
[v0.6.2 release snapshot](https://github.com/hjosugi/iroha-zip/tree/main/evidence/releases/v0.6.2).

### ARM64 setup

1. Download `iroha-zip-0.6.3-windows-arm64.zip` and verify its SHA-256 and attestation.
2. Install `mingw-w64-clang-aarch64-libarchive` in native ARM64 MSYS2.
3. Choose **Import from MSYS2** in the ARM64 Settings build. It selects CLANGARM64 automatically.
4. For CLI automation, use:

```powershell
.\scripts\export-msys2-backend.ps1 `
  -Msys2Root C:\msys64 `
  -Environment CLANGARM64
.\iroha-zip.exe doctor
```

The official ZIP does not bundle backend binaries; import them from an MSYS2 installation you trust.

### Remaining scope

- Automated ARM64 device evidence comes from GitHub's Windows 11 ARM runner. Windows 10 ARM64,
  multiple retail devices, endpoint-security products, mixed DPI, and screen readers remain untested.
- Experimental LPAC was not available on this runner. Normal AppContainer remains the default and
  there is no silent downgrade.
- ARM64 support is not a security audit and does not establish the safety of libarchive itself.

Primary references:

- [Rust Windows MSVC platform support](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)
- [GitHub-hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [MSYS2 ARM64 support](https://www.msys2.org/docs/arm64/)
- [MSYS2 CLANGARM64 libarchive package](https://packages.msys2.org/packages/mingw-w64-clang-aarch64-libarchive)
