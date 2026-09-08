use super::*;

#[test]
fn repository_identity_rejects_paths_and_option_injection() {
    for value in ["", "../repo", "-repo", "a/b", "repo?x=y", "a\nb", "a b"] {
        assert!(
            Repository::new("github.com".into(), "owner".into(), value.into()).is_err(),
            "{value:?}"
        );
    }
    assert_eq!(
        Repository::new("github.example.com".into(), "team".into(), "my-repo".into())
            .unwrap()
            .endpoint("issues"),
        "repos/team/my-repo/issues"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn draft_pr_cannot_enable_auto_merge() {
    let repository = Repository::new("github.com".into(), "team".into(), "repo".into()).unwrap();
    let pr = PullRequest {
        number: 7,
        node_id: "id".into(),
        html_url: "https://github.com/team/repo/pull/7".into(),
        state: "open".into(),
        draft: true,
        merged_at: None,
        auto_merge: None,
        head: PullRequestBranch {
            name: "feature".into(),
            sha: "abc".into(),
        },
        base: PullRequestBranch {
            name: "main".into(),
            sha: "def".into(),
        },
    };
    let github = GitHub {
        executable: "must-not-execute".into(),
    };
    assert_eq!(
        github
            .enable_auto_merge(&repository, &pr, MergeMethod::Squash)
            .await,
        Err("Automatic merge requires an open, non-draft PR".into())
    );
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_issue_page_does_not_start_a_process() {
    let repository = Repository::new("github.com".into(), "team".into(), "repo".into()).unwrap();
    let github = GitHub {
        executable: "must-not-execute".into(),
    };
    assert!(
        github
            .issues(&repository, IssueState::Open, 0)
            .await
            .unwrap_err()
            .contains("page")
    );
    assert!(
        github
            .issue(&repository, 0)
            .await
            .unwrap_err()
            .contains("positive")
    );
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn issue_reader_excludes_prs_and_reads_every_comment_page() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let issue = serde_json::json!({"number":3,"title":"Fix it","body":"details","html_url":"https://github.com/team/repo/issues/3","updated_at":"now","state":"open"});
    let mut closed = issue.clone();
    closed["number"] = 9.into();
    closed["state"] = "closed".into();
    let mut pr = issue.clone();
    pr["number"] = 4.into();
    pr["pull_request"] = serde_json::json!({"url":"pr"});
    let comment = serde_json::json!({"body":"comment","html_url":"comment-url","updated_at":"now"});
    for (name, value) in [
        ("list", serde_json::json!([issue.clone(), pr])),
        ("closed", serde_json::json!([closed])),
        ("issue", issue),
        ("comments", serde_json::json!(vec![comment; 100])),
        ("empty", serde_json::json!([])),
    ] {
        std::fs::write(
            directory.path().join(name),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
    }
    let script = directory.path().join("gh");
    std::fs::write(&script, format!("#!/bin/sh\ncase \"$6\" in\n*'/comments?'*'page=1') cat '{0}/comments';;\n*'/comments?'*) cat '{0}/empty';;\n*'/issues/3') cat '{0}/issue';;\n*'/issues?state=open&'*'page=1') cat '{0}/list';;\n*'/issues?state=closed&'*'page=2') cat '{0}/closed';;\n*) exit 9;;\nesac\n", directory.path().display())).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let github = GitHub { executable: script };
    let repository = Repository::new("github.com".into(), "team".into(), "repo".into()).unwrap();
    let page = github
        .issues(&repository, IssueState::Open, 1)
        .await
        .unwrap();
    assert_eq!(
        page.issues
            .iter()
            .map(|issue| issue.number)
            .collect::<Vec<_>>(),
        [3]
    );
    assert_eq!(page.next_page, None);
    let closed = github
        .issues(&repository, IssueState::Closed, 2)
        .await
        .unwrap();
    assert_eq!(closed.issues.len(), 1);
    assert_eq!(
        (closed.issues[0].number, closed.issues[0].state.as_str()),
        (9, "closed")
    );
    let snapshot = github.issue(&repository, 3).await.unwrap();
    assert_eq!(snapshot.comments.len(), 100);
    assert_eq!(snapshot.issue.body.as_deref(), Some("details"));
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn graphql_errors_are_failures_even_when_the_process_succeeds() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("gh");
    std::fs::write(&script, "#!/bin/sh\ncat >/dev/null\nprintf '%s' '{\"errors\":[{\"message\":\"Auto-merge disabled\"}]}'\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let github = GitHub { executable: script };
    let repository = Repository::new("github.com".into(), "team".into(), "repo".into()).unwrap();
    let error = github
        .api::<serde_json::Value>(
            &repository,
            "POST",
            "graphql",
            Some(serde_json::json!({"query":"test"})),
        )
        .await
        .unwrap_err();
    assert!(error.contains("Auto-merge disabled"));
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn automatic_merge_uses_the_selected_method_and_exact_reviewed_head() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("gh");
    let args_path = directory.path().join("args");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n",
            args_path.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let github = GitHub { executable: script };
    let repository = Repository::new("github.com".into(), "team".into(), "repo".into()).unwrap();
    let pr = PullRequest {
        number: 7,
        node_id: "pr".into(),
        html_url: "https://github.com/team/repo/pull/7".into(),
        state: "open".into(),
        draft: false,
        merged_at: None,
        auto_merge: None,
        head: PullRequestBranch {
            name: "issue/task".into(),
            sha: "a".repeat(40),
        },
        base: PullRequestBranch {
            name: "main".into(),
            sha: "b".repeat(40),
        },
    };
    for (method, flag) in [
        (MergeMethod::Merge, "--merge"),
        (MergeMethod::Squash, "--squash"),
        (MergeMethod::Rebase, "--rebase"),
    ] {
        github
            .enable_auto_merge(&repository, &pr, method)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&args_path)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            vec![
                "pr",
                "merge",
                "7",
                "--repo",
                "github.com/team/repo",
                "--auto",
                flag,
                "--match-head-commit",
                pr.head.sha.as_str()
            ]
        );
    }
}
