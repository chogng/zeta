use super::*;

#[test]
fn binding_freezes_canonical_root_and_authority() {
    let directory = tempfile::tempdir().unwrap();
    let root = WorkspaceRoot::open(directory.path()).unwrap();

    let binding = WorkspaceBinding::from_root(&root);

    assert_eq!(binding.root(), root.canonical_path());
    assert_eq!(binding.authority_id, root.trust_id());
    assert!(binding.matches_root(&root));
}

#[test]
fn binding_rejects_another_workspace_authority() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let first = WorkspaceRoot::open(first.path()).unwrap();
    let second = WorkspaceRoot::open(second.path()).unwrap();

    assert!(!WorkspaceBinding::from_root(&first).matches_root(&second));
}
