use super::*;
use std::fs;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[test]
fn manager_discards_snapshots_from_superseded_query_revisions() {
    let workspace = temporary_workspace();
    fs::write(workspace.join("alpha.rs"), "alpha").unwrap();
    fs::write(workspace.join("beta.rs"), "beta").unwrap();
    let mut manager = FileSearchManager::new(workspace.clone());

    manager.update_query("alpha");
    manager.update_query("beta");
    manager.update_query("alpha");
    let expected_revision = manager.latest_query_revision.unwrap();
    let snapshots = wait_for_results(&mut manager, |snapshot| snapshot.search_complete);

    assert!(snapshots.iter().all(|snapshot| {
        snapshot.query == "alpha" && snapshot.query_revision == expected_revision
    }));
    assert_eq!(
        snapshots.last().unwrap().matches[0].path,
        std::path::Path::new("alpha.rs")
    );
    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn stopping_a_query_drops_the_handle_and_pending_results() {
    let workspace = temporary_workspace();
    fs::write(workspace.join("notes.md"), "notes").unwrap();
    let mut manager = FileSearchManager::new(workspace.clone());

    manager.update_query("notes");
    manager.stop();

    assert!(manager.poll().is_empty());
    assert!(manager.handle.is_none());
    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn unavailable_search_root_finishes_with_an_empty_snapshot() {
    let workspace = temporary_workspace();
    fs::remove_dir_all(&workspace).unwrap();
    let mut manager = FileSearchManager::new(workspace);

    manager.update_query("notes");
    let snapshots = manager.poll();

    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].query, "notes");
    assert!(snapshots[0].matches.is_empty());
    assert!(snapshots[0].search_complete);
}

fn wait_for_results(
    manager: &mut FileSearchManager,
    predicate: impl Fn(&PathSearchSnapshot) -> bool,
) -> Vec<PathSearchSnapshot> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut collected = Vec::new();
    loop {
        collected.extend(manager.poll());
        if collected.iter().any(&predicate) {
            return collected;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for file-search manager results"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn temporary_workspace() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "zeta-tui-file-search-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}
