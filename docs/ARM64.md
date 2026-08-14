# Windows ARM64 status / Windows ARM64 対応状況

Updated: 2026-08-14

## 日本語

### 現在の検証境界

CI は GitHub の native `windows-11-arm` runner 上で、Rust host が
`aarch64-pc-windows-msvc` であることを確認してから次を実行します。

- format、全 target の test、Clippy、release build
- `iroha-zip.exe`、`iroha-zip-settings.exe`、`iroha-zip-shell.exe` の3本が PE machine
  `0xAA64` であること
- native ARM64 processによる通常AppContainer、capability 0、loopback denial、timeout、
  Job memory limit、異常終了、loader failure、process temp、staging DACL、7 profile/root cleanup
- 実行binaryのSHA-256とisolation probeのSHA-256が一致すること

このjobが保存するJSONは診断証跡です。ARM64のbinary、ZIP、checksumをReleaseへ公開しません。

### 未完のbackend境界

現在の署名検証付きexporterはMSYS2 UCRT64 x64を対象にしています。MSYS2には
CLANGARM64版libarchiveがありますが、同じpackage/version/evidence contractと全書庫matrixをまだ
通していません。ARM64 process内で動く専用UTF-8 listing childはx64 DLLをloadできないため、x64
backendをそのままARM64 packageへ組み合わせません。

ARM64 Releaseを追加する前に、次がすべて必要です。

1. CLANGARM64 package署名、依存関係、license、SPDX、payload hashを現在のx64 exporterと同じ強さで検証する。
2. native ARM64 backendでcreate/read、悪性コーパス、shell、settings、LPAC/fail-closed matrixを通す。
3. `windows-arm64` を含む別名ZIP/EXE/checksumを作り、x64 assetとの取り違えを機械的に拒否する。
4. 未署名表示、SHA-256、attestation、immutable-release検証をx64と独立に通す。

この条件を満たすまでは、公開済み`v0.4.0`はWindows x64専用であり、ARM64 native Releaseは未提供です。

## English

### Current validation boundary

CI runs on GitHub's native `windows-11-arm` runner. After requiring the Rust host to be
`aarch64-pc-windows-msvc`, it checks:

- formatting, every-target tests, Clippy, and a release build;
- PE machine `0xAA64` on all three iroha-zip executables;
- normal AppContainer, zero capabilities, loopback denial, timeout, Job memory limit, abnormal
  termination, loader failure, process temp, staging DACL, and seven profile/root cleanups from a
  native ARM64 process; and
- equality between the executed release binary hash and the isolation-probe hash.

The job retains diagnostic JSON only. It does not publish ARM64 binaries, a ZIP, or checksums as
release assets.

### Backend work still required

The supported, signature-verifying exporter currently targets MSYS2 UCRT64 x64. MSYS2 provides a
CLANGARM64 libarchive package, but that package has not passed the same package/version/evidence
contract and complete archive matrix. The dedicated UTF-8 listing child loads libarchive in-process;
a native ARM64 process cannot simply load the x64 DLL bundle. iroha-zip therefore does not combine
the x64 backend with a nominally native ARM64 package.

Before an ARM64 release is added, all of the following must pass:

1. CLANGARM64 package signatures, dependencies, licenses, SPDX, and payload hashes under a contract
   as strong as the x64 exporter.
2. Native ARM64 create/read, hostile-corpus, shell, Settings, and LPAC/fail-closed matrices.
3. Separately named `windows-arm64` ZIP/EXE/checksum assets with mechanical x64/ARM64 mix-up rejection.
4. Independent unsigned disclosure, SHA-256, attestation, and immutable-release verification.

Until then, published `v0.4.0` is Windows x64 only and no native ARM64 release is offered.

Primary references:

- [Rust Windows MSVC platform support](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)
- [GitHub-hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [MSYS2 ARM64 support](https://www.msys2.org/docs/arm64/)
- [MSYS2 CLANGARM64 libarchive package](https://packages.msys2.org/packages/mingw-w64-clang-aarch64-libarchive)
- [Microsoft Windows process interoperability](https://learn.microsoft.com/en-us/windows/win32/winprog64/process-interoperability)
