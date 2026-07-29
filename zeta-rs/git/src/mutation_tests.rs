use super::{GitCommitRequest, GitPathspecSet};
use crate::test_support::{TestBareRepository, TestRepository};
use crate::{GitChangeStatus, GitClient, GitError};
use std::path::PathBuf;

#[test]
fn pathspec_and_commit_requests_reject_ambiguous_inputs() {
    assert!(GitPathspecSet::new(Vec::new()).is_err());
    assert!(GitPathspecSet::new(vec![PathBuf::from("../outside")]).is_err());
    assert!(GitPathspecSet::new(vec![PathBuf::from("/absolute")]).is_err());
    assert!(GitPathspecSet::new(vec![PathBuf::from("path\0suffix")]).is_err());
    assert!(GitCommitRequest::new("   ".into()).is_err());
    assert!(GitCommitRequest::new("message\0suffix".into()).is_err());
}

#[tokio::test]
async fn stages_unstages_discards_and_commits_selected_paths() {
    let repository = TestRepository::init();
    repository.write("tracked.txt", "initial\n");
    repository.commit_all("initial");
    repository.write("tracked.txt", "changed\n");
    repository.write("new.txt", "new\n");
    let client = GitClient::system();
    let opened = client.open_repository(repository.root()).await.unwrap();
    let tracked = GitPathspecSet::new(vec![PathBuf::from("tracked.txt")]).unwrap();
    let new_file = GitPathspecSet::new(vec![PathBuf::from("new.txt")]).unwrap();

    client.stage(&opened, &tracked).await.unwrap();
    let staged = client.snapshot(&opened).await.unwrap();
    assert_eq!(
        staged.changes()[0].index_status(),
        GitChangeStatus::Modified
    );

    client.unstage(&opened, &tracked).await.unwrap();
    let unstaged = client.snapshot(&opened).await.unwrap();
    assert_eq!(
        unstaged.changes()[0].index_status(),
        GitChangeStatus::Unmodified
    );
    assert_eq!(
        unstaged.changes()[0].worktree_status(),
        GitChangeStatus::Modified
    );

    client.discard_worktree(&opened, &tracked).await.unwrap();
    assert_eq!(
        repository.read("tracked.txt").replace("\r\n", "\n"),
        "initial\n"
    );
    client.stage(&opened, &new_file).await.unwrap();
    let commit = client
        .commit(
            &opened,
            &GitCommitRequest::new("add new file".into()).unwrap(),
        )
        .await
        .unwrap();
    assert!(!commit.object_id().is_empty());
    assert!(client.snapshot(&opened).await.unwrap().is_clean());
}

#[tokio::test]
async fn fetches_fast_forward_pulls_and_pushes_against_a_local_remote() {
    let origin = TestBareRepository::init();
    let first = TestRepository::clone_from(origin.root());
    first.write("shared.txt", "initial\n");
    first.commit_all("initial");
    first.git(&["push", "--set-upstream", "origin", "main"]);
    let second = TestRepository::clone_from(origin.root());
    let client = GitClient::system();
    let first_repository = client.open_repository(first.root()).await.unwrap();
    let second_repository = client.open_repository(second.root()).await.unwrap();

    first.write("shared.txt", "from first\n");
    first.commit_all("update from first");
    client.push(&first_repository).await.unwrap();
    let first_head = first.git(&["rev-parse", "HEAD"]);

    client.fetch(&second_repository).await.unwrap();
    assert_eq!(
        second.git(&["rev-parse", "refs/remotes/origin/main"]),
        first_head
    );
    client.pull_fast_forward(&second_repository).await.unwrap();
    assert_eq!(second.read("shared.txt"), "from first\n");
    assert_eq!(second.git(&["rev-parse", "HEAD"]), first_head);

    second.write("second.txt", "from second\n");
    second.commit_all("update from second");
    client.push(&second_repository).await.unwrap();
    let second_head = second.git(&["rev-parse", "HEAD"]);
    assert_eq!(origin.git(&["rev-parse", "refs/heads/main"]), second_head);

    client.pull_fast_forward(&first_repository).await.unwrap();
    first.write("first-only.txt", "upstream\n");
    first.commit_all("upstream divergence");
    client.push(&first_repository).await.unwrap();
    second.write("second-only.txt", "local\n");
    second.commit_all("local divergence");
    let local_head = second.git(&["rev-parse", "HEAD"]);

    let error = client
        .pull_fast_forward(&second_repository)
        .await
        .expect_err("a non-fast-forward pull must fail");
    assert!(matches!(error, GitError::CommandFailed { .. }));
    assert_eq!(second.git(&["rev-parse", "HEAD"]), local_head);
}
