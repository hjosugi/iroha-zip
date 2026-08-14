# Policy-safe archive preview and selective extraction

Updated: 2026-08-09

This document records the UX-002 prototype boundary. The CLI can now preview the exact
policy-safe tree that normal extraction would publish and can publish selected preview-relative
paths. UX-002 remains open until its prerequisite Windows/security matrices and native graphical
flow are complete.

## Why preview performs a complete temporary extraction

The main process does not load libarchive and does not parse archive headers. On Windows, a
dedicated zero-capability AppContainer child loads only manifest-pinned backend DLL candidates and
emits bounded UTF-8 pathnames through libarchive's official API; on Unix, the sandboxed backend
emits the equivalent bounded listing. The main process validates only that name stream.
The official [bsdtar manual](https://github.com/libarchive/libarchive/blob/master/tar/bsdtar.1)
defines `-t` as a filename listing to stdout and describes command-line selections as shell-style
patterns. Its `--null` option applies only to filename/pattern input through `-I` or `-T`; it does
not define a NUL-delimited structured listing output. Treating lines or verbose columns from an
attacker-controlled archive as a protocol would therefore introduce ambiguous metadata parsing.

Instead, preview intentionally pays the cost of a complete temporary extraction:

~~~text
handle-retaining input snapshot
  -> verified backend copied into an ephemeral AppContainer/LPAC workspace
  -> bounded raw-name preflight inside the same isolation boundary
  -> full bsdtar extraction with the normal safe flags
  -> the same timeout, Job Object memory/process, live file/directory/byte limits
  -> the same path, link, reparse-point, ADS, hardlink, type, and size audit
  -> SHA-256 tree fingerprint
  -> typed and sorted inventory from the audited filesystem
  -> second SHA-256 tree fingerprint
  -> destroy the workspace without publishing
~~~

The inventory contains only `file`/`directory`, byte size, and a preview-relative path. Path policy
rejects non-Unicode names, control characters, tabs, newlines, Windows aliases, absolute paths,
parent traversal, links, and special objects before any row is emitted. Archive timestamps,
owners, permissions, comments, link targets, and other untrusted metadata are not exposed.

## CLI

Preview writes one tab-separated typed row per safe entry to stdout and its bounded summary to
stderr:

~~~powershell
iroha-zip.exe preview .\archive.zip
iroha-zip.exe preview .\legacy.lzh --encoding cp932
~~~

No destination or partial publication directory is created. On non-Windows systems,
`--allow-unsandboxed` remains an explicit test-only exception, consistent with extract/create.

Selective extraction uses paths exactly as shown by preview:

~~~powershell
iroha-zip.exe extract .\archive.zip --select "docs\readme.txt"
iroha-zip.exe extract .\archive.zip --select "写真" --select "資料\index.txt"
~~~

Selectors are never forwarded to bsdtar, so they cannot become backend options or archive
patterns. iroha-zip rejects absolute, parent, empty, dot, non-normalized, duplicate,
case-insensitive duplicate, and parent/child-overlapping selectors. Every selector must exist in
the policy-safe staged payload.

## Selective publication boundary

Selection starts only after the complete staged tree has passed the normal post-extraction audit.
Before copying, iroha-zip fingerprints the entire payload, then:

1. opens selected regular files through the handle-retaining snapshot API;
2. copies selected directories through the audited tree-copy API;
3. fingerprints and audits the resulting minimal tree against the original limits;
4. fingerprints the complete source payload again and rejects any change;
5. passes only the selected tree to the existing partial-copy, Mark-of-the-Web, optional Windows
   Attachment Services, post-handoff audit, and atomic-rename publication path.

Selecting fewer entries never relaxes the archive-wide extraction limits or the final tree audit.
An unsafe unselected archive entry still rejects the entire operation. This is deliberate:
selection is a publication filter, not a way to bypass validation.

## Tradeoffs and remaining evidence

- Preview can consume as much temporary space and time as full extraction. The configured limits
  and timeout remain the only accepted bound; a fast metadata-only path is not implemented.
- The current user-facing surface is CLI output. A native searchable tree, checkbox selection,
  progress/cancellation, keyboard/screen-reader automation, and empty/archive-warning states remain
  open.
- Platform-neutral tests cover typed/sorted inventory, Unicode paths, exact file/directory
  selection, minimal output, and fail-closed unsafe/missing/duplicate/case-colliding/overlapping
  selectors. A Unix integration test runs the complete preview and selected-publication flow
  through a deterministic fake backend; it validates orchestration, not libarchive format parsing.
- Disposable Windows 10/11 tests must still cover every supported format, very large inventories,
  duplicate archive entries, malformed metadata, cancellation, cleanup, Mark-of-the-Web, trust
  handoff, and selected publication.

No preview result is a malware verdict. It only describes the tree that passed iroha-zip's current
structural policy.
