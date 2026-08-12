use super::GitRuntime;
use crate::server::notification_queue::NotificationQueue;
use crate::server::update_broker::UpdateBroker;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use zeta_workspace::{
    TrustedWorkspace, WorkspaceCapability, WorkspaceRoot, WorkspaceTrustDecision,
    WorkspaceTrustSource,
};

#[test]
fn runtime_revisions_and_notifies_only_for_changed_workspace_projection() {
    let repository = TestRepository::init();
    repository.write("workspace/tracked.txt", "initial\n");
    repository.write("outside.txt", "initial\n");
    repository.git(&["add", "."]);
    repository.git(&["commit", "-m", "initial"]);
    let broker = Arc::new(UpdateBroker::default());
    let queue = NotificationQueue::default();
    broker.register(1, &queue);
    let trusted = trusted_workspace(&repository.root().join("workspace"));
    let workspace_root = trusted.root().canonical_path().to_path_buf();
    let repository_root = WorkspaceRoot::open(repository.root())
        .unwrap()
        .canonical_path()
        .to_path_buf();
    let runtime = GitRuntime::new(trusted, broker).unwrap();

    let initial = runtime.status().unwrap();
    assert_eq!(initial.revision, 1);
    assert_eq!(initial.workspace_path, "workspace");
    assert!(initial.changes.is_empty());
    assert_eq!(queue.drain().len(), 1);
    let watched_paths = runtime.watched_paths();
    assert!(
        watched_paths
            .iter()
            .any(|watch| { watch.path == workspace_root && watch.recursive })
    );
    assert!(
        watched_paths
            .iter()
            .any(|watch| { watch.path == repository_root.join(".gitignore") && !watch.recursive })
    );
    assert!(
        watched_paths
            .iter()
            .any(|watch| { watch.path == repository_root.join(".git") && watch.recursive })
    );

    repository.write("outside.txt", "outside change\n");
    let outside_only = runtime.status().unwrap();
    assert_eq!(outside_only.stream_instance_id, initial.stream_instance_id);
    assert_eq!(outside_only.revision, 1);
    assert!(outside_only.changes.is_empty());
    assert!(queue.drain().is_empty());

    repository.write("workspace/tracked.txt", "workspace change\n");
    let changed = runtime.status().unwrap();
    assert_eq!(changed.stream_instance_id, initial.stream_instance_id);
    assert_eq!(changed.revision, 2);
    assert_eq!(changed.changes.len(), 1);
    let notifications = queue.drain();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0]["method"], "git/statusChanged");
    assert_eq!(
        notifications[0]["params"]["status"]["streamInstanceId"],
        initial.stream_instance_id.as_str()
    );
    assert_eq!(notifications[0]["params"]["status"]["revision"], 2);

    let unchanged = runtime.status().unwrap();
    assert_eq!(unchanged.revision, 2);
    assert!(queue.drain().is_empty());
}

#[test]
fn runtime_incarnations_use_distinct_revision_scopes() {
    let repository = TestRepository::init();
    let broker = Arc::new(UpdateBroker::default());
    let first = GitRuntime::new(trusted_workspace(repository.root()), Arc::clone(&broker)).unwrap();
    let second = GitRuntime::new(trusted_workspace(repository.root()), broker).unwrap();

    let first_status = first.status().unwrap();
    let second_status = second.status().unwrap();

    assert!(first_status.workspace_path.is_empty());
    assert_ne!(
        first_status.stream_instance_id,
        second_status.stream_instance_id
    );
    assert_eq!(first_status.revision, 1);
    assert_eq!(second_status.revision, 1);
}

#[test]
fn runtime_projects_text_diffs_and_switches_only_existing_local_branches() {
    let repository = TestRepository::init();
    repository.write("tracked.txt", "before\n");
    repository.git(&["add", "tracked.txt"]);
    repository.git(&["commit", "-m", "initial"]);
    repository.git(&["branch", "topic"]);
    repository.write("tracked.txt", "after\n");
    let runtime = GitRuntime::new(
        trusted_workspace(repository.root()),
        Arc::new(UpdateBroker::default()),
    )
    .unwrap();

    let projection = runtime.text_diff().unwrap();

    assert_eq!(projection.status.changes.len(), 1);
    assert_eq!(projection.diffs.len(), 1);
    assert_eq!(projection.diffs[0].path, "tracked.txt");
    assert_eq!(projection.diffs[0].original, "before\n");
    assert_eq!(projection.diffs[0].modified, "after\n");
    assert_eq!(projection.statistics.files, 1);
    let history = runtime.recent_commits().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].subject, "initial");
    assert_eq!(history[0].object_id.len(), 40);
    let branches = runtime.local_branches().unwrap();
    assert!(
        branches
            .iter()
            .any(|branch| branch.name == "main" && branch.current)
    );
    assert!(
        branches
            .iter()
            .any(|branch| branch.name == "topic" && !branch.current)
    );

    let switched = runtime.switch_branch("topic").unwrap();
    assert!(matches!(
        switched.head,
        zeta_app_server_protocol::protocol::git::GitHeadDto::Branch { ref name, .. }
            if name == "topic"
    ));
    assert!(runtime.switch_branch("missing").is_err());
}

struct TestRepository {
    root: PathBuf,
}

impl TestRepository {
    fn init() -> Self {
        let root = std::env::temp_dir().join(format!(
            "zeta-app-server-git-runtime-{}-{}",
            std::process::id(),
            unique_sequence()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let repository = Self { root };
        repository.git(&["init", "--initial-branch=main"]);
        repository.git(&["config", "user.name", "Zeta Test"]);
        repository.git(&["config", "user.email", "zeta@example.invalid"]);
        repository
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    fn git(&self, arguments: &[&str]) {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(&self.root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

impl Drop for TestRepository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn unique_sequence() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn trusted_workspace(root: &Path) -> TrustedWorkspace {
    TrustedWorkspace::require(
        WorkspaceRoot::open(root).unwrap(),
        WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::HostConfiguration),
        WorkspaceCapability::MutateRepository,
    )
    .unwrap()
}
