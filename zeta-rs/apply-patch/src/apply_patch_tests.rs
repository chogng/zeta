use super::*;
use serde_json::json;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};
use zeta_async_utils::CancellationSource;
use zeta_protocol::{ToolCallId, TurnId};
use zeta_sandboxing::WorkspaceRoot;
use zeta_tools::{
    ToolBinding, ToolBindingId, ToolDefinition, ToolEnvironmentId, ToolExecutionContext,
    ToolExecutionOutcome, ToolExecutor, ToolInvocation, ToolOperationId, ToolOutputStatus,
    ToolPayload, ToolRegistryGeneration, ToolRuntimeKey,
};

#[test]
fn applies_an_update_and_an_add_after_preparing_the_whole_patch() {
    let workspace = TestWorkspace::new();
    workspace.write("src/lib.rs", "pub fn old() {}\n");
    let tool = ApplyPatchTool::new(
        environment_id(),
        workspace.root(),
        ApplyPatchLimits::default(),
    )
    .unwrap();
    let definition = tool.definition();
    let patch = "*** Begin Patch\n*** Update File: src/lib.rs\n@@\n-pub fn old() {}\n+pub fn new() {}\n*** Add File: src/new.rs\n+pub fn added() {}\n*** End Patch\n";

    let outcome = resolve(tool.execute(invocation(&definition, json!({"patch": patch}))));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("patch should return a tool output");
    };
    assert_eq!(output.status(), ToolOutputStatus::Success);
    assert_eq!(
        fs::read_to_string(workspace.path().join("src/lib.rs")).unwrap(),
        "pub fn new() {}\n"
    );
    assert_eq!(
        fs::read_to_string(workspace.path().join("src/new.rs")).unwrap(),
        "pub fn added() {}\n"
    );
}

#[test]
fn failed_later_operation_does_not_commit_an_earlier_add() {
    let workspace = TestWorkspace::new();
    workspace.write("src/lib.rs", "actual\n");
    let tool = ApplyPatchTool::new(
        environment_id(),
        workspace.root(),
        ApplyPatchLimits::default(),
    )
    .unwrap();
    let definition = tool.definition();
    let patch = "*** Begin Patch\n*** Add File: src/new.rs\n+new\n*** Update File: src/lib.rs\n@@\n-expected\n+replacement\n*** End Patch\n";

    let outcome = resolve(tool.execute(invocation(&definition, json!({"patch": patch}))));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("invalid hunk should return a tool error");
    };
    assert_eq!(output.status(), ToolOutputStatus::Error);
    assert!(!workspace.path().join("src/new.rs").exists());
    assert_eq!(
        fs::read_to_string(workspace.path().join("src/lib.rs")).unwrap(),
        "actual\n"
    );
}

#[test]
fn deletes_an_existing_workspace_file() {
    let workspace = TestWorkspace::new();
    workspace.write("src/obsolete.rs", "obsolete\n");
    let tool = ApplyPatchTool::new(
        environment_id(),
        workspace.root(),
        ApplyPatchLimits::default(),
    )
    .unwrap();
    let definition = tool.definition();
    let patch = "*** Begin Patch\n*** Delete File: src/obsolete.rs\n*** End Patch\n";

    let outcome = resolve(tool.execute(invocation(&definition, json!({"patch": patch}))));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("delete should return a tool output");
    };
    assert_eq!(output.status(), ToolOutputStatus::Success);
    assert!(!workspace.path().join("src/obsolete.rs").exists());
    assert!(format!("{:?}", output.content()).contains("src/obsolete.rs"));
}

#[test]
fn rejects_parent_directory_paths() {
    let workspace = TestWorkspace::new();
    let tool = ApplyPatchTool::new(
        environment_id(),
        workspace.root(),
        ApplyPatchLimits::default(),
    )
    .unwrap();
    let definition = tool.definition();
    let patch = "*** Begin Patch\n*** Add File: ../outside.txt\n+no\n*** End Patch\n";

    let outcome = resolve(tool.execute(invocation(&definition, json!({"patch": patch}))));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("invalid patch path should return a tool error");
    };
    assert_eq!(output.status(), ToolOutputStatus::Error);
    assert!(format!("{:?}", output.content()).contains("must be relative"));
}

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-apply-patch-tests-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn root(&self) -> WorkspaceRoot {
        WorkspaceRoot::open(&self.path).unwrap()
    }

    fn write(&self, relative: impl AsRef<Path>, content: &str) {
        let target = self.path.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, content).unwrap();
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn environment_id() -> ToolEnvironmentId {
    ToolEnvironmentId::new("test-environment").unwrap()
}

fn invocation(definition: &ToolDefinition, arguments: serde_json::Value) -> ToolInvocation {
    ToolInvocation::new(
        ToolOperationId::new("operation-1").unwrap(),
        ToolCallId::new("call-1").unwrap(),
        TurnId::new("turn-1").unwrap(),
        ToolBinding::new(
            ToolRegistryGeneration::new(1),
            ToolBindingId::new("binding-1").unwrap(),
            definition.name().clone(),
            definition.digest(),
            ToolRuntimeKey::new("local:test").unwrap(),
        ),
        ToolPayload::FunctionArguments(arguments),
        ToolExecutionContext::new(environment_id(), CancellationSource::new().token()),
    )
}

fn resolve(
    mut future: Pin<Box<dyn Future<Output = ToolExecutionOutcome> + Send + '_>>,
) -> ToolExecutionOutcome {
    let waker: &Waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(outcome) => outcome,
        Poll::Pending => panic!("local tool future should complete synchronously"),
    }
}
