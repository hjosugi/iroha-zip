# Contributing to iroha-zip

iroha-zip processes untrusted archives, so changes at trust boundaries require more evidence than ordinary application changes.

## Development checks

Use the pinned toolchain from `rust-toolchain.toml` and keep `Cargo.lock` committed.

```text
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --locked
cargo check --all-targets --target x86_64-pc-windows-msvc --locked
```

Changes to `src/platform/windows_impl.rs`, archive transfer/policy code, backend installation, release packaging, or file associations must explain the affected trust boundary and add a regression test where practical.

## Pull requests

- Keep each change reviewable and describe its security impact.
- Do not add an unsandboxed fallback that can activate without an explicit per-command opt-in.
- Do not commit backend executables, DLLs, generated manifests, secrets, malicious samples, or build output.
- Preserve the no-overwrite and fail-closed defaults.
- Update `docs/THREAT_MODEL.md`, `docs/ISSUE_BACKLOG.md`, and `CHANGELOG.md` when behavior or residual risk changes.

Report exploitable vulnerabilities privately as described in `SECURITY.md`; do not attach weaponized archives to public issues.
