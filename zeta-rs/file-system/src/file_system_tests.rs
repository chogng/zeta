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
fn reads_text_inside_the_selected_workspace() {
    let workspace = TestWorkspace::new();
    workspace.write("src/main.rs", "fn main() {}\n");
    let tool = FileSystemTool::new(
        environment_id(),
        workspace.root(),
        FileSystemLimits::default(),
    )
    .unwrap();
    let definition = tool.definition();

    let outcome = resolve(tool.execute(invocation(
        &definition,
        json!({"operation": "read", "path": "src/main.rs"}),
    )));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("read should return a tool output");
    };
    assert_eq!(output.status(), ToolOutputStatus::Success);
    assert!(format!("{:?}", output.content()).contains("fn main() {}"));
}

#[test]
fn lists_entries_in_name_order_with_a_bound() {
    let workspace = TestWorkspace::new();
    workspace.write("docs/zeta.md", "z");
    workspace.write("docs/alpha.md", "a");
    let limits = FileSystemLimits::new(64, 1).unwrap();
    let tool = FileSystemTool::new(environment_id(), workspace.root(), limits).unwrap();
    let definition = tool.definition();

    let outcome = resolve(tool.execute(invocation(
        &definition,
        json!({"operation": "list", "path": "docs"}),
    )));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("list should return a tool output");
    };
    let text = format!("{:?}", output.content());
    assert!(text.contains("alpha.md"));
    assert!(!text.contains("zeta.md"));
    assert!(text.contains("truncated"));
}

#[test]
fn rejects_workspace_escape() {
    let workspace = TestWorkspace::new();
    let tool = FileSystemTool::new(
        environment_id(),
        workspace.root(),
        FileSystemLimits::default(),
    )
    .unwrap();
    let definition = tool.definition();

    let outcome = resolve(tool.execute(invocation(
        &definition,
        json!({"operation": "metadata", "path": "../outside"}),
    )));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("escape should become a model-visible tool error");
    };
    assert_eq!(output.status(), ToolOutputStatus::Error);
    assert!(format!("{:?}", output.content()).contains("must be a relative path"));
}

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-file-system-tests-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
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
