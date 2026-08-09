# Backend manifest v1

`backend-manifest.tsv` fixes the exact executable and file hashes accepted as an iroha-zip backend bundle. Runtime verification rejects a bundle unless its regular-file tree exactly matches the manifest and every SHA-256 digest matches.

## Encoding and limits

- The file must be valid UTF-8 and at most 4 MiB.
- The first line must be `IROHA-ZIP-BACKEND-MANIFEST<TAB>1`.
- Empty lines and lines beginning with `#` are ignored after the header.
- At most 4096 hashed-file records are accepted.
- Each relative path is at most 4096 UTF-8 bytes and 64 components.
- CRLF and LF line endings are accepted.

## Records

Exactly one executable record is required:

```text
executable<TAB>bin/bsdtar.exe
```

Every backend file has one hash record:

```text
sha256<TAB>0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef<TAB>bin/bsdtar.exe
```

The executable path must also have a hash record. Digests may use upper- or lowercase hexadecimal input and are normalized to lowercase in memory. Unknown record types, extra fields, malformed hashes, duplicate paths, and multiple executable records are rejected.

## Path rules

Manifest paths use `/` separators and must be normalized relative paths. iroha-zip rejects:

- absolute, drive-prefixed, parent, current-directory, empty, or backslash-separated paths;
- empty components and paths beyond the byte or depth limits;
- control characters, NTFS stream syntax, Windows-invalid characters, and trailing dots or spaces;
- Windows device names such as `CON`, `NUL`, `COM1`, and `LPT1`.

After parsing, iroha-zip enumerates the executable payload without following links. Symlinks, reparse points, special files, missing files, extra files, unsafe file identities, and digest mismatches fail verification. The only reserved non-payload entry is `.iroha-zip-evidence/`; when present, its entire separate tree is validated against the provenance, SPDX, and license inventories before the backend is accepted. Evidence files are not copied into the AppContainer.

## Trust boundary

The v1 manifest proves consistency with the bytes recorded during local import. Source classification, enforced MSYS2/pacman signature policy, package archive digests, SPDX 2.3 SBOM, payload ownership, and license evidence are recorded and independently checked as described in [Backend provenance, SBOM, and license evidence](BACKEND_EVIDENCE.md).
