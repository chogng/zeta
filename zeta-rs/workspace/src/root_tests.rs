use super::*;
use std::fs;

#[test]
fn opens_a_directory_with_absolute_namespaces() {
    let directory = tempfile::tempdir().unwrap();
    let root = WorkspaceRoot::open(directory.path()).unwrap();

    assert!(root.requested_path().is_absolute());
    assert!(root.canonical_path().is_absolute());
}

#[test]
fn rejects_files_as_workspace_roots() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("file.txt");
    fs::write(&file, "not a directory").unwrap();

    assert!(matches!(
        WorkspaceRoot::open(&file),
        Err(WorkspacePathError::RootNotDirectory(path)) if path == dunce::canonicalize(&file).unwrap()
    ));
}

#[test]
fn rejects_parent_directory_before_resolving_a_write_target() {
    let directory = tempfile::tempdir().unwrap();
    let root = WorkspaceRoot::open(directory.path()).unwrap();

    assert_eq!(
        root.resolve_for_write("../outside.txt"),
        Err(WorkspacePathError::InvalidRelativePath(
            "../outside.txt".into()
        ))
    );
}

#[test]
#[cfg(unix)]
fn rejects_existing_symlink_that_leaves_the_workspace() {
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    create_directory_symlink(outside.path(), &directory.path().join("escape"));
    let root = WorkspaceRoot::open(directory.path()).unwrap();

    assert!(matches!(
        root.resolve_existing("escape"),
        Err(WorkspacePathError::OutsideWorkspace(path)) if path == dunce::canonicalize(outside.path()).unwrap()
    ));
}

#[test]
#[cfg(unix)]
fn projects_requested_and_canonical_observer_namespaces() {
    let directory = tempfile::tempdir().unwrap();
    let alias_parent = tempfile::tempdir().unwrap();
    let alias = alias_parent.path().join("workspace-alias");
    create_directory_symlink(directory.path(), &alias);
    let root = WorkspaceRoot::open(&alias).unwrap();

    assert_eq!(
        root.project_observed_path(alias.join("removed.txt")),
        Some(PathBuf::from("removed.txt"))
    );
    assert_eq!(
        root.project_observed_path(
            dunce::canonicalize(directory.path())
                .unwrap()
                .join("created.txt")
        ),
        Some(PathBuf::from("created.txt"))
    );
}

#[cfg(unix)]
fn create_directory_symlink(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}
