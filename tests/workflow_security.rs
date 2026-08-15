use std::fs;
use std::path::Path;

#[test]
fn workflows_pin_actions_bound_jobs_and_drop_checkout_credentials() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflows = ["ci.yml", "fuzz.yml", "pages.yml", "release.yml"];
    let workflow_directory = repository.join(".github/workflows");
    let mut discovered_workflows = fs::read_dir(&workflow_directory)
        .expect("workflow directory must be readable")
        .map(|entry| entry.expect("workflow entry must be readable"))
        .map(|entry| {
            let file_type = entry.file_type().unwrap_or_else(|error| {
                panic!("cannot inspect {}: {error}", entry.path().display())
            });
            assert!(
                file_type.is_file() && !file_type.is_symlink(),
                "workflow must be a regular non-link file: {}",
                entry.path().display()
            );
            entry
                .file_name()
                .into_string()
                .expect("workflow name must be UTF-8")
        })
        .collect::<Vec<_>>();
    discovered_workflows.sort();
    assert_eq!(
        discovered_workflows,
        workflows.map(str::to_owned),
        "every workflow file must be reviewed by this contract"
    );

    let mut checkout_count = 0;
    let mut external_action_count = 0;
    let mut combined = String::new();

    for workflow in workflows {
        let path = repository.join(".github/workflows").join(workflow);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let lines = source.lines().collect::<Vec<_>>();
        let runner_count = source.matches("\n    runs-on:").count();
        let timeout_count = source.matches("\n    timeout-minutes:").count();
        assert!(runner_count > 0, "{workflow} must define at least one job");
        assert_eq!(
            timeout_count, runner_count,
            "every job in {workflow} must have an explicit timeout"
        );

        assert!(!source.contains("pull_request_target:"));
        assert!(!source.contains("workflow_run:"));
        assert!(!source.contains("secrets: inherit"));
        assert!(!source.contains("permissions: write-all"));

        let mut workflow_checkout_count = 0;
        for (index, line) in lines.iter().enumerate() {
            let Some(action) = line.trim().strip_prefix("uses:") else {
                continue;
            };
            let action = action
                .split_ascii_whitespace()
                .next()
                .expect("uses value must not be empty");
            if action.starts_with("./") {
                continue;
            }

            external_action_count += 1;
            let (_, revision) = action
                .rsplit_once('@')
                .unwrap_or_else(|| panic!("external action is not pinned in {workflow}: {action}"));
            assert_eq!(
                revision.len(),
                40,
                "external action is not pinned to a full commit in {workflow}: {action}"
            );
            assert!(
                revision
                    .chars()
                    .all(|character| character.is_ascii_hexdigit()),
                "external action has a non-hex revision in {workflow}: {action}"
            );

            if action.starts_with("actions/checkout@") {
                checkout_count += 1;
                workflow_checkout_count += 1;
                assert!(
                    lines
                        .iter()
                        .skip(index + 1)
                        .take(5)
                        .any(|following| following.trim() == "persist-credentials: false"),
                    "checkout must not persist its token in {workflow}"
                );
            }
        }

        assert_eq!(
            source.matches("persist-credentials: false").count(),
            workflow_checkout_count,
            "credential persistence settings must map exactly to checkouts in {workflow}"
        );
        combined.push_str(&source);
    }

    assert_eq!(external_action_count, 28);
    assert_eq!(checkout_count, 10);
    assert_eq!(combined.matches("contents: write").count(), 1);
    assert_eq!(combined.matches("pages: write").count(), 1);
    assert_eq!(combined.matches("id-token: write").count(), 3);
    assert_eq!(combined.matches("attestations: write").count(), 2);
    assert!(!combined.contains("actions: write"));
}
