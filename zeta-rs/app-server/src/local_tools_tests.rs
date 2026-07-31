#![cfg(unix)]

use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use zeta_async_utils::CancellationSource;
use zeta_protocol::{ToolCallId, ToolName};
use zeta_sandboxing::{PreparedCommand, SandboxCommand, SandboxError, SandboxKind};

struct PassThroughBackend;

impl SandboxBackend for PassThroughBackend {
    fn kind(&self) -> SandboxKind {
        SandboxKind::Unrestricted
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        _: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError> {
        assert!(policy == read_only_sandbox() || policy == shell_sandbox());
        Ok(PreparedCommand::unrestricted(command))
    }
}

#[test]
fn local_registry_exposes_shell_command_and_preserves_read_only_ripgrep() {
    let workspace = TestWorkspace::new();
    let service = LocalShellToolService::new(
        workspace.root(),
        RipgrepExecutable::from_path(workspace.ripgrep()).unwrap(),
        PassThroughBackend,
    )
    .unwrap();
    let definition = &service.definitions()[0];
    assert_eq!(definition.name.as_str(), "shell-command");
    assert_eq!(
        definition.parameters["properties"]["program"]["type"],
        "string"
    );

    let call = tool_call(json!({
        "program": "rg",
        "arguments": ["needle", "."],
        "working_directory": "."
    }));
    let review = service.prepare(&call).unwrap();
    let policy = LocalShellPolicy;
    assert_eq!(
        policy
            .decide(&review, &CancellationSource::new().token())
            .unwrap(),
        ExecutionDecision::RunSandboxed(read_only_sandbox())
    );
    let output = service
        .execute(
            &call,
            &ToolAuthorization::Sandboxed(read_only_sandbox()),
            &CancellationSource::new().token(),
        )
        .unwrap();
    let ToolExecutionOutput::Success(output) = output else {
        panic!("fake ripgrep should complete");
    };
    assert!(output.contains("--no-config needle ."));
}

#[test]
fn local_registry_accepts_shell_processes_but_rejects_ripgrep_workspace_escape_arguments() {
    let workspace = TestWorkspace::new();
    let service = LocalShellToolService::new(
        workspace.root(),
        RipgrepExecutable::from_path(workspace.ripgrep()).unwrap(),
        PassThroughBackend,
    )
    .unwrap();

    let shell = tool_call(json!({
        "program": "/bin/sh",
        "arguments": ["-lc", "printf hello"],
        "working_directory": "."
    }));
    let review = service.prepare(&shell).unwrap();
    assert_eq!(
        LocalShellPolicy
            .decide(&review, &CancellationSource::new().token())
            .unwrap(),
        ExecutionDecision::RunSandboxed(shell_sandbox())
    );

    assert!(
        service
            .prepare(&tool_call(json!({
                "program": "rg",
                "arguments": ["--pre", "decoder", "needle"],
                "working_directory": "."
            })))
            .is_err()
    );
    assert!(
        service
            .prepare(&tool_call(json!({
                "program": "rg",
                "arguments": ["needle", "../outside"],
                "working_directory": "."
            })))
            .is_err()
    );
    std::os::unix::fs::symlink("/etc", workspace.path().join("outside-link")).unwrap();
    assert!(
        service
            .prepare(&tool_call(json!({
                "program": "rg",
                "arguments": ["needle", "outside-link/passwd"],
                "working_directory": "."
            })))
            .is_err()
    );
}

fn tool_call(arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: ToolCallId::new("call-1").unwrap(),
        name: ToolName::new("shell-command").unwrap(),
        arguments,
    }
}

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-local-tools-tests-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn root(&self) -> WorkspaceRoot {
        WorkspaceRoot::open(&self.path).unwrap()
    }

    fn ripgrep(&self) -> PathBuf {
        let path = self.path.join("rg");
        fs::write(&path, "#!/bin/sh\nprintf '%s' \"$*\"\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.path());
    }
}
