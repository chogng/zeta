use crate::SqliteIssueTaskStore;
use zeta_github::Issue;
use zeta_github::IssueSnapshot;
use zeta_github::IssueTask;
use zeta_github::PullRequest;
use zeta_github::PullRequestBranch;
use zeta_github::Repository;

#[test]
fn record_pull_request_reports_missing_issue_task() {
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteIssueTaskStore::open(&directory.path().join("state.sqlite")).unwrap();
    let pull_request = PullRequest {
        number: 7,
        node_id: "pr".into(),
        html_url: "https://github.com/team/repo/pull/7".into(),
        state: "open".into(),
        draft: false,
        merged_at: None,
        head: PullRequestBranch {
            name: "issue/3-task".into(),
            sha: "a".repeat(40),
        },
        base: PullRequestBranch {
            name: "main".into(),
            sha: "b".repeat(40),
        },
        auto_merge: None,
    };

    assert_eq!(
        store.record_pull_request("missing", &pull_request),
        Err("This Session has no associated issues".into())
    );
}

#[test]
fn combined_issue_preparation_survives_reopen_and_rejects_changed_retries() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("state.sqlite");
    let task = IssueTask {
        command_id: "command".into(),
        fingerprint: "fingerprint".into(),
        session_id: "session".into(),
        repository: Repository::new("github.com".into(), "team".into(), "repo".into()).unwrap(),
        issues: [3, 5]
            .into_iter()
            .map(|number| IssueSnapshot {
                issue: Issue {
                    labels: Vec::new(),
                    assignees: Vec::new(),
                    number,
                    title: format!("Issue {number}"),
                    body: Some("requirements".into()),
                    html_url: format!("https://github.com/team/repo/issues/{number}"),
                    updated_at: "now".into(),
                    state: "open".into(),
                    pull_request: None,
                },
                comments: vec![],
            })
            .collect(),
        source_root: directory.path().into(),
        start_commit: "a".repeat(40),
        start_tree: "b".repeat(40),
        branch: "issue/3-task".into(),
        target_branch: "main".into(),
        read_at: 1,
        pull_request: None,
    };
    let store = SqliteIssueTaskStore::open(&database).unwrap();
    store.prepare(&task).unwrap();
    store.prepare(&task).unwrap();
    let mut changed = task.clone();
    changed.issues.remove(0);
    assert!(store.prepare(&changed).is_err());
    drop(store);
    let store = SqliteIssueTaskStore::open(&database).unwrap();
    assert_eq!(store.read_command("command").unwrap(), Some(task.clone()));
    assert_eq!(store.read_session("session").unwrap(), Some(task));
    assert_eq!(store.read_session("unrelated").unwrap(), None);
}
