use super::{
    ToolConcurrency, ToolExecutionContext, ToolExecutionFuture, ToolExecutionOutcome, ToolExecutor,
    ToolInvocation, ToolPayload,
};
use crate::{
    ToolBinding, ToolBindingId, ToolDefinition, ToolEnvironmentId, ToolInputSchema, ToolLoading,
    ToolName, ToolOperationId, ToolOutput, ToolOutputSchema, ToolRegistryGeneration,
    ToolRuntimeKey, ToolSchemaMode,
};
use serde_json::json;
use zeta_async_utils::CancellationSource;
use zeta_protocol::{ToolCallId, TurnId};

#[test]
fn invocation_keeps_the_frozen_binding_and_environment() {
    let definition = definition();
    let binding = ToolBinding::new(
        ToolRegistryGeneration::new(7),
        ToolBindingId::new("binding_1").expect("valid binding"),
        ToolName::new("search").expect("valid name"),
        definition.digest(),
        ToolRuntimeKey::new("builtin:search").expect("valid runtime"),
    );
    let invocation = ToolInvocation::new(
        ToolOperationId::new("operation_1").expect("valid operation"),
        ToolCallId::new("call_1").expect("valid call"),
        TurnId::new("turn_1").expect("valid turn"),
        binding,
        ToolPayload::FunctionArguments(json!({"query": "zeta"})),
        ToolExecutionContext::new(
            ToolEnvironmentId::new("workspace_1").expect("valid environment"),
            CancellationSource::new().token(),
        ),
    );

    assert_eq!(
        invocation.binding().runtime_key().as_str(),
        "builtin:search"
    );
    assert_eq!(invocation.binding().registry_generation().get(), 7);
    assert_eq!(
        invocation.context().environment_id().as_str(),
        "workspace_1"
    );
}

#[test]
fn executor_defaults_to_exclusive_direct_exposure() {
    let executor = TestExecutor {
        definition: definition(),
    };

    assert_eq!(executor.concurrency(), ToolConcurrency::Exclusive);
    assert_eq!(executor.exposure(), crate::ToolExposure::Direct);
}

struct TestExecutor {
    definition: ToolDefinition,
}

impl ToolExecutor for TestExecutor {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn execute(&self, _invocation: ToolInvocation) -> ToolExecutionFuture<'_> {
        Box::pin(std::future::ready(ToolExecutionOutcome::Returned(
            ToolOutput::success(Vec::new()),
        )))
    }
}

fn definition() -> ToolDefinition {
    ToolDefinition::function(
        ToolName::new("search").expect("valid tool name"),
        "Search documents.",
        ToolInputSchema::parse(json!({"type": "object", "properties": {}})).expect("valid schema"),
        ToolOutputSchema::Unspecified,
        ToolSchemaMode::ProviderDefault,
        ToolLoading::Eager,
    )
    .expect("valid definition")
}
