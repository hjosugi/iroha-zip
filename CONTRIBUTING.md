# Contributing to iroha-zip / iroha-zipへの貢献

[English](#english) | [日本語](#日本語)

## English

iroha-zip processes untrusted archives, so changes at trust boundaries require more evidence than ordinary application changes.

### Development checks

Use the pinned toolchain from `rust-toolchain.toml` and keep `Cargo.lock` committed.

```text
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --locked
cargo check --all-targets --target x86_64-pc-windows-msvc --locked
cargo test --locked --features fuzzing --test fuzz_regressions
```

Changes to `src/platform/windows_impl.rs`, archive transfer/policy code, backend installation, release packaging, or file associations must explain the affected trust boundary and add a regression test where practical.

Changes to backend manifests, path policy, archive destination naming, Windows process arguments, or configuration parsing must also update the relevant target in [docs/FUZZING.md](docs/FUZZING.md). Never commit a raw crash artifact: minimize it and promote it with `scripts/promote-fuzz-regression.ps1` so ordinary CI reproduces it deterministically.

### Pull requests

- Keep each change reviewable and describe its security impact.
- Do not add an unsandboxed fallback that can activate without an explicit per-command opt-in.
- Do not commit backend executables, DLLs, generated manifests, secrets, malicious samples, or build output.
- Preserve the no-overwrite and fail-closed defaults.
- Update `docs/THREAT_MODEL.md`, `docs/ISSUE_BACKLOG.md`, and `CHANGELOG.md` when behavior or residual risk changes.

Report exploitable vulnerabilities privately as described in [SECURITY.md](SECURITY.md); do not attach weaponized archives to public issues.

## 日本語

iroha-zip は未信頼の書庫を処理するため、信頼境界の変更には通常のアプリケーション変更より強い証拠が必要です。

### 開発時の確認

`rust-toolchain.toml` で固定した toolchain を使い、`Cargo.lock` を必ず commit してください。実行するコマンドは上の English セクションと同一です。

`src/platform/windows_impl.rs`、書庫の転送・policy、backend 取り込み、Release package、関連付けを変更する場合は、影響する信頼境界を説明し、可能な限り回帰試験を追加してください。

backend manifest、Windows path、出力名、Windows process argument、設定 parser を変更する場合は [fuzzing 手順](docs/FUZZING.md) の対象も更新してください。生の crash artifact は commit せず、最小化して `scripts/promote-fuzz-regression.ps1` で通常 CI が再現できる回帰入力へ変換します。

### Pull request

- 変更を review 可能な大きさに保ち、セキュリティへの影響を説明する。
- command ごとの明示 opt-in なしに有効になる unsandboxed fallback を追加しない。
- backend の EXE/DLL、生成 manifest、秘密情報、悪性 sample、build output を commit しない。
- 既存出力を上書きしない既定値と fail-closed 動作を維持する。
- 動作や残存リスクを変えた場合は `docs/THREAT_MODEL.md`、`docs/ISSUE_BACKLOG.md`、`CHANGELOG.md` を更新する。

悪用可能な脆弱性は [SECURITY.md](SECURITY.md#日本語) に従って非公開で報告し、weaponized archive を公開 Issue へ添付しないでください。
