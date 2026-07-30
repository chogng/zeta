use super::*;

#[test]
fn changed_paths_are_projected_to_sorted_workspace_relative_paths() {
    let root = Path::new("/workspace");
    let projected = project_event(
        root,
        FileWatcherEvent::PathsChanged {
            paths: vec![
                root.join("src/main.rs"),
                PathBuf::from("/outside/ignored.rs"),
                root.join("README.md"),
                root.join("src/main.rs"),
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
    let projected = project_event(
        Path::new("/workspace"),
        FileWatcherEvent::RescanRequired {
            watched_paths: vec![PathBuf::from("/workspace")],
        },
    );

    assert_eq!(projected, Some(FsChanged::RescanRequired));
}
