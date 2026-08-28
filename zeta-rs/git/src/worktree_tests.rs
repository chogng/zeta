use pretty_assertions::assert_eq;

use super::GitWorktreeAvailability;
use super::parse_worktrees;
use crate::GitClient;
use crate::GitError;
use crate::test_support::TestRepository;

#[tokio::test(flavor = "current_thread")]
async fn lists_primary_linked_locked_and_prunable_worktrees() {
    let repository = TestRepository::init();
    repository.write("tracked.txt", "tracked\n");
    repository.commit_all("initial");
    let linked = repository.path("linked");
    repository.git(&[
        "worktree",
        "add",
        "-b",
        "topic",
        linked.to_str().expect("UTF-8 linked path"),
        "HEAD",
    ]);
    repository.git(&[
        "worktree",
        "lock",
        "--reason",
        "in use",
        linked.to_str().expect("UTF-8 linked path"),
    ]);
    let stale = repository.path("stale");
    repository.git(&[
        "worktree",
        "add",
        "-b",
        "stale",
        stale.to_str().expect("UTF-8 stale path"),
        "HEAD",
    ]);
    std::fs::remove_dir_all(&stale).expect("remove stale checkout");

    let client = GitClient::system();
    let opened = client
        .open_repository(repository.root())
        .await
        .expect("open repository");
    let worktrees = client.worktrees(&opened).await.expect("list worktrees");

    assert_eq!(worktrees.len(), 3);
    assert_eq!(worktrees[0].checkout_root(), repository.root());
    assert_eq!(worktrees[0].branch(), Some("main"));
    assert_eq!(
        worktrees[1].availability(),
        &GitWorktreeAvailability::Locked {
            reason: Some("in use".to_string())
        }
    );
    assert!(matches!(
        worktrees[2].availability(),
        GitWorktreeAvailability::Prunable { .. }
    ));
}

#[test]
fn parser_accepts_a_detached_record_fixture() {
    let output = b"worktree /repo/topic\0HEAD 0123456789abcdef\0detached\0\0";

    let worktrees = parse_worktrees(output, "git worktree list").expect("parse fixture");

    assert_eq!(worktrees.len(), 1);
    assert_eq!(
        worktrees[0].checkout_root(),
        std::path::Path::new("/repo/topic")
    );
    assert_eq!(worktrees[0].head(), "0123456789abcdef");
    assert_eq!(worktrees[0].branch(), None);
}

#[test]
fn parser_rejects_truncated_unknown_and_misordered_records() {
    for output in [
        b"worktree /repo/topic\0HEAD 0123456789abcdef\0".as_slice(),
        b"worktree /repo/topic\0HEAD 0123456789abcdef\0future field\0\0".as_slice(),
        b"HEAD 0123456789abcdef\0worktree /repo/topic\0\0".as_slice(),
    ] {
        assert!(matches!(
            parse_worktrees(output, "git worktree list"),
            Err(GitError::InvalidOutput { .. })
        ));
    }
}

#[test]
fn parser_rejects_non_utf8_branch_names() {
    let output = b"worktree /repo/topic\0HEAD 0123456789abcdef\0branch refs/heads/\xff\0\0";

    assert!(matches!(
        parse_worktrees(output, "git worktree list"),
        Err(GitError::InvalidOutput { .. })
    ));
}
