use super::*;
use serde_json::json;
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use zeta_async_utils::CancellationSource;
use zeta_protocol::{ToolCallId, TurnId};
use zeta_sandboxing::WorkspaceRoot;
use zeta_tools::{
    ToolBinding, ToolBindingId, ToolDefinition, ToolEnvironmentId, ToolExecutionContext,
    ToolExecutionOutcome, ToolExecutor, ToolInvocation, ToolOperationId, ToolOutputStatus,
    ToolPayload, ToolRegistryGeneration, ToolRuntimeKey,
};

struct DenyAll;

impl ApprovalPolicy for DenyAll {
    fn requirement_for(&self, _: &str) -> zeta_exec::ApprovalRequirement {
        zeta_exec::ApprovalRequirement::Denied
    }
}

#[test]
fn denied_command_is_returned_as_a_model_visible_error() {
    let workspace = TestWorkspace::new();
    let tool = ShellCommandTool::new(
        environment_id(),
        workspace.root(),
        DenyAll,
        ShellCommandLimits {
            timeout: Duration::from_secs(1),
            max_output_bytes: 1024,
        },
    )
    .unwrap();
    let definition = tool.definition();

    let outcome = resolve(tool.execute(invocation(
        &definition,
        json!({"program": "not-started", "arguments": [], "working_directory": "."}),
    )));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("denied command should return a tool error");
    };
    assert_eq!(output.status(), ToolOutputStatus::Error);
    assert!(format!("{:?}", output.content()).contains("denied by policy"));
}

#[test]
fn empty_program_is_rejected_before_execution() {
    let workspace = TestWorkspace::new();
    let tool = ShellCommandTool::new(
        environment_id(),
        workspace.root(),
        DenyAll,
        ShellCommandLimits {
            timeout: Duration::from_secs(1),
            max_output_bytes: 1024,
        },
    )
    .unwrap();
    let definition = tool.definition();

    let outcome = resolve(tool.execute(invocation(
        &definition,
        json!({"program": "", "arguments": [], "working_directory": "."}),
    )));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("empty program should return a tool error");
    };
    assert_eq!(output.status(), ToolOutputStatus::Error);
    assert!(format!("{:?}", output.content()).contains("must not be empty"));
}

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-shell-command-tests-{}-{sequence}",
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
