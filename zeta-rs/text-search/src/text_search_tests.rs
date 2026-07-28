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
    ToolPayload, ToolRegistryGeneration, ToolRuntimeAuthority, ToolRuntimeKey,
};

#[test]
fn finds_text_recursively_without_following_binary_content() {
    let workspace = TestWorkspace::new();
    workspace.write("src/lib.rs", "let needle = 1;\n");
    workspace.write("src/other.rs", "let NEEDLE = 2;\n");
    fs::write(workspace.path().join("src/blob.bin"), b"needle\0").unwrap();
    let tool = TextSearchTool::new(
        environment_id(),
        workspace.root(),
        TextSearchLimits::default(),
    )
    .unwrap();
    let definition = tool.definition();
    assert_eq!(definition.name().as_str(), "text-search");

    let outcome = resolve(tool.execute(invocation(
        &definition,
        json!({"query": "needle", "path": "src", "case": "insensitive"}),
    )));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("search should return a tool output");
    };
    assert_eq!(output.status(), ToolOutputStatus::Success);
    let text = format!("{:?}", output.content());
    assert!(text.contains("lib.rs"));
    assert!(text.contains("other.rs"));
    assert!(text.contains("skipped_non_text_files"));
}

#[test]
fn bounds_model_visible_matches() {
    let workspace = TestWorkspace::new();
    workspace.write("src/lib.rs", "needle\nneedle\n");
    let limits = TextSearchLimits::new(1024, 1, 10).unwrap();
    let tool = TextSearchTool::new(environment_id(), workspace.root(), limits).unwrap();
    let definition = tool.definition();

    let outcome = resolve(tool.execute(invocation(
        &definition,
        json!({"query": "needle", "path": "src", "case": "sensitive"}),
    )));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("search should return a tool output");
    };
    let text = format!("{:?}", output.content());
    assert_eq!(text.matches("column_byte").count(), 1);
    assert!(text.contains("truncated"));
}

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-text-search-tests-{}-{sequence}",
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
        ToolExecutionContext::new(
            environment_id(),
            CancellationSource::new().token(),
            ToolRuntimeAuthority::Unrestricted,
        ),
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
