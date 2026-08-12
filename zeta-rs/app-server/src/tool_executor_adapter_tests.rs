use std::future;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use zeta_async_utils::CancellationSource;
use zeta_core::CoreError;
use zeta_core::ToolAuthorization;
use zeta_core::ToolOutputSink;
use zeta_policy::ActionDigest;
use zeta_policy::ActionKind;
use zeta_policy::ActionProvenance;
use zeta_policy::ActionReviewRequest;
use zeta_policy::ActionSource;
use zeta_policy::CapabilitySet;
use zeta_policy::GrantId;
use zeta_policy::PolicyRevision;
use zeta_policy::ResolvedAction;
use zeta_policy::SandboxCompatibility;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolExecutionOutput;
use zeta_protocol::ToolName;
use zeta_protocol::ToolOutputStream;
use zeta_protocol::TurnId;
use zeta_tools::ToolBinding;
use zeta_tools::ToolBindingId;
use zeta_tools::ToolContent;
use zeta_tools::ToolDefinition;
use zeta_tools::ToolEnvironmentId;
use zeta_tools::ToolExecutionFuture;
use zeta_tools::ToolExecutionOutcome;
use zeta_tools::ToolExecutor;
use zeta_tools::ToolInputSchema;
use zeta_tools::ToolLoading;
use zeta_tools::ToolOutput;
use zeta_tools::ToolOutputSchema;
use zeta_tools::ToolPayload;
use zeta_tools::ToolRegistryGeneration;
use zeta_tools::ToolRuntimeKey;
use zeta_tools::ToolSchemaMode;

use super::PreparedToolExecution;
use super::ToolExecutorReviewer;
use super::ToolExecutorRuntime;

struct RecordingExecutor {
    definition: ToolDefinition,
    saw_frozen_binding: Arc<AtomicBool>,
}

impl ToolExecutor for RecordingExecutor {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn execute(&self, invocation: zeta_tools::ToolInvocation) -> ToolExecutionFuture<'_> {
        self.saw_frozen_binding.store(
            invocation.binding().id().as_str() == "binding-7"
                && invocation.binding().registry_generation() == ToolRegistryGeneration::new(7)
                && invocation.context().environment_id().as_str() == "workspace-7",
            Ordering::SeqCst,
        );
        Box::pin(future::ready(ToolExecutionOutcome::Returned(
            ToolOutput::success(vec![ToolContent::Text("executor-result".into())]),
        )))
    }
}

struct UnusedReviewer;

impl ToolExecutorReviewer for UnusedReviewer {
    fn prepare(&self, call: &ToolCall) -> Result<PreparedToolExecution, CoreError> {
        Ok(PreparedToolExecution::new(
            ActionReviewRequest::new(
                ResolvedAction::new(
                    ActionDigest::from_canonical_bytes(b"executor-test"),
                    ActionKind::SystemOperation,
                    "executor test",
                    CapabilitySet::new([]),
                ),
                ActionProvenance::new(ActionSource::BuiltInTool, "executor-test"),
                SandboxCompatibility::NotApplicable {
                    reason: "test".into(),
                },
                PolicyRevision::new("test"),
            ),
            ToolPayload::FunctionArguments(call.arguments.clone()),
        ))
    }
}

#[derive(Default)]
struct RecordingSink {
    values: Vec<(ToolOutputStream, String)>,
}

impl ToolOutputSink for RecordingSink {
    fn emit(&mut self, stream: ToolOutputStream, text: String) -> Result<(), CoreError> {
        self.values.push((stream, text));
        Ok(())
    }
}

#[test]
fn executor_runtime_preserves_registry_binding_environment_and_output() {
    let definition = ToolDefinition::function(
        ToolName::new("executor-tool").unwrap(),
        "executor tool",
        ToolInputSchema::parse(serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }))
        .unwrap(),
        ToolOutputSchema::Unspecified,
        ToolSchemaMode::ProviderDefault,
        ToolLoading::Eager,
    )
    .unwrap();
    let binding = ToolBinding::new(
        ToolRegistryGeneration::new(7),
        ToolBindingId::new("binding-7").unwrap(),
        definition.name().clone(),
        definition.digest(),
        ToolRuntimeKey::new("executor:7").unwrap(),
    );
    let observed = Arc::new(AtomicBool::new(false));
    let runtime = ToolExecutorRuntime::new(
        Arc::new(RecordingExecutor {
            definition,
            saw_frozen_binding: Arc::clone(&observed),
        }),
        ToolEnvironmentId::new("workspace-7").unwrap(),
        Arc::new(UnusedReviewer),
    );
    let call = ToolCall {
        id: ToolCallId::new("call-7").unwrap(),
        name: ToolName::new("executor-tool").unwrap(),
        arguments: serde_json::json!({}),
    };
    let mut sink = RecordingSink::default();
    runtime.prepare(&call).unwrap();

    let output = runtime
        .execute_for_turn(
            &binding,
            &call,
            &ToolAuthorization::UnsandboxedGrant {
                grant_id: GrantId::new("test"),
            },
            &CancellationSource::new().token(),
            &TurnId::new("turn-7").unwrap(),
            &mut sink,
        )
        .unwrap();

    assert!(observed.load(Ordering::SeqCst));
    assert!(matches!(
        output,
        ToolExecutionOutput::Success(text) if text.contains("executor-result")
    ));
    assert_eq!(
        sink.values,
        vec![(ToolOutputStream::Stdout, "executor-result".into())]
    );
}
