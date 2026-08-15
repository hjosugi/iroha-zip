#[test]
fn closed_issue_documents_distinguish_acceptance_from_future_work() {
    let preview = include_str!("../docs/ARCHIVE_PREVIEW.md");
    assert!(preview.contains("Issue #12"));
    assert!(preview.contains("is closed because its preview and"));
    assert!(preview.contains("native graphical browser remains a separate future"));
    assert!(!preview.contains("UX-002 remains open"));

    let handoff = include_str!("../docs/ANTIMALWARE_HANDOFF.md");
    assert!(handoff.contains("Issue #10"));
    assert!(handoff.contains("is closed because SAFE-008's acceptance"));
    assert!(handoff.contains("future interoperability and product-claim work"));
    assert!(!handoff.contains("SAFE-008 remains open"));
    assert!(handoff.contains("must not be described as antivirus protection"));
}
