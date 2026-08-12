use super::*;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_WORKSPACE: AtomicU64 = AtomicU64::new(1);

#[test]
fn lists_metadata_and_bounded_file_content_inside_workspace() {
    let workspace = TestWorkspace::new();
    fs::create_dir(workspace.path.join("src")).unwrap();
    fs::write(workspace.path.join("src/lib.rs"), "hello").unwrap();
    let file_system = workspace.file_system();

    assert_eq!(
        file_system.read_directory(Path::new("src")).unwrap(),
        vec![DirectoryEntry {
            name: "lib.rs".into(),
            file_type: FileType::File,
        }],
    );
    assert_eq!(
        file_system.read_file(Path::new("src/lib.rs"), 5).unwrap(),
        b"hello",
    );
    let metadata = file_system.get_metadata(Path::new("src/lib.rs")).unwrap();
    assert_eq!(metadata.file_type, FileType::File);
    assert_eq!(metadata.size_bytes, 5);
}

#[test]
fn rejects_parent_traversal_and_read_overflow() {
    let workspace = TestWorkspace::new();
    fs::write(workspace.path.join("large.txt"), "123456").unwrap();
    let file_system = workspace.file_system();

    assert!(matches!(
        file_system.get_metadata(Path::new("../outside")),
        Err(FileSystemError::InvalidPath(_)),
    ));
    assert_eq!(
        file_system.read_file(Path::new("large.txt"), 5),
        Err(FileSystemError::ReadLimitExceeded { maximum_bytes: 5 }),
    );
}

#[test]
fn atomically_replaces_and_creates_bounded_files() {
    let workspace = TestWorkspace::new();
    fs::create_dir(workspace.path.join("src")).unwrap();
    fs::write(workspace.path.join("src/lib.rs"), "old").unwrap();
    let file_system = workspace.file_system();

    let replaced = file_system
        .write_file(Path::new("src/lib.rs"), b"updated", 7)
        .unwrap();
    let created = file_system
        .write_file(Path::new("src/new.rs"), b"new", 7)
        .unwrap();

    assert_eq!(
        fs::read(workspace.path.join("src/lib.rs")).unwrap(),
        b"updated"
    );
    assert_eq!(fs::read(workspace.path.join("src/new.rs")).unwrap(), b"new");
    assert_eq!(replaced.size_bytes, 7);
    assert_eq!(created.size_bytes, 3);
}

#[test]
fn conditionally_writes_only_the_revision_that_was_read() {
    let workspace = TestWorkspace::new();
    fs::write(workspace.path.join("document.txt"), "first").unwrap();
    let file_system = workspace.file_system();
    let read = file_system
        .read_file_with_revision(Path::new("document.txt"), 1024)
        .unwrap();

    file_system
        .write_file_with_condition(
            Path::new("document.txt"),
            b"second",
            1024,
            &FileWriteCondition::ExpectedRevision(read.revision.clone()),
        )
        .unwrap();
    assert_eq!(
        file_system.write_file_with_condition(
            Path::new("document.txt"),
            b"stale",
            1024,
            &FileWriteCondition::ExpectedRevision(read.revision),
        ),
        Err(FileSystemError::RevisionConflict(PathBuf::from(
            "document.txt"
        ))),
    );
    assert_eq!(
        fs::read_to_string(workspace.path.join("document.txt")).unwrap(),
        "second"
    );
}

#[test]
fn rejects_unsafe_or_oversized_write_targets() {
    let workspace = TestWorkspace::new();
    fs::create_dir(workspace.path.join("src")).unwrap();
    let file_system = workspace.file_system();

    assert!(matches!(
        file_system.write_file(Path::new("../outside"), b"content", 7),
        Err(FileSystemError::InvalidPath(_)),
    ));
    assert_eq!(
        file_system.write_file(Path::new("src/large.txt"), b"123456", 5),
        Err(FileSystemError::WriteLimitExceeded { maximum_bytes: 5 }),
    );
    assert!(matches!(
        file_system.write_file(Path::new("missing/file.txt"), b"content", 7),
        Err(FileSystemError::Io(_)),
    ));
    assert!(matches!(
        file_system.write_file(Path::new("src"), b"content", 7),
        Err(FileSystemError::NotFile(_)),
    ));
}

#[test]
fn creates_renames_overwrites_and_deletes_workspace_files() {
    let workspace = TestWorkspace::new();
    fs::write(workspace.path.join("source.txt"), "source").unwrap();
    fs::write(workspace.path.join("target.txt"), "target").unwrap();
    let file_system = workspace.file_system();

    file_system
        .create_file(Path::new("created.txt"), ExistingTargetBehavior::Error)
        .unwrap();
    file_system
        .rename(
            Path::new("source.txt"),
            Path::new("target.txt"),
            ExistingTargetBehavior::Overwrite,
        )
        .unwrap();
    file_system
        .delete(
            Path::new("created.txt"),
            MissingTargetBehavior::Error,
            FileDeleteMode::FileOrEmptyDirectory,
        )
        .unwrap();
    file_system
        .delete(
            Path::new("created.txt"),
            MissingTargetBehavior::Ignore,
            FileDeleteMode::FileOrEmptyDirectory,
        )
        .unwrap();

    assert_eq!(
        fs::read_to_string(workspace.path.join("target.txt")).unwrap(),
        "source"
    );
    assert!(!workspace.path.join("source.txt").exists());
    assert!(!workspace.path.join("created.txt").exists());
}

#[cfg(unix)]
#[test]
fn preserves_existing_file_permissions_during_replacement() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TestWorkspace::new();
    let path = workspace.path.join("script.sh");
    fs::write(&path, "old").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let file_system = workspace.file_system();

    file_system
        .write_file(Path::new("script.sh"), b"new", 3)
        .unwrap();

    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-file-system-tests-{}-{sequence}",
            std::process::id(),
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn file_system(&self) -> LocalFileSystem {
        LocalFileSystem::new(WorkspaceRoot::open(&self.path).unwrap())
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
