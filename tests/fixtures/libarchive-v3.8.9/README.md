# Pinned libarchive read fixtures

These four UUencoded reference archives are copied without modification from the official
libarchive `v3.8.9` source tree. They are benign test inputs, not backend executables or libraries.
They are included with the E2E harness so a packaged source-validation run remains self-contained.

- Annotated tag object: `f1f785cc218bb05876c54680f10d3d4e54575ea2`
- Tag commit: `27cbc7827172698143e440801fc0ba39ccb4f1f5`
- Upstream directory:
  <https://github.com/libarchive/libarchive/tree/v3.8.9/libarchive/test>
- License: upstream BSD-2-Clause terms reproduced in [`COPYING`](COPYING)

| UUencoded source | Encoded SHA-256 | Decoded bytes | Decoded SHA-256 | Safe expected tree |
| --- | --- | ---: | --- | --- |
| `test_read_format_rar_windows.rar.uu` | `d934dc7895212d468a2d44111e77d95536d79c3c9eae56690667d483ae9419d7` | 814 | `8d689455e9ecd92c19426604e2360b5ef8eb023890fe46aabbe2864260b70fc9` | two directories, two text files, one ordinary `.lnk` file |
| `test_read_format_rar5_stored.rar.uu` | `ec73ba623a8e8eee4909dcdf45f0526ff9adc1d856d054ce742e6c1ba1fb5fa8` | 109 | `35d75e315d164d2e329afc28f7d844f013271b4fcffd4ddd78efcdd114a383a7` | one regular text file |
| `test_read_format_lha_header3.lzh.uu` | `4bcbe7e493bca4d79eb21d1c2dc8031190b1b41fb487fc61e71c757d3232b33f` | 548 | `d36f9beaf7d1aa482315e810c8cfca327975ffd31a05082004102327310e419d` | two directories and two regular text files |
| `test_read_format_zip_bzip2.zipx.uu` | `7baa771d86ac20a4d1ed079be94088c1628d8a513843981f353bda27ba36d359` | 708 | `373ec637744c762bb6c69c2c4f6cc2d9dad85ed5d4662b0ffb9373077dbf01a5` | one regular file |

The Windows E2E harness rejects any source/hash/envelope/decoded-length drift before passing an
archive to iroha-zip. It then runs both `preview` and `extract` through the normal AppContainer
path and compares the extracted paths, object kinds, lengths, and content hashes with a hardcoded
expected inventory. Host-side `bsdtar` is not used to inspect these inputs in CI.
