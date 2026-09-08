use super::IssueRepository;
use super::IssueStartPoint;
use super::IssueTaskCreateParams;
use crate::local::LocalAppServerOptions;
use crate::local::open_local_app_server;
use sha2::Digest;
use sha2::Sha256;
use std::path::Path;
use std::process::Command;
use zeta_protocol::CommandId;
use zeta_protocol::ThreadId;

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

#[test]
fn prepared_issue_task_recovers_exact_code_and_scoped_context_without_duplicate_sessions() {
    verify_recovery(Path::new("."));
    verify_recovery(Path::new("subdir"));
}

fn verify_recovery(relative: &Path) {
    let profile = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(directory.path()).unwrap();
    let source = root.join(relative);
    std::fs::create_dir_all(&source).unwrap();
    let source = dunce::canonicalize(source).unwrap();
    git(&root, &["init", "--initial-branch=main"]);
    git(&root, &["config", "user.name", "Issue Test"]);
    git(&root, &["config", "user.email", "issue@example.invalid"]);
    std::fs::write(source.join("file"), "base").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "base"]);
    let commit = git(&root, &["rev-parse", "HEAD"]);
    let tree = git(&root, &["rev-parse", "HEAD^{tree}"]);
    std::fs::write(source.join("file"), "keep dirty work").unwrap();
    let params = IssueTaskCreateParams {
        command_id: CommandId::new("issue-recovery").unwrap(),
        repository: IssueRepository {
            host: "github.com".into(),
            owner: "team".into(),
            name: "repo".into(),
        },
        numbers: vec![3, 5],
        start: IssueStartPoint::CurrentBranch,
    };
    let task = zeta_github::IssueTask {
        command_id: params.command_id.to_string(),
        fingerprint: format!("{:x}", Sha256::digest(serde_json::to_vec(&params).unwrap())),
        session_id: "thread:issue-recovery".into(),
        repository: zeta_github::Repository::new("github.com".into(), "team".into(), "repo".into())
            .unwrap(),
        issues: params
            .numbers
            .iter()
            .map(|&number| zeta_github::IssueSnapshot {
                issue: zeta_github::Issue {
                    number,
                    title: format!("Fix {number}"),
                    body: Some("exact saved requirement".into()),
                    html_url: format!("https://github.com/team/repo/issues/{number}"),
                    updated_at: "then".into(),
                    state: "open".into(),
                    pull_request: None,
                },
                comments: vec![],
            })
            .collect(),
        source_root: source.clone(),
        start_commit: commit,
        start_tree: tree,
        branch: "issue/recovery".into(),
        target_branch: "main".into(),
        read_at: 7,
        pull_request: None,
    };
    let open = || {
        open_local_app_server(
            LocalAppServerOptions::new(profile.path())
                .without_built_in_skills()
                .with_dir_root(&source),
        )
        .unwrap()
    };
    let server = open();
    server.issue_tasks.as_ref().unwrap().prepare(&task).unwrap();
    let mut connection = server.connection();
    let value = serde_json::to_value(&params).unwrap();
    let first = server
        .issue_task_create(&mut connection, &value)
        .map_err(|error| error.message)
        .unwrap();
    let second = server
        .issue_task_create(&mut connection, &value)
        .map_err(|error| error.message)
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(first["task"]["issues"].as_array().unwrap().len(), 2);
    let thread_id = ThreadId::new(task.session_id.clone()).unwrap();
    let binding = server
        .turn_changes
        .as_ref()
        .unwrap()
        .binding(&thread_id)
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(binding.dir().join("file")).unwrap(),
        "base"
    );
    assert_eq!(
        std::fs::read_to_string(source.join("file")).unwrap(),
        "keep dirty work"
    );
    assert_eq!(
        server
            .session_views()
            .map_err(|error| error.message)
            .unwrap()
            .len(),
        1
    );
    let session_id = zeta_protocol::SessionId::new(task.session_id.clone()).unwrap();
    let context = server
        .issue_input(&session_id, 3)
        .map_err(|error| error.message)
        .unwrap();
    assert!(
        matches!(context, zeta_protocol::UserInput::Context { name, content } if name == "issue #3" && content.contains("exact saved requirement"))
    );
    assert!(server.issue_input(&session_id, 99).is_err());
    server.close_connection(connection);
    drop(server);
    let server = open();
    let read = server
        .issue_task_read(&serde_json::json!({"sessionId": session_id}))
        .map_err(|error| error.message)
        .unwrap();
    assert_eq!(read, first);
    let recovered = server
        .turn_changes
        .as_ref()
        .unwrap()
        .binding(&thread_id)
        .unwrap();
    assert_eq!(recovered.dir(), binding.dir());
}
