use crate::CaptureState;
use crate::ChangeFile;
use crate::ChangeFileKind;
use crate::ChangeSetId;
use crate::CommitState;
use crate::MessageState;
use crate::TerminalTurnState;
use crate::TurnChangeError;
use crate::TurnChangeSet;
use crate::TurnChangeSetDraft;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;

#[derive(Default)]
struct MemoryStore {
    records: Mutex<BTreeMap<ChangeSetId, TurnChangeSet>>,
}

impl crate::TurnChangeStore for MemoryStore {
    fn insert(&self, change_set: &TurnChangeSet) -> Result<(), crate::TurnChangeStoreError> {
        let mut records = self.records.lock().unwrap();
        if records.contains_key(&change_set.change_set_id) {
            return Err(crate::TurnChangeStoreError::AlreadyExists(
                change_set.change_set_id.to_string(),
            ));
        }
        records.insert(change_set.change_set_id.clone(), change_set.clone());
        Ok(())
    }

    fn load(
        &self,
        change_set_id: &ChangeSetId,
    ) -> Result<TurnChangeSet, crate::TurnChangeStoreError> {
        self.records
            .lock()
            .unwrap()
            .get(change_set_id)
            .cloned()
            .ok_or_else(|| crate::TurnChangeStoreError::NotFound(change_set_id.to_string()))
    }

