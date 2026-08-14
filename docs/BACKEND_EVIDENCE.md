# Backend provenance, SBOM, and license evidence

iroha-zip keeps executable backend payloads and their supply-chain evidence at one atomic installation boundary. `backend-manifest.tsv` hashes only the executable payload. The reserved `.iroha-zip-evidence/` directory is never copied into the AppContainer, but when it exists its complete tree is validated before the backend is accepted.

The Windows runtime never changes this installed boundary. After validating and hash-checking a sandbox copy, it replaces only that disposable EXE's manifest resource with fixed `asInvoker`, long-path, and UTF-8 process-code-page declarations, then reads the resource back byte-for-byte before launch. It neither changes nor redistributes the evidence-covered source bytes. Raw-name preflight derives its DLL candidate list only from the same verified payload manifest and loads those candidates inside a zero-capability AppContainer child through a restricted DLL search. Names come from libarchive's official UTF-8 pathname API, avoiding the [libarchive 3.8.6+ Windows current-code-page regression](https://github.com/libarchive/libarchive/issues/3063) without adding an unrecorded backend file.

## Supported sources

The supported binary sources are the current `ucrt64` x64 and `clangarm64` native ARM64 package
sets exported from an up-to-date MSYS2 installation by `scripts/export-msys2-backend.ps1`.
The source record, repository, and package-name prefix are a closed pair: UCRT64 evidence cannot
claim CLANGARM64 packages or vice versa.

```powershell
# x64 (default)
.\scripts\export-msys2-backend.ps1 -Environment UCRT64

# native ARM64 on Windows 11 ARM64
.\scripts\export-msys2-backend.ps1 -Environment CLANGARM64
```

The native Settings application supplies this selector automatically: the x64 build requests UCRT64
and the ARM64 build requests CLANGARM64. This prevents the normal UI import path from installing a
supported package set for the wrong executable architecture.

The exporter:

1. resolves `bsdtar.exe` and its transitive same-environment DLLs with `ldd`;
2. identifies the installed package owning every selected file with `pacman -Qqo`;
3. creates an isolated pacman database configured with `SigLevel = Required TrustedOnly` and
   refreshes its signed `msys` plus selected `ucrt64` or `clangarm64` database;
4. requires the installed version to equal the freshly signature-verified repository version and takes all source metadata from that isolated database;
5. downloads each exact package through pacman under the same required/trusted-only policy;
6. compares the downloaded archive SHA-256 with the signed repository metadata and records the detached package signature that pacman verified and retained;
7. runs pacman's detailed installed-file check and compares every selected installed file with the corresponding file extracted from the verified archive;
8. requires every archive-derived EXE/DLL to be x64 PE machine `0x8664` for UCRT64 or ARM64
   machine `0xAA64` for CLANGARM64; and
9. imports those bytes and the package's standard license files.

This follows the documented MSYS2 package ownership and license queries and pacman's signature policy. See the [MSYS2 package-management guide](https://www.msys2.org/docs/package-management/), [MSYS2 package tips](https://www.msys2.org/docs/package-management-tips/), and [`pacman.conf(5)` signature checking](https://man.archlinux.org/man/pacman.conf.5.en#PACKAGE_AND_DATABASE_SIGNATURE_CHECKING).

The exporter fails rather than silently treating a stale installed version, another repository, an unsigned package, a weak signature policy, an archive digest mismatch, or installed/archive byte drift as supported provenance.

Each bash, `ldd`, and pacman child is launched through MSYS2 coreutils `timeout`. The default
per-command limit is 180 seconds; automation may set `-CommandTimeoutSeconds` from 30 through 1800
seconds when a deliberately slow managed mirror requires it. Exit 124/137 is reported as a timeout,
the temporary evidence tree is removed, and no destination bundle is installed. Named progress
boundaries identify dependency resolution, signed-database refresh, metadata resolution, package
download, and payload extraction without printing package signatures or file contents.

## Unsupported local bundles

An arbitrary local bundle cannot prove its distributor or acquisition path. The settings screen shows a dedicated warning and requires a second confirmation. The automation script fails unless the caller explicitly adds:

```powershell
.\scripts\install-backend.ps1 `
  -SourceDirectory C:\path\to\bundle `
  -AllowUnsupportedSource
```

The generated provenance permanently records `supported: false`, `unverified`, and `explicit-user-accepted-local-bundle`. `doctor` and `verify-backend-evidence` print an explicit warning. Private packaging rejects this source by default; the separate `-AllowUnsupportedBackendSource` switch is required in addition to `-IncludeBackend`.

## Evidence layout

```text
backend/libarchive/
  backend-manifest.tsv
  bsdtar.exe
  *.dll
  .iroha-zip-evidence/
    backend-provenance.json
    backend.spdx.json
    backend-license-inventory.json
    THIRD-PARTY-NOTICES.md
    licenses/<package-id>/...
```

- `backend-provenance.json` records the UTC import time, source classification, enforced verification method, installed `msys2-keyring` version, manifest digest, package versions, repository/archive hashes, verified detached signatures, distributor license metadata, and exact package owner of every payload file.
- `backend.spdx.json` is an SPDX 2.3 JSON document. It includes every payload file and SHA-256, one analyzed SPDX package per recorded owner, SPDX package verification codes, and exact `DESCRIBES`/`CONTAINS` relationships. See the [SPDX 2.3 package rules](https://spdx.github.io/spdx-spec/v2.3/package-information/) and [relationship rules](https://spdx.github.io/spdx-spec/v2.3/relationships-between-SPDX-elements/).
- `backend-license-inventory.json` repeats the exact payload-to-package mapping and hashes the generated notice and every copied license file.
- `THIRD-PARTY-NOTICES.md` presents the recorded package/version/license data and makes the unsupported-source warning visible without a JSON reader. It preserves distributor metadata and is not an independent legal conclusion.

The validator requires all three machine-readable views to agree exactly with the manifest. It rejects unknown JSON fields, unsupported schema versions, duplicate records, unsafe Windows paths, missing or extra files/directories, symlinks/reparse points, digest drift, ownership drift, unrelated packages, invalid SPDX package verification codes and relationships, more than 256 packages, more than 1024 evidence files, any JSON document over 4 MiB, or an evidence tree over 32 MiB.

## Verification and packaging

```powershell
.\iroha-zip.exe doctor
.\iroha-zip.exe verify-backend-evidence .\backend\libarchive
.\iroha-zip.exe verify-backend-evidence .\backend\libarchive --require-supported
```

Normal public release packages remain backend-free. A private package can embed a verified backend and its evidence:

```powershell
.\scripts\build-release.ps1 -IncludeBackend
```

The build validates the source backend, the copied release tree, and the completed ZIP after re-expansion. At every boundary it checks the payload, provenance, SPDX SBOM, license inventory, notice, and license files. To package a deliberately unsupported local source after independent review, both risk switches are required:

```powershell
.\scripts\build-release.ps1 `
  -IncludeBackend `
  -AllowUnsupportedBackendSource
```

## Scope limits

The evidence describes the import event and the exact bytes accepted then. It does not make a package vulnerability-free, turn distributor metadata into legal advice, or protect a machine whose administrator, MSYS2 keyring, pacman executable, or iroha-zip scripts are already compromised. Public releases therefore continue to exclude third-party backend binaries.
