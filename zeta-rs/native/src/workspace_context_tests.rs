use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{WorkspaceContext, display_working_directory};

static NEXT_REPOSITORY_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn home_relative_working_directory_uses_a_compact_label() {
    assert_eq!(
        display_working_directory(
            Path::new("/Users/lance/Desktop/zeta"),
            Some(Path::new("/Users/lance")),
        ),
        "~/Desktop/zeta"
    );
    assert_eq!(
        display_working_directory(Path::new("/Users/lance"), Some(Path::new("/Users/lance"))),
        "~"
    );
}

#[test]
fn fixture_exposes_all_four_toolbar_values_without_inventing_git_state() {
    let repository = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(3));
    let plain_directory = WorkspaceContext::fixture("/tmp/plain", None, None);

    assert_eq!(repository.location_label(), "Local");
    assert_eq!(repository.working_directory_label(), "~/Desktop/zeta");
    assert_eq!(repository.git_branch_label(), "main");
    assert_eq!(repository.diff_summary_label(), "Changes 3 • +3 -0");
    assert_eq!(plain_directory.git_branch_label(), "No Git");
    assert_eq!(plain_directory.diff_summary_label(), "Changes —");
}

#[test]
fn repository_capture_builds_real_changed_file_diffs() {
    let root = std::env::temp_dir().join(format!(
        "zeta-native-workspace-context-{}-{}",
        std::process::id(),
        NEXT_REPOSITORY_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    run_git(&root, &["init", "--initial-branch=main"]);
    run_git(&root, &["config", "user.name", "Zeta Test"]);
    run_git(&root, &["config", "user.email", "zeta@example.invalid"]);
    std::fs::write(root.join("tracked.txt"), "before\n").unwrap();
    std::fs::write(root.join("deleted.txt"), "gone\n").unwrap();
    run_git(&root, &["add", "tracked.txt", "deleted.txt"]);
    run_git(&root, &["commit", "-m", "initial"]);
    std::fs::write(root.join("tracked.txt"), "after\n").unwrap();
    std::fs::remove_file(root.join("deleted.txt")).unwrap();
    std::fs::write(root.join("untracked.txt"), "new\nfile\n").unwrap();

    let context = WorkspaceContext::capture(root.clone());

    assert_eq!(context.git_branch_label(), "main");
    assert_eq!(context.diff_summary_label(), "Changes 3 • +3 -2");
    let tracked = context
        .diffs()
        .iter()
        .find(|diff| diff.path() == "tracked.txt")
        .unwrap();
    assert!(!tracked.document().hunks().is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn switching_working_directory_replaces_path_and_repository_projection() {
    let root = std::env::temp_dir().join(format!(
        "zeta-native-workspace-switch-{}-{}",
        std::process::id(),
        NEXT_REPOSITORY_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let child = root.join("plain");
    std::fs::create_dir_all(&child).unwrap();
    let mut context = WorkspaceContext::capture_current();

    context.switch_working_directory(child.clone()).unwrap();

    assert_eq!(context.working_directory(), child.canonicalize().unwrap());
    assert_eq!(context.git_branch_label(), "No Git");
    assert_eq!(context.diff_summary_label(), "Changes —");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn switching_branch_refreshes_the_repository_projection() {
    let root = std::env::temp_dir().join(format!(
        "zeta-native-branch-switch-{}-{}",
        std::process::id(),
        NEXT_REPOSITORY_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    run_git(&root, &["init", "--initial-branch=main"]);
    run_git(&root, &["config", "user.name", "Zeta Test"]);
    run_git(&root, &["config", "user.email", "zeta@example.invalid"]);
    std::fs::write(root.join("tracked.txt"), "main\n").unwrap();
    run_git(&root, &["add", "tracked.txt"]);
    run_git(&root, &["commit", "-m", "initial"]);
    run_git(&root, &["branch", "topic"]);
    let mut context = WorkspaceContext::capture_current();
    context.switch_working_directory(root.clone()).unwrap();
    let topic = context
        .local_branches()
        .unwrap()
        .into_iter()
        .find(|branch| branch.name() == "topic")
        .unwrap();

    context.switch_branch(&topic).unwrap();

    assert_eq!(context.git_branch_label(), "topic");
    std::fs::remove_dir_all(root).unwrap();
}

fn run_git(root: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