    fn list_for_thread(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Vec<TurnChangeSet>, crate::TurnChangeStoreError> {
        Ok(self
            .records
            .lock()
            .unwrap()
            .values()
            .filter(|record| &record.thread_id == thread_id)
            .cloned()
            .collect())
    }

    fn compare_and_swap(
        &self,
        expected_revision: u64,
        change_set: &TurnChangeSet,
    ) -> Result<(), crate::TurnChangeStoreError> {
        let mut records = self.records.lock().unwrap();
        let current = records.get(&change_set.change_set_id).ok_or_else(|| {
            crate::TurnChangeStoreError::NotFound(change_set.change_set_id.to_string())
        })?;
        if current.revision != expected_revision {
            return Err(crate::TurnChangeStoreError::RevisionConflict {
                expected: expected_revision,
                actual: current.revision,
            });
        }
        records.insert(change_set.change_set_id.clone(), change_set.clone());
        Ok(())
    }
}

fn open_change_set() -> TurnChangeSet {
    TurnChangeSet::open(TurnChangeSetDraft {
        change_set_id: ChangeSetId::new("changes-1").unwrap(),
        session_id: SessionId::new("session-1").unwrap(),
        thread_id: ThreadId::new("thread-1").unwrap(),
        turn_id: TurnId::new("turn-1").unwrap(),
        repository_id: "repository-1".into(),
        worktree_root: PathBuf::from("/dir/repository-1"),
        target_branch: Some("main".into()),
        base_object_id: Some("head".into()),
        before_tree: "before".into(),
        snapshot_backend: crate::SnapshotBackend::Git,
        baseline_dependency_paths: BTreeSet::new(),
        message_state: MessageState::Unconfigured,
    })
    .unwrap()
}

fn file() -> ChangeFile {
    ChangeFile {
        path: PathBuf::from("src/lib.rs"),
        previous_path: None,
        kind: ChangeFileKind::Modified,
        before_object_id: Some("old".into()),
        after_object_id: Some("new".into()),
        before_mode: Some("100644".into()),
        after_mode: Some("100644".into()),
        binary: false,
        additions: 3,
        deletions: 1,
    }
}

#[test]
fn sealed_change_set_preserves_manual_draft_when_generation_finishes() {
    let mut change_set = open_change_set();
    change_set
        .seal(
            "after".into(),
            TerminalTurnState::Completed,
            vec![file()],
            BTreeSet::new(),
        )
        .unwrap();
    change_set.queue_message().unwrap();
    change_set.begin_message_generation().unwrap();
    change_set
        .update_draft("fix(core): keep the manual wording".into())
        .unwrap();
    change_set
        .finish_message_generation("fix(core): generated wording".into())
        .unwrap();

    assert_eq!(
        (
            change_set.capture_state,
            change_set.message_state,
            change_set.generated_message.as_deref(),
            change_set.draft_message.as_deref(),
        ),
        (
            CaptureState::Sealed,
            MessageState::Ready,
            Some("fix(core): generated wording"),
            Some("fix(core): keep the manual wording"),
        )
    );
}

#[test]
fn unresolved_dependency_prevents_commit_queueing() {
    let mut change_set = open_change_set();
    change_set
        .seal(
            "after".into(),
            TerminalTurnState::Interrupted,
            vec![file()],
            BTreeSet::from([ChangeSetId::new("dependency").unwrap()]),
        )
        .unwrap();
    change_set
        .update_draft("fix(core): retain interrupted work".into())
        .unwrap();

    assert_eq!(
        change_set.queue_commit(),
        Err(TurnChangeError::UnresolvedDependencies)
    );
    assert_eq!(change_set.commit_state, CommitState::Idle);
}

#[test]
fn initial_dir_dependency_prevents_commit_queueing() {
    let mut change_set = open_change_set();
    change_set
        .baseline_dependency_paths
        .insert(PathBuf::from("src/config.rs"));
    change_set
        .record_tool_scope([PathBuf::from("src/config.rs")], [], false)
        .unwrap();
    change_set
        .seal(
            "after".into(),
            TerminalTurnState::Completed,
            vec![file()],
            BTreeSet::new(),
        )
        .unwrap();
    change_set
        .update_draft("fix(core): use dir config".into())
        .unwrap();

    assert_eq!(
        change_set.queue_commit(),
        Err(TurnChangeError::UnresolvedExternalDependencies)
    );
    assert_eq!(
        change_set.external_dependency_paths,
        BTreeSet::from([PathBuf::from("src/config.rs")])
    );
    change_set
        .satisfy_external_dependencies([PathBuf::from("src/config.rs")])
        .unwrap();
    change_set.queue_commit().unwrap();
    assert_eq!(change_set.commit_state, CommitState::Queued);
}

#[test]
fn open_or_incomplete_change_set_cannot_be_committed() {
    let mut change_set = open_change_set();
    change_set
        .update_draft("feat(core): add change capture".into())
        .unwrap();
    assert_eq!(
        change_set.queue_commit(),
        Err(TurnChangeError::InvalidTransition)
    );
    change_set.mark_incomplete("late dir write".into()).unwrap();
    assert_eq!(
        change_set.queue_commit(),
        Err(TurnChangeError::InvalidTransition)
    );
}

#[test]
fn ledger_seals_rename_mode_and_line_statistics_from_immutable_trees() {
    let directory = tempfile::tempdir().unwrap();
    run_git(
        directory.path(),
        &["init", "--quiet", "--initial-branch=main"],
    );
    std::fs::write(directory.path().join("before.txt"), "one\ntwo\n").unwrap();
    run_git(directory.path(), &["add", "."]);
    run_git(directory.path(), &["commit", "--quiet", "-m", "initial"]);
    let head = run_git(directory.path(), &["rev-parse", "HEAD"]);
    let store = std::sync::Arc::new(MemoryStore::default());
    let ledger = crate::TurnChangeLedger::start(store).unwrap();
    let session_id = SessionId::new("session-1").unwrap();
    let thread_id = ThreadId::new("thread-1").unwrap();
    let turn_id = TurnId::new("turn-1").unwrap();
    ledger
        .begin_turn(crate::TurnChangeBeginRequest {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            repositories: vec![crate::RepositoryCaptureTarget {
                repository_id: "repository-1".into(),
                worktree_root: directory.path().to_path_buf(),
                target_branch: Some("main".into()),
                base_object_id: Some(head),
                snapshot_backend: crate::SnapshotBackend::Git,
                baseline_dependency_paths: BTreeSet::new(),
            }],
            commit_message_configured: true,
            opaque_dependencies: false,
        })
        .unwrap();

    std::fs::rename(
        directory.path().join("before.txt"),
        directory.path().join("after.txt"),
    )
    .unwrap();
    std::fs::write(directory.path().join("after.txt"), "one\ntwo\nthree\n").unwrap();
    ledger
        .record_tool_scope(crate::ToolChangeScope {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            read_paths: BTreeSet::from([PathBuf::from("before.txt")]),
            write_paths: BTreeSet::from([PathBuf::from("before.txt"), PathBuf::from("after.txt")]),
            repository_paths: BTreeMap::from([("repository-1".into(), PathBuf::from("."))]),
            opaque_dependencies: false,
        })
        .unwrap();
    let open = ledger
        .refresh_turn(session_id.clone(), thread_id.clone(), turn_id.clone())
        .unwrap();
    assert_eq!(open[0].capture_state, CaptureState::Open);
    assert_eq!(open[0].files.len(), 1);
    assert_eq!(open[0].files[0].kind, ChangeFileKind::Renamed);
    let sealed = ledger
        .seal_turn(crate::TurnChangeSealRequest {
            session_id,
            thread_id,
            turn_id,
            terminal_state: TerminalTurnState::Completed,
        })
        .unwrap();

    assert_eq!(sealed.len(), 1);
    assert_eq!(sealed[0].capture_state, CaptureState::Sealed);
    assert_eq!(sealed[0].files.len(), 1);
    assert_eq!(sealed[0].files[0].kind, ChangeFileKind::Renamed);
    assert_eq!(sealed[0].files[0].path, PathBuf::from("after.txt"));
    assert_eq!(
        sealed[0].files[0].previous_path,
        Some(PathBuf::from("before.txt"))
    );
    assert_eq!(sealed[0].files[0].additions, 1);
    assert_eq!(sealed[0].files[0].deletions, 0);
    assert_eq!(sealed[0].files[0].before_mode.as_deref(), Some("100644"));
    assert_eq!(sealed[0].files[0].after_mode.as_deref(), Some("100644"));
}

#[test]
fn opaque_reads_do_not_claim_writes_outside_a_recorded_execution_window() {
    let directory = tempfile::tempdir().unwrap();
    run_git(
        directory.path(),
        &["init", "--quiet", "--initial-branch=main"],
    );
    std::fs::write(directory.path().join("tracked.txt"), "before\n").unwrap();
    run_git(directory.path(), &["add", "."]);
    run_git(directory.path(), &["commit", "--quiet", "-m", "initial"]);
    let head = run_git(directory.path(), &["rev-parse", "HEAD"]);
    let store = std::sync::Arc::new(MemoryStore::default());
    let ledger = crate::TurnChangeLedger::start(store).unwrap();
    let session_id = SessionId::new("session-opaque").unwrap();
    let thread_id = ThreadId::new("thread-opaque").unwrap();
    let turn_id = TurnId::new("turn-opaque").unwrap();
    ledger
        .begin_turn(crate::TurnChangeBeginRequest {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            repositories: vec![crate::RepositoryCaptureTarget {
                repository_id: "repository-opaque".into(),
                worktree_root: directory.path().to_path_buf(),
                target_branch: Some("main".into()),
                base_object_id: Some(head),
                snapshot_backend: crate::SnapshotBackend::Git,
                baseline_dependency_paths: BTreeSet::new(),
            }],
            commit_message_configured: false,
            opaque_dependencies: true,
        })
        .unwrap();

    std::fs::write(directory.path().join("tracked.txt"), "outside lifecycle\n").unwrap();
    let sealed = ledger
        .seal_turn(crate::TurnChangeSealRequest {
            session_id,
            thread_id,
            turn_id,
            terminal_state: TerminalTurnState::Completed,
        })
        .unwrap();

    assert_eq!(sealed[0].capture_state, CaptureState::Incomplete);
    assert!(
        sealed[0].warnings[0].contains("writes outside a known Tool lifecycle"),
        "warning was {:?}",
        sealed[0].warnings
    );
}

#[test]
fn directory_snapshots_capture_and_restore_non_git_changes() {
    let dir = tempfile::tempdir().unwrap();
    let objects = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("before.txt"), "one\ntwo\n").unwrap();
    std::fs::write(dir.path().join("binary.bin"), [0, 1, 2]).unwrap();
    let snapshots = crate::DirectorySnapshotStore::new(objects.path());
    let before = snapshots.capture(dir.path()).unwrap();

