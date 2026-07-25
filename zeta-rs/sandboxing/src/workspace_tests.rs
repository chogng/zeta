use super::*;

#[test]
fn workspace_root_keeps_its_canonical_path() {
    let root = WorkspaceRoot::open(".").unwrap();
    assert!(root.path().is_absolute());
}
