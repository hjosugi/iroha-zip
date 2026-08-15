#[test]
fn contribution_templates_are_bilingual_current_and_security_aware() {
    let version = env!("CARGO_PKG_VERSION");
    let bug = include_str!("../.github/ISSUE_TEMPLATE/bug.yml");
    let feature = include_str!("../.github/ISSUE_TEMPLATE/feature.yml");
    let config = include_str!("../.github/ISSUE_TEMPLATE/config.yml");
    let pull_request = include_str!("../.github/pull_request_template.md");
    let private_advisory = "https://github.com/hjosugi/iroha-zip/security/advisories/new";
    let security_policy = "https://github.com/hjosugi/iroha-zip/blob/main/SECURITY.md";

    assert!(bug.contains(&format!("placeholder: v{version}")));
    assert!(!bug.contains("placeholder: v0.6.0"));
    assert!(bug.contains("labels: [\"bug\", \"needs-triage\"]"));
    assert!(feature.contains("labels: [\"enhancement\", \"needs-triage\"]"));
    for form in [bug, feature] {
        assert_eq!(form.matches(private_advisory).count(), 2);
        assert_eq!(form.matches(security_policy).count(), 2);
        assert!(form.contains("GitHub Security Advisory"));
        assert!(form.contains("required: true"));
    }

    assert!(config.contains("blank_issues_enabled: false"));
    assert_eq!(config.matches(private_advisory).count(), 1);
    assert!(config.contains("Private vulnerability report / 脆弱性の非公開報告"));
    for heading in [
        "## What changed / 変更内容",
        "## Security impact / セキュリティへの影響",
        "## Validation / 検証",
    ] {
        assert!(pull_request.contains(heading));
    }
    for command in [
        "`cargo fmt --all -- --check`",
        "`cargo test --all-targets --locked`",
        "`cargo test --features fuzzing --test fuzz_regressions --locked`",
        "`cargo clippy --all-targets --locked`",
    ] {
        assert!(pull_request.contains(command));
    }
}
