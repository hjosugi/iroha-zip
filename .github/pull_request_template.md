## What changed

Describe the behavior and why it is needed.

## Security impact

Identify affected trust boundaries, inputs, permissions, processes, or fail-closed behavior. Write “none” only after checking `docs/THREAT_MODEL.md`.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --all-targets --locked`
- [ ] `cargo clippy --all-targets --locked`
- [ ] Windows target or real-Windows checks appropriate to the change
- [ ] Documentation/changelog updated where behavior or residual risk changed
