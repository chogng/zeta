use crate::SqliteIssueTaskStore;
use zeta_github::Issue;
use zeta_github::IssueSnapshot;
use zeta_github::IssueTask;
use zeta_github::Repository;

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
