# Malicious archive regression corpus

Updated: 2026-08-10

This document defines the SAFE-002 regression corpus and its evidence contract. The generator and Windows workflow are implemented on the current local branch, but that branch has not yet produced a passing GitHub Actions artifact. Source inspection and cross-compilation are not substitutes for a real Windows result.

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

1. Run verified `bsdtar -t` inside the same zero-capability AppContainer/LPAC boundary.
2. Monitor the listing logs, timeout, memory, and temporary tree. The minimal backend environment fixes `LANG` and `LC_ALL` to `C.UTF-8` on Unix and the [UCRT-supported `.UTF8` code page](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/setlocale-wsetlocale?view=msvc-170#utf-8-support) on Windows; listing stdout is capped at 64 MiB and must be UTF-8.
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

The artifact upload runs even after a failing test, but a panic before report initialization may leave no report. Artifact retention is 14 days. The JSON is diagnostic evidence, not a signature, malware verdict, or release attestation.

## Remaining SAFE-002 work

The implemented corpus does not close SAFE-002 until both fixed-label Windows jobs produce reviewed passing evidence. Further work remains for:

1. Windows 10 and Windows 11 x64 runs outside the Server-only hosted matrix;
2. format-specific hostile RAR/RAR5, LHA/LZH, CAB, ZIPX, cpio, ISO, and raw-stream samples that are independently redistributable;
3. invalid/legacy filename encodings and names containing embedded control/newline bytes;
4. nested archive recursion, decompressor CPU bombs, extreme compression ratios, and memory-pressure combinations;
5. malformed central-directory, ZIP64, PAX, extended sparse-map, and truncated archive cases;
6. crash, loader-failure, cancellation, disk-full, and reparse-race exit paths;
7. long-term reviewed evidence retention beyond ephemeral Actions artifacts.

Do not claim complete format coverage, malware safety, or Windows desktop certification from this matrix.
