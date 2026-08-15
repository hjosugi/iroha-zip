## What changed / 変更内容

Describe the behavior and why it is needed.

変更した動作と、その変更が必要な理由を書いてください。

## Security impact / セキュリティへの影響

Identify affected trust boundaries, inputs, permissions, processes, or fail-closed behavior. Write
“none” only after checking `docs/THREAT_MODEL.md`.

影響する信頼境界、入力、権限、プロセス、fail-closed動作を書いてください。
`docs/THREAT_MODEL.md`を確認したうえで、影響がない場合だけ「なし」と書いてください。

## Validation / 検証

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --all-targets --locked`
- [ ] `cargo test --features fuzzing --test fuzz_regressions --locked`
- [ ] `cargo clippy --all-targets --locked`
- [ ] Windows checks appropriate to the change / 変更に応じたWindows検証
- [ ] Documentation/changelog updated for behavior or residual-risk changes / 動作・残存リスクに応じた文書とchangelog更新
