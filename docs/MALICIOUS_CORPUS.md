# Malicious archive regression corpus

Updated: 2026-08-15

This document defines the SAFE-002 regression corpus and its evidence contract. Both fixed-label Windows jobs and the native Windows 11 ARM64 job produced reviewed passing evidence in [Actions run 31875638650](https://github.com/hjosugi/iroha-zip/actions/runs/31875638650) from exact `main` commit `9debd02e819899f8dbdfdd5281d3d0b2a68a89db`: the benign control was accepted, all 18 hostile archives were rejected without publication, all three native policy fixtures were rejected, and each temporary root was removed. Canonical copies and both source/canonical SHA-256 inventories are retained in the [durable evidence snapshot](https://github.com/hjosugi/iroha-zip/tree/main/evidence/windows/31875638650); no generated archive is retained.

## Distribution and retention policy

The corpus is generated deterministically from project-authored Rust source in `tests/malicious_archive_corpus.rs`. No third-party sample, malware, executable payload, or downloaded archive is embedded in the repository. The generated entries contain only short inert text or repeated bytes.

Generated ZIP/TAR files are still security-test inputs. The workflow creates them only below a unique disposable runner directory, removes that directory before the test can pass, and never uploads the files. Do not attach generated archives to issues, pull requests, releases, chat, or ordinary CI artifacts. Upload only the bounded JSON evidence described below.

For intentional local format review, set `IROHA_ZIP_CORPUS_MATERIALIZE_DIR` to a path that does not exist and run the non-ignored generator test. The test refuses to reuse an existing directory. Review and delete that directory locally; never treat it as a release artifact.

## Generated archive matrix

The generator creates one benign control and 18 inputs that must fail before the requested destination exists:

| ID | Container | Threat | Required result |
|---|---|---|---|
| `control-zip` | ZIP | Benign harness control | Extract `ok.txt` with exact content. |
| `zip-parent-traversal` | ZIP | `..` traversal | Reject; no destination. |
| `zip-absolute-path` | ZIP | Leading `/` | Reject; no destination. |
| `zip-drive-path` | ZIP | Drive-prefixed path | Reject; no destination. |
| `zip-unc-path` | ZIP | UNC path | Reject; no destination. |
| `zip-ads-name` | ZIP | NTFS ADS syntax | Reject; no destination. |
| `zip-device-name` | ZIP | Windows device name | Reject; no destination. |
| `zip-trailing-dot-alias` | ZIP | Win32 trailing-dot alias | Reject; no destination. |
| `zip-trailing-space-alias` | ZIP | Win32 trailing-space alias | Reject; no destination. |
| `zip-invalid-character` | ZIP | Windows-invalid character | Reject; no destination. |
| `zip-duplicate-name` | ZIP | Duplicate member overwrite | Reject; no destination. |
| `zip-symlink` | ZIP | Unix-mode symbolic link | Reject; no destination. |
| `tar-symlink` | ustar | Symbolic link | Reject; no destination. |
| `tar-hardlink` | ustar | Hardlink/duplicate identity | Reject; no destination. |
| `tar-depth-limit` | ustar | Path-depth bomb | Reject; no destination. |
| `tar-file-count-limit` | ustar | File-count bomb | Reject; no destination. |
| `tar-single-file-limit` | ustar | Single-file expansion | Reject; no destination. |
| `tar-total-size-limit` | ustar | Aggregate expansion | Reject; no destination. |
| `tar-sparse-expansion` | old GNU tar | 1 stored byte with a 2 MiB logical size | Reject; no destination. |

The ZIP writer stores data without compression and emits CRC-32, local records, central records, UTF-8 flags, and Unix mode attributes. The tar writers emit checksummed 512-byte ustar or old-GNU headers. Unit tests require unique IDs, a 4 MiB maximum archive size, valid container signatures, and deterministic SHA-256 values. Local review confirmed that libarchive 3.8.9 and GNU tar 1.35 recognize the sparse sample as a 2,097,152-byte logical file; this observation is not Windows evidence.

## Preflight and publication boundary

Post-extraction inspection cannot recover information that an archive backend normalized or overwrote. In particular, libarchive may remove an absolute, UNC, or drive prefix, and repeated members may collapse to one output path. iroha-zip therefore performs two sandboxed backend operations:

1. Run raw-name preflight inside the same zero-capability AppContainer/LPAC boundary. Unix uses verified `bsdtar -t`. Windows copies and seals a dedicated iroha-zip child, derives DLL candidates only from the verified backend manifest, rechecks the child token, loads libarchive with DLL-directory/System32-only search, and calls `archive_entry_pathname_utf8`.
2. Monitor the listing logs, timeout, memory, and temporary tree. The minimal backend environment fixes `LANG` and `LC_ALL` to `C.UTF-8` on Unix. On Windows, the sandbox backend EXE also receives a byte-verified UTF-8 `activeCodePage` manifest for its create/extract operations; `.utf8` locale hints remain in the minimal environment for compatible CRT builds. Listing stdout is capped at 64 MiB and must be UTF-8.
3. Validate every raw listed member for relative normalized separators, `.`/`..`, Windows-invalid components, device names, ADS syntax, depth, path length, and case-insensitive duplicate aliases.
4. Only after preflight succeeds, run the existing sandboxed extraction.
5. Continue live resource monitoring, full post-extraction filesystem audit, fingerprint checks, optional trust handoff, and atomic publication.

The Rust process does not parse ZIP, TAR, RAR, or other archive structures. Archive parsing remains in the sandboxed backend; Rust parses only a bounded line-oriented name stream. Link targets and filesystem objects remain subject to the post-extraction reparse/link/ADS/identity audit.

## Native filesystem fixtures

The disposable Windows test also constructs three filesystem-only fixtures after the archive matrix:

- two names sharing one hardlink identity;
- a regular file with an NTFS alternate data stream;
- a directory containing a junction/reparse point.

Each fixture must return the policy error class from `audit_tree`. They are generated below the same unique temporary root and deleted before success.

## Windows execution

The fixed-label `windows-2022` and `windows-2025` jobs set:

```text
IROHA_ZIP_CORPUS_EXECUTABLE=<release iroha-zip.exe>
IROHA_ZIP_CORPUS_BACKEND=<verified MSYS2 backend>
IROHA_ZIP_CORPUS_EVIDENCE=<runner temp>/malicious-corpus.json
```

They then run the ignored Windows test. The configuration uses a 4 MiB input cap, eight files, eight directories, 160 KiB total output, 128 KiB per file, depth eight, and 512 UTF-8 path bytes. The benign control proves the harness and verified backend can publish a valid archive under those limits.

`malicious-corpus.json` has `schemaVersion: 1` and records:

- status and failure text;
- executable and backend-manifest SHA-256;
- each sample ID, threat, archive SHA-256/length, expected result, exit code, rejection class, and destination-publication boolean;
- each native policy fixture and returned error class;
- whether the complete temporary root was removed.

The artifact upload runs even after a failing test, but a panic before report initialization may leave no report. Artifact retention is 90 days, the maximum configured and permitted for this public repository. The JSON is diagnostic evidence, not a signature, malware verdict, or release attestation.

## Verified scope and further work

The implemented scope meets SAFE-002's acceptance criteria. The passing Server evidence does not cover these useful extensions:

1. Windows 10 and Windows 11 x64 runs outside the Server-only hosted matrix;
2. format-specific hostile RAR/RAR5, LHA/LZH, CAB, ZIPX, cpio, ISO, and raw-stream samples that are independently redistributable;
3. invalid/legacy filename encodings and names containing embedded control/newline bytes;
4. nested archive recursion, decompressor CPU bombs, extreme compression ratios, and memory-pressure combinations;
5. malformed central-directory, ZIP64, PAX, extended sparse-map, and truncated archive cases;
6. crash, loader-failure, cancellation, disk-full, and reparse-race exit paths;
7. periodic promotion of additional reviewed runs beyond the one durable snapshot and rolling
   90-day Actions artifact window.

Do not claim complete format coverage, malware safety, or Windows desktop certification from this matrix.
