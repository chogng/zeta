use std::path::Path;

use super::GitChangeFileComparison;
use super::GitFileRevision;
use crate::GitClient;
use crate::test_support::TestRepository;

#[tokio::test]
async fn reads_head_and_index_content_and_reports_missing_paths() {
    let repository = TestRepository::init();
    repository.write("tracked.txt", "head\n");
    repository.git(&["add", "tracked.txt"]);
    repository.git(&["commit", "-m", "initial"]);
    repository.write("tracked.txt", "index\n");
    repository.git(&["add", "tracked.txt"]);

    let client = GitClient::system();
    let opened = client.open_repository(repository.root()).await.unwrap();

    assert_eq!(
        client
            .read_file_at_revision(
                &opened,
                Path::new("tracked.txt"),
                GitFileRevision::Head,
                1024,
            )
            .await
            .unwrap(),
        Some(b"head\n".to_vec())
    );
    assert_eq!(
        client
            .read_file_at_revision(
                &opened,
                Path::new("tracked.txt"),
                GitFileRevision::Index,
                1024,
            )
            .await
            .unwrap(),
        Some(b"index\n".to_vec())
    );
    assert_eq!(
        client
            .read_file_at_revision(
                &opened,
                Path::new("missing.txt"),
                GitFileRevision::Head,
                1024,
            )
            .await
            .unwrap(),
        None
    );
    let head = repository.git(&["rev-parse", "HEAD"]);
    let tree = client.resolve_tree(&opened, &head).await.unwrap();
    repository.write("tracked.txt", "later worktree\n");
    assert_eq!(
        client
            .read_file_at_tree(&opened, &tree, Path::new("tracked.txt"), 1024)
            .await
            .unwrap(),
        Some(b"head\n".to_vec())
    );
}

#[tokio::test]
async fn reads_staged_and_unstaged_change_sides_without_collapsing_the_index() {
    let repository = TestRepository::init();
    repository.write("tracked.txt", "head\n");
    repository.commit_all("initial");
    repository.write("tracked.txt", "index\n");
    repository.git(&["add", "tracked.txt"]);
    repository.write("tracked.txt", "worktree\n");

    let client = GitClient::system();
    let opened = client.open_repository(repository.root()).await.unwrap();
    let snapshot = client.snapshot(&opened).await.unwrap();
    let change = &snapshot.changes()[0];

    let staged = client
        .change_file(&opened, change, GitChangeFileComparison::Staged, 1024)
        .await
        .unwrap();
    assert_eq!(staged.original(), Some(b"head\n".as_slice()));
    assert_eq!(staged.modified(), Some(b"index\n".as_slice()));

    let unstaged = client
        .change_file(&opened, change, GitChangeFileComparison::Unstaged, 1024)
        .await
        .unwrap();
    assert_eq!(unstaged.original(), Some(b"index\n".as_slice()));
    assert_eq!(unstaged.modified(), Some(b"worktree\n".as_slice()));
}
