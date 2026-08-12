use super::*;
use std::fs;
use tempfile::TempDir;

fn workspace() -> TempDir {
    let directory = tempfile::tempdir().expect("workspace");
    fs::create_dir(directory.path().join(".git")).expect("git marker");
    directory
}

#[test]
fn search_revalidates_content_and_marks_a_lagging_projection_stale() {
    let directory = workspace();
    let source_path = directory.path().join("lib.rs");
    fs::write(&source_path, "pub fn before_watcher() {}\n").expect("source");
    let runtime = CodeIndexRuntime::open(
        WorkspaceRoot::open(directory.path()).expect("root"),
        CodeIndexStorage::Memory,
    )
    .expect("runtime");
    runtime.rebuild().expect("rebuild");

    fs::write(&source_path, "pub fn after_watcher() {}\n").expect("changed source");
    let error = runtime
        .search(&CodeIndexQuery::new("before_watcher"))
        .expect_err("stale result must be rejected");

    assert!(matches!(
        error,
        CodeIndexRuntimeError::Index(CodeIndexError::StaleRevision { .. })
    ));
    assert!(matches!(runtime.state(), CodeIndexRuntimeState::Stale(_)));
}

#[test]
fn reopened_projection_remains_stale_until_the_workspace_is_reconciled() {
    let directory = workspace();
    fs::write(directory.path().join("lib.rs"), "pub fn persisted() {}\n").expect("source");
    let state = tempfile::tempdir().expect("state");
    let storage = CodeIndexStorage::Persistent(state.path().join("code-index.sqlite3"));
    let runtime = CodeIndexRuntime::open(
        WorkspaceRoot::open(directory.path()).expect("root"),
        storage.clone(),
    )
    .expect("runtime");
    runtime.rebuild().expect("rebuild");
    drop(runtime);

    let reopened = CodeIndexRuntime::open(
        WorkspaceRoot::open(directory.path()).expect("root"),
        storage,
    )
    .expect("reopen");
    assert!(matches!(reopened.state(), CodeIndexRuntimeState::Stale(_)));
    reopened.rebuild().expect("reconcile");
    assert!(matches!(reopened.state(), CodeIndexRuntimeState::Ready(_)));
}

#[test]
fn irrelevant_watcher_hint_returns_runtime_to_ready_without_a_generation_change() {
    let directory = workspace();
    fs::write(directory.path().join("lib.rs"), "pub fn current() {}\n").expect("source");
    fs::create_dir(directory.path().join(".zeta")).expect("runtime directory");
    let runtime_path = directory.path().join(".zeta/runtime.json");
    fs::write(&runtime_path, "{}\n").expect("runtime source");
    let runtime = CodeIndexRuntime::open(
        WorkspaceRoot::open(directory.path()).expect("root"),
        CodeIndexStorage::Memory,
    )
    .expect("runtime");
    let before = runtime.rebuild().expect("rebuild");

    runtime.apply_watcher_event(&FileWatcherEvent::PathsChanged {
        paths: vec![runtime_path],
    });

    let CodeIndexRuntimeState::Ready(after) = runtime.state() else {
        panic!("irrelevant hint should restore ready state");
    };
    assert_eq!(after.generation, before.generation);
}

#[test]
fn irrelevant_watcher_hint_does_not_clear_a_stale_projection() {
    let directory = workspace();
    fs::write(directory.path().join("lib.rs"), "pub fn persisted() {}\n").expect("source");
    fs::create_dir(directory.path().join(".zeta")).expect("runtime directory");
    let runtime_path = directory.path().join(".zeta/runtime.json");
    fs::write(&runtime_path, "{}\n").expect("runtime source");
    let state = tempfile::tempdir().expect("state");
    let storage = CodeIndexStorage::Persistent(state.path().join("code-index.sqlite3"));
    let runtime = CodeIndexRuntime::open(
        WorkspaceRoot::open(directory.path()).expect("root"),
        storage.clone(),
    )
    .expect("runtime");
    runtime.rebuild().expect("rebuild");
    drop(runtime);

    let reopened = CodeIndexRuntime::open(
        WorkspaceRoot::open(directory.path()).expect("root"),
        storage,
    )
    .expect("reopen");
    reopened.apply_watcher_event(&FileWatcherEvent::PathsChanged {
        paths: vec![runtime_path],
    });

    assert!(matches!(reopened.state(), CodeIndexRuntimeState::Stale(_)));
}
