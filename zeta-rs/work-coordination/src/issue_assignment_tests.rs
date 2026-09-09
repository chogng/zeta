use super::*;

#[test]
fn issue_workflow_rejects_ambiguous_labels_and_unsafe_branch_templates() {
    let mut workflow = IssueWorkflow::default();
    assert!(workflow.validate().is_ok());
    workflow.labels.review = workflow.labels.queued.clone();
    assert!(workflow.validate().is_err());
    workflow.labels = IssueLabels::default();
    for template in [
        "../{number}-{attempt}",
        "refs/{number}/../{attempt}",
        "{number}-{unknown}-{attempt}",
        "{number}",
    ] {
        workflow.branch_template = template.into();
        assert!(workflow.validate().is_err(), "{template}");
    }
}

#[test]
fn issue_branch_names_are_stable_bounded_and_isolate_attempts() {
    let workflow = IssueWorkflow::default();
    let issue = IssueIdentity {
        node_id: "issue18".into(),
        number: 18,
        title: "Search / Cache 修复".into(),
        updated_at: "now".into(),
        material_digest: "digest".into(),
    };
    assert_eq!(
        workflow.branch_name(&issue, "a1").unwrap(),
        "codex/issue-18-search-cache-a1"
    );
    assert_ne!(
        workflow.branch_name(&issue, "a1").unwrap(),
        workflow.branch_name(&issue, "a2").unwrap()
    );
    assert!(workflow.branch_name(&issue, "../main").is_err());
}

#[test]
fn issue_auto_claim_is_bounded_and_cannot_take_another_owners_work() {
    let mut workflow = IssueWorkflow::default();
    workflow.assignee = "me".into();
    workflow.auto_claim = Some(IssueAutoClaim {
        labels: vec!["ready".into()],
        assignee: None,
        max_issues: 3,
    });
    assert!(workflow.validate().is_ok());
    workflow.auto_claim.as_mut().unwrap().assignee = Some("someone-else".into());
    assert!(workflow.validate().is_err());
    workflow.auto_claim.as_mut().unwrap().assignee = None;
    workflow.auto_claim.as_mut().unwrap().max_issues = 0;
    assert!(workflow.validate().is_err());
}

#[test]
fn issue_scope_conflicts_include_shared_apis_and_unknown_ranges() {
    let mut one = IssueWorkItem {
        id: "one".into(),
        issues: Vec::new(),
        objective: "one".into(),
        acceptance_conditions: Vec::new(),
        scope: Default::default(),
        dependencies: Default::default(),
        agent: String::new(),
    };
    let mut two = one.clone();
    assert!(one.conflicts_with(&two));
    one.scope.paths.insert("src/list/**".into());
    two.scope.paths.insert("src/config/**".into());
    assert!(!one.conflicts_with(&two));
    one.scope.contracts.insert("Issue protocol".into());
    two.scope.contracts.insert("Issue protocol".into());
    assert!(one.conflicts_with(&two));
    two.scope.contracts.clear();
    two.scope.paths.insert("src/list/api.rs".into());
    assert!(one.conflicts_with(&two));
}