    std::fs::rename(dir.path().join("before.txt"), dir.path().join("after.txt")).unwrap();
    std::fs::write(dir.path().join("after.txt"), "one\ntwo\nthree\n").unwrap();
    std::fs::remove_file(dir.path().join("binary.bin")).unwrap();
    std::fs::write(dir.path().join("added.txt"), "added\n").unwrap();
    let after = snapshots.capture(dir.path()).unwrap();
    let changes = snapshots.diff(&before, &after).unwrap();

    assert!(
        changes
            .iter()
            .any(|change| change.path == PathBuf::from("after.txt"))
    );
    assert!(changes.iter().any(|change| {
        change.path == PathBuf::from("binary.bin")
            && change.kind == ChangeFileKind::Deleted
            && change.binary
    }));
    assert!(changes.iter().any(|change| {
        change.path == PathBuf::from("added.txt") && change.kind == ChangeFileKind::Added
    }));

    snapshots.replace_directory(dir.path(), &before).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("before.txt")).unwrap(),
        "one\ntwo\n"
    );
    assert!(!dir.path().join("after.txt").exists());
    assert_eq!(
        std::fs::read(dir.path().join("binary.bin")).unwrap(),
        [0, 1, 2]
    );
}

fn run_git(root: &std::path::Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "user.name=Zeta Test",
            "-c",
            "user.email=zeta@example.invalid",
            "-c",
            "commit.gpgSign=false",
            "-c",
            if cfg!(windows) {
                "core.hooksPath=NUL"
            } else {
                "core.hooksPath=/dev/null"
            },
        ])
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}
