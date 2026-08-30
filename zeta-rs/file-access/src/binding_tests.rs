use super::*;

#[test]
fn binding_freezes_canonical_directory_identity() {
    let directory = tempfile::tempdir().unwrap();
    let root = Dir::open_local(directory.path()).unwrap();

    let binding = DirBinding::from_dir(&root);

    assert_eq!(binding.path(), root.canonical_path());
    assert_eq!(binding.id, root.id());
    assert!(binding.matches(&root));
}

#[test]
fn binding_rejects_another_directory() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let first = Dir::open_local(first.path()).unwrap();
    let second = Dir::open_local(second.path()).unwrap();

    assert!(!DirBinding::from_dir(&first).matches(&second));
}
