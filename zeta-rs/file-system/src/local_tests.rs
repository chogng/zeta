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
