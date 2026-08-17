use std::num::NonZeroUsize;

use pretty_assertions::assert_eq;

use super::GitBranch;
use super::GitCommitSummary;
use super::GitRemote;
use super::parse_commits;
use crate::GitClient;
use crate::test_support::TestRepository;

#[tokio::test(flavor = "current_thread")]
async fn lists_local_branches_with_current_marker() {
    let repository = TestRepository::init();
    repository.write("tracked.txt", "tracked\n");
    repository.commit_all("initial");
    let main_oid = repository.git(&["rev-parse", "HEAD"]);
    repository.git(&["branch", "topic"]);

    let client = GitClient::system();
    let opened = client
        .open_repository(repository.root())
        .await
        .expect("open repository");
    let branches = client.local_branches(&opened).await.expect("list branches");

    assert_eq!(
        branches,
        vec![
            GitBranch {
                name: "main".to_string(),
                object_id: main_oid.clone(),
                current: true,
                upstream: None,
            },
            GitBranch {
                name: "topic".to_string(),
                object_id: main_oid,
                current: false,
                upstream: None,
            },
        ]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn lists_remote_fetch_and_push_urls() {
    let repository = TestRepository::init();
    repository.git(&[
        "remote",
        "add",
        "origin",
        "https://example.invalid/repo.git",
    ]);
    repository.git(&[
        "remote",
        "set-url",
        "--push",
        "origin",
        "ssh://git@example.invalid/repo.git",
    ]);

    let client = GitClient::system();
    let opened = client
        .open_repository(repository.root())
        .await
        .expect("open repository");
    let remotes = client.remotes(&opened).await.expect("list remotes");

    assert_eq!(
        remotes,
        vec![GitRemote {
            name: "origin".to_string(),
            fetch_urls: vec!["https://example.invalid/repo.git".to_string()],
            push_urls: vec!["ssh://git@example.invalid/repo.git".to_string()],
        }]
    );
}

#[test]
fn projects_github_identity_without_credentials() {
    let remote = GitRemote {
        name: "origin".to_string(),
        fetch_urls: vec!["https://token@example.com/ignored/repo.git".to_string()],
        push_urls: vec!["git@github.com:chogng/zeta.git".to_string()],
    };

    let identity = remote.identity().expect("remote identity");
    assert_eq!(identity.host(), "example.com");
    assert_eq!(identity.owner(), "ignored");
    assert_eq!(identity.repository(), "repo");
    assert_eq!(identity.provider(), super::GitRemoteProvider::Other);

    let github = GitRemote {
        name: "origin".to_string(),
        fetch_urls: vec!["ssh://git@github.com:22/chogng/zeta.git?transport=ssh".to_string()],
        push_urls: Vec::new(),
    };
    let identity = github.identity().expect("GitHub identity");
    assert_eq!(identity.host(), "github.com");
    assert_eq!(identity.owner(), "chogng");
    assert_eq!(identity.repository(), "zeta");
    assert_eq!(identity.provider(), super::GitRemoteProvider::Github);
}

#[tokio::test(flavor = "current_thread")]
async fn returns_bounded_recent_commit_summaries() {
    let repository = TestRepository::init();
    repository.write("tracked.txt", "one\n");
    repository.commit_all("first");
    repository.write("tracked.txt", "two\n");
    repository.commit_all("second");
    let latest_oid = repository.git(&["rev-parse", "HEAD"]);
    let latest_timestamp = repository
        .git(&["show", "-s", "--format=%ct", "HEAD"])
        .parse()
        .expect("timestamp");

    let client = GitClient::system();
    let opened = client
        .open_repository(repository.root())
        .await
        .expect("open repository");
    let commits = client
        .recent_commits(&opened, NonZeroUsize::new(1).expect("non-zero"))
        .await
        .expect("recent commits");

    assert_eq!(
        commits,
        vec![GitCommitSummary {
            object_id: latest_oid,
            parent_object_ids: vec![repository.git(&["rev-parse", "HEAD^"])],
            timestamp_seconds: latest_timestamp,
            subject: "second".to_string(),
        }]
    );
}

#[test]
fn commit_parser_preserves_an_empty_subject_field() {
    let commits = parse_commits(b"abc\0parent-one parent-two\0\x31\x32\x33\0\0", "git log")
        .expect("parse commits");

    assert_eq!(
        commits,
        vec![GitCommitSummary {
            object_id: "abc".to_string(),
            parent_object_ids: vec!["parent-one".to_string(), "parent-two".to_string()],
            timestamp_seconds: 123,
            subject: String::new(),
        }]
    );
}
