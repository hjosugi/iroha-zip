# CodeQL baseline and triage

## 日本語

GitHub CodeQL default setupは、standard GitHub-hosted runnerでRust、GitHub Actions、
JavaScript／TypeScriptを解析します。query suiteは`extended`、threat modelは
`remote_and_local`で、push／pull requestと週次scheduleが対象です。初回の
[setup run 31788227953](https://github.com/hjosugi/iroha-zip/actions/runs/31788227953)は
3言語すべてに成功しました。ActionsとJavaScript／TypeScriptはalert 0でした。

Rust baselineは`rust/path-injection`だけを233件報告しました。このdesktop CLIは、
利用者が明示したarchive、source、destination、設定pathを同じ利用者権限で処理し、さらに
攻撃者が選べるarchive member名を検査するため、local threat modelは意図的なfilesystem境界も
taintとして追跡します。全233 sinkと報告されたdataflow sourceを個別に確認し、次のように
分類しました。

- 188件は`tests/**`、`#[cfg(test)]`、またはtest-only sourceから到達するsinkで、
  `used in tests`です。
- 45件はproduction codeですが、次の検証済み境界のいずれかで、報告された「未検証path」には
  該当しないため`false positive`です。
  - 利用者が同一権限で明示したinput／output／configuration path
  - `validate_manifest_path`または`policy::validate_relative_path`を通った相対path
  - 一意に作成し、identity／link／reparse／fingerprintを検査するsandbox／staging root
  - `create_new`で公開するdestinationと、同じtransactionが作成したexact cleanup target
  - 保持directory handleまたは検査済みrootから有界列挙したchild path

その後、Pages release behavior regressionの`vm.runInNewContext`に`js/code-injection` alert #234が
1件出ました。実行対象はconstant相対URLから同期読込するchecked-in `site/assets/site.js`だけで、
contextもtest内の合成DOM／fetch objectです。利用者入力やproduction sourceは到達しないため
`used in tests`としてdismissしました。2026-08-14時点のopen CodeQL alertは0件です。

dismissはruleを無効化しません。新しいdataflowやsinkは今後の解析で別alertになり、同じ基準で
再確認します。CodeQL合格やdismissは、Windows実機検証、fuzzing、manual review、独立security
auditの代替ではありません。

## English

GitHub CodeQL default setup analyzes Rust, GitHub Actions, and JavaScript/TypeScript on standard
GitHub-hosted runners. It uses the `extended` query suite and the `remote_and_local` threat model on
pushes, pull requests, and a weekly schedule. The initial
[setup run 31788227953](https://github.com/hjosugi/iroha-zip/actions/runs/31788227953) succeeded for all
three languages. Actions and JavaScript/TypeScript produced zero alerts.

The Rust baseline reported 233 instances of only `rust/path-injection`. This desktop CLI deliberately
processes archive, source, destination, and configuration paths selected by the user under that same
user's authority, while also inspecting attacker-selected archive member names. The local threat
model therefore tracks intentional filesystem boundaries as taint. Every reported sink and dataflow
source was reviewed and classified as follows:

- 188 instances are in `tests/**`, inside `#[cfg(test)]`, or reachable only from test sources, so they
  are `used in tests`.
- 45 instances are in production code, but are `false positive` because each flow reaches one of
  these reviewed boundaries rather than an uncontrolled path:
  - an input, output, or configuration path explicitly selected by the same-privilege user;
  - a relative path accepted by `validate_manifest_path` or `policy::validate_relative_path`;
  - a uniquely created sandbox/staging root with identity, link, reparse, and fingerprint checks;
  - a `create_new` publication destination or an exact cleanup target created by the same transaction;
  - a child path boundedly enumerated from a retained directory handle or inspected root.

The later Pages release-behavior regression produced one `js/code-injection` alert (#234) at
`vm.runInNewContext`. Its only executed subject is checked-in `site/assets/site.js`, synchronously
read from a constant relative URL, and its context contains only synthetic DOM/fetch objects. No
user or production source reaches it, so it was dismissed as `used in tests`. Open CodeQL alerts
were zero as of 2026-08-14.

Dismissal does not disable the rule. A new dataflow or sink creates a new alert for review against the
same criteria. A passing or dismissed CodeQL result does not replace physical-Windows validation,
fuzzing, manual review, or an independent security audit.
