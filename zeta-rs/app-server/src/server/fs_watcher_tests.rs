use super::*;
use std::path::PathBuf;

#[test]
fn changed_paths_are_projected_to_sorted_workspace_relative_paths() {
    let directory = tempfile::tempdir().unwrap();
    let root = WorkspaceRoot::open(directory.path()).unwrap();
    let projected = project_event(
        &root,
        FileWatcherEvent::PathsChanged {
            paths: vec![
                root.requested_path().join("src/main.rs"),
                directory
                    .path()
                    .parent()
                    .unwrap()
                    .join("outside/ignored.rs"),
                root.canonical_path().join("README.md"),
                root.requested_path().join("src/main.rs"),
            ],
        },
    );

    assert_eq!(
        projected,
        Some(FsChanged::PathsChanged {
            paths: vec![PathBuf::from("README.md"), PathBuf::from("src/main.rs")],
        }),
    );
}

#[test]
fn watcher_overflow_becomes_a_root_scoped_rescan_hint() {
    let directory = tempfile::tempdir().unwrap();
    let root = WorkspaceRoot::open(directory.path()).unwrap();
    let projected = project_event(
        &root,
        FileWatcherEvent::RescanRequired {
            watched_paths: vec![root.requested_path().to_path_buf()],
        },
    );

    assert_eq!(projected, Some(FsChanged::RescanRequired));
}
