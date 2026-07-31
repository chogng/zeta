use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn workspace_write_is_a_read_only_root_with_a_writable_workspace_overlay() {
    let fixture = TestWorkspace::new();
    for name in PROTECTED_WORKSPACE_METADATA_NAMES {
        fs::create_dir_all(fixture.path.join(name)).unwrap();
    }
    let workspace = fixture.root();
    let command = SandboxCommand::new("echo", ["hello"], workspace.canonical_path());
    let policy = SandboxPolicy::new(FileSystemAccess::WorkspaceWrite, NetworkAccess::Denied);

    let prepared =
        LinuxSandbox::new("/usr/bin/bwrap").prepare_command(&command, policy, &workspace);
    let arguments: Vec<_> = prepared
        .arguments()
        .iter()
        .map(|argument| argument.to_string_lossy())
        .collect();

    assert_eq!(prepared.kind(), SandboxKind::LinuxBubblewrap);
    assert_eq!(prepared.program(), "/usr/bin/bwrap");
    assert!(
        arguments
            .windows(3)
            .any(|args| args == ["--ro-bind", "/", "/"])
    );
    let workspace_path = workspace.canonical_path().to_string_lossy();
    assert!(
        arguments
            .windows(3)
            .any(|args| args == ["--bind", workspace_path.as_ref(), workspace_path.as_ref()])
    );
    for name in PROTECTED_WORKSPACE_METADATA_NAMES {
        let path = workspace.canonical_path().join(name);
        if !path.exists() {
            continue;
        }
        let path = path.to_string_lossy();
        assert!(
            arguments
                .windows(3)
                .any(|args| args == ["--ro-bind", path.as_ref(), path.as_ref()]),
            "bubblewrap invocation did not protect {name}"
        );
    }
    assert!(arguments.iter().any(|argument| argument == "--unshare-net"));
}

#[test]
fn bubblewrap_denial_classification_requires_a_platform_marker() {
    let backend = LinuxSandbox::new("/usr/bin/bwrap");

    assert_eq!(
        backend.classify_denial(
            SandboxProcessExitStatus::Code(1),
            "",
            "touch: Read-only file system",
        ),
        Some(SandboxProcessDenial::process_may_have_started(
            "Linux Bubblewrap denied the sandboxed process operation"
        ))
    );
    assert_eq!(
        backend.classify_denial(
            SandboxProcessExitStatus::Code(1),
            "",
            "application returned an error",
        ),
        None
    );
    assert_eq!(
        backend.classify_denial(
            SandboxProcessExitStatus::Code(1),
            "",
            "bwrap: setting up uid map: permission denied",
        ),
        Some(SandboxProcessDenial::before_process_start(
            "Linux Bubblewrap could not establish the sandbox"
        ))
    );
}

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-linux-sandbox-tests-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn root(&self) -> WorkspaceRoot {
        WorkspaceRoot::open(&self.path).unwrap()
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
