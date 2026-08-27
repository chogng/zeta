use std::future;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use zeta_action_policy::ActionDigest;
use zeta_action_policy::ActionKind;
use zeta_action_policy::ActionPolicyRevision;
use zeta_action_policy::ActionProvenance;
use zeta_action_policy::ActionReviewRequest;
use zeta_action_policy::ActionSource;
use zeta_action_policy::CapabilitySet;
use zeta_action_policy::GrantId;
use zeta_action_policy::ResolvedAction;
use zeta_action_policy::SandboxCompatibility;
use zeta_async_utils::CancellationSource;
use zeta_core::CoreError;
use zeta_core::ToolAuthorization;
use zeta_core::ToolOutputSink;
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
    content: Vec<ToolContent>,
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
            ToolOutput::success(self.content.clone()),
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
                ActionPolicyRevision::new("test"),
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
            content: vec![ToolContent::Text("executor-result".into())],
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
        ToolExecutionOutput::SuccessContent(content)
            if content == vec![zeta_protocol::ContentPart::Text("executor-result".into())]
    ));
    assert_eq!(
        sink.values,
        vec![(ToolOutputStream::Stdout, "executor-result".into())]
    );
}

#[test]
fn executor_runtime_preserves_original_image_detail_until_model_capability_gate() {
    let definition = ToolDefinition::function(
        ToolName::new("image-tool").unwrap(),
        "image tool",
        ToolInputSchema::parse(serde_json::json!({"type": "object"})).unwrap(),
        ToolOutputSchema::Unspecified,
        ToolSchemaMode::ProviderDefault,
        ToolLoading::Eager,
    )
    .unwrap();
    let binding = ToolBinding::new(
        ToolRegistryGeneration::new(8),
        ToolBindingId::new("binding-8").unwrap(),
        definition.name().clone(),
        definition.digest(),
        ToolRuntimeKey::new("executor:8").unwrap(),
    );
    let runtime = ToolExecutorRuntime::new(
        Arc::new(RecordingExecutor {
            definition,
            saw_frozen_binding: Arc::new(AtomicBool::new(false)),
            content: vec![ToolContent::Image {
                url: "data:image/png;base64,AA==".into(),
                detail: zeta_tools::ImageDetail::Original,
            }],
        }),
        ToolEnvironmentId::new("workspace-7").unwrap(),
        Arc::new(UnusedReviewer),
    );
    let call = ToolCall {
        id: ToolCallId::new("call-8").unwrap(),
        name: ToolName::new("image-tool").unwrap(),
        arguments: serde_json::json!({}),
    };
    runtime.prepare(&call).unwrap();

    let output = runtime
        .execute_for_turn(
            &binding,
            &call,
            &ToolAuthorization::UnsandboxedGrant {
                grant_id: GrantId::new("test"),
            },
            &CancellationSource::new().token(),
            &TurnId::new("turn-8").unwrap(),
            &mut RecordingSink::default(),
        )
        .unwrap();

    assert!(matches!(
        output,
        ToolExecutionOutput::SuccessContent(content)
            if content == vec![zeta_protocol::ContentPart::ImageUrl {
                url: "data:image/png;base64,AA==".into(),
                detail: zeta_protocol::ImageDetail::Original,
            }]
    ));
}

#[test]
fn executor_output_adapter_truncates_text_before_protocol_and_streaming() {
    let mut sink = RecordingSink::default();
    let output = super::returned_output_with_policy(
        ToolOutput::success(vec![ToolContent::Text("executor output ".repeat(32))]),
        &mut sink,
        zeta_tools::ToolOutputTruncationPolicy::Bytes(128),
    )
    .expect("tool output should adapt");

    let text = match output {
        ToolExecutionOutput::SuccessContent(content) => match &content[..] {
            [zeta_protocol::ContentPart::Text(text)] => text.clone(),
            other => panic!("unexpected content: {other:?}"),
        },
        other => panic!("unexpected output: {other:?}"),
    };
    assert!(text.len() <= 128);
    assert!(text.contains("Warning: truncated output"));
    assert_eq!(sink.values, vec![(ToolOutputStream::Stdout, text.clone())]);
}
