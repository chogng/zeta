use super::*;

#[test]
fn workspace_root_keeps_its_canonical_path() {
    let root = WorkspaceRoot::open(".").unwrap();
    assert!(root.path().is_absolute());
}

#[test]
fn rejects_parent_directory_before_resolving_a_write_target() {
    let root = WorkspaceRoot::open(".").unwrap();

    assert_eq!(
        root.resolve_for_write("../outside.txt"),
        Err(SandboxError::InvalidRelativePath("../outside.txt".into()))
    );
}
