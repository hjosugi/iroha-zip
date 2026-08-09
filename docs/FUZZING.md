# Reproducible fuzzing and regression promotion

Updated: 2026-08-10

QA-001 covers parser and normalization code that consumes attacker-controlled bytes before or
around the archive backend. The fuzz workspace is intentionally separate from the release
workspace: production binaries do not link `libfuzzer-sys`, and the small `fuzzing` feature only
exposes test harness entry points.

## Pinned environment

- Rust `nightly-2026-08-01`
- `cargo-fuzz` `0.13.2`
- `libfuzzer-sys` `0.4.13`, pinned exactly in `fuzz/Cargo.toml`
- `fuzz/Cargo.lock`, committed and checked before each scheduled run

The fuzz-only `libfuzzer-sys` graph is checked separately with `fuzz/deny.toml`. Its NCSA-licensed
LLVM runtime is build/test tooling and is not linked into or shipped with production artifacts.

`cargo-fuzz` needs a nightly compiler and sanitizer support. The scheduled workflow therefore
runs on Linux, installs the pinned nightly plus `rust-src`, fetches the locked graph once, and
then sets `CARGO_NET_OFFLINE=true`. The [cargo-fuzz README](https://github.com/rust-fuzz/cargo-fuzz)
and [Rust Fuzz Book CI guidance](https://rust-fuzz.github.io/book/cargo-fuzz/ci.html) are the
upstream operational references. The time and artifact switches follow the
[LLVM libFuzzer options](https://llvm.org/docs/LibFuzzer.html#options).

## Targets and invariants

| Target | Input surface | Checked invariant |
| --- | --- | --- |
| `backend_manifest` | bounded manifest bytes | any accepted manifest has a bounded non-empty file map and a hashed executable |
| `windows_paths` | arbitrary platform filename/path bytes | accepted relative paths contain only bounded, individually valid normal components; non-Unicode names fail safely |
| `archive_name` | UTF-8 archive filename | the derived destination component is always non-empty and Windows-safe |
| `command_line` | arbitrary UTF-16 units | accepted quoting round-trips exactly; command lines have one terminal NUL and obey the 32,767-unit limit |
| `config_round_trip` | configuration bytes | every accepted and validated configuration serializes, reparses, revalidates, and remains equal |

The command-line encoder is platform-neutral but is used by the Windows `CreateProcessW` path.
It rejects an empty or quoted program name, interior NUL units, and oversized command lines before
calling Windows. Its quoting rules match Microsoft's documented
[C runtime argument parsing](https://learn.microsoft.com/en-us/cpp/c-language/parsing-c-command-line-arguments?view=msvc-170),
while the overall length bound comes from
[`CreateProcessW`](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw).

## Bounded CI campaign

`.github/workflows/fuzz.yml` runs weekly and on manual dispatch with read-only repository
permissions. It runs each of the five targets for at most 45 seconds, limits input to 65,536
bytes, limits a single execution to 10 seconds, caps RSS at 2 GiB, and caps the whole job at 20
minutes. Checked-in seeds are copied to `$RUNNER_TEMP`, so fuzzing cannot rewrite the source
corpus. A failure uploads `fuzz/artifacts/` for 14 days.

## Local run

Install the exact tools, then run one bounded target from the repository root:

```text
rustup toolchain install nightly-2026-08-01 --profile minimal --component rust-src
cargo +1.97.1 install cargo-fuzz --version 0.13.2 --locked
cargo +nightly-2026-08-01 fuzz check
cargo +nightly-2026-08-01 fuzz run backend_manifest -- \
  -max_len=65536 -max_total_time=60 -rss_limit_mb=2048 -timeout=10
```

Do not commit the generated working corpus or raw crash samples.

## Reproduce, minimize, and promote

1. Download the failing artifact and reproduce it once:

   ```text
   cargo +nightly-2026-08-01 fuzz run TARGET PATH_TO_ARTIFACT -- -runs=1
   ```

2. Minimize the reproducing input and use the minimized path reported by `cargo-fuzz`:

   ```text
   cargo +nightly-2026-08-01 fuzz tmin TARGET PATH_TO_ARTIFACT -- -max_total_time=300
   ```

3. Fix the defect, then promote only the minimized input:

   ```text
   ./scripts/promote-fuzz-regression.ps1 -Target TARGET -Artifact PATH_TO_MINIMIZED_ARTIFACT
   ```

   The script rejects inputs over 65,536 bytes and stores the bytes as
   `fuzz/regressions/TARGET/<sha256>.bin`, avoiding filename collisions and duplicate samples.

4. Run the deterministic gate:

   ```text
   cargo test --locked --features fuzzing --test fuzz_regressions
   ```

The ordinary Linux CI job runs this gate on every push and pull request. A fuzzing failure is not
closed until its minimized input is committed and this deterministic test passes with the fix.
