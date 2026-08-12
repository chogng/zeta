use std::sync::Arc;

use super::compose_extension_tools;
use crate::tool_composition::combine_tool_ports;
use serde_json::json;
use zeta_async_utils::CancellationSource;
use zeta_extension_api::ExtensionError;
use zeta_extension_api::ExtensionRegistryBuilder;
use zeta_extension_api::ReadOnlyToolContributor;
use zeta_policy::ExecutionDecision;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolCallId;
use zeta_tools::ToolConcurrency;
use zeta_tools::ToolDefinition;
use zeta_tools::ToolExecutionFuture;
use zeta_tools::ToolExecutionOutcome;
use zeta_tools::ToolExecutor;
use zeta_tools::ToolInputSchema;
use zeta_tools::ToolLoading;
use zeta_tools::ToolName;
use zeta_tools::ToolOutput;
use zeta_tools::ToolOutputSchema;
use zeta_tools::ToolSchemaMode;

struct Contributor;

impl ReadOnlyToolContributor for Contributor {
    fn contribute(&self) -> Result<Vec<Arc<dyn ToolExecutor>>, ExtensionError> {
        Ok(vec![Arc::new(ReadOnlyExtensionTool {
            definition: ToolDefinition::function(
                ToolName::new("extension-read").unwrap(),
                "Read extension-owned metadata.",
                ToolInputSchema::parse(json!({
                    "type": "object",
                    "properties": {"key": {"type": "string"}},
                    "required": ["key"]
                }))
                .unwrap(),
                ToolOutputSchema::Unspecified,
                ToolSchemaMode::Strict,
                ToolLoading::Eager,
            )
            .unwrap(),
        })])
    }
}

struct ReadOnlyExtensionTool {
    definition: ToolDefinition,
}

impl ToolExecutor for ReadOnlyExtensionTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn concurrency(&self) -> ToolConcurrency {
        ToolConcurrency::ParallelSafe
    }

    fn execute(&self, _: zeta_tools::ToolInvocation) -> ToolExecutionFuture<'_> {
        Box::pin(std::future::ready(ToolExecutionOutcome::Returned(
            ToolOutput::success(Vec::new()),
        )))
    }
}

#[test]
fn read_only_extension_contributors_enter_the_shared_registry_and_policy() {
    let mut builder = ExtensionRegistryBuilder::new();
    builder.read_only_tool_contributor(Arc::new(Contributor));
    let registry = builder.build();
    let port = compose_extension_tools(&registry).unwrap().unwrap();
    let combined = combine_tool_ports(vec![port]).unwrap().unwrap();
    assert_eq!(
        combined
            .tools
            .model_definitions(&Default::default())
            .unwrap()
            .iter()
            .map(|definition| definition.name.as_str())
            .collect::<Vec<_>>(),
        vec!["extension-read"]
    );
    let call = ToolCall {
        id: ToolCallId::new("call-1").unwrap(),
        name: zeta_protocol::ToolName::new("extension-read").unwrap(),
        arguments: json!({"key": "value"}),
    };
    let review = combined.tools.prepare(&call).unwrap();
    assert!(matches!(
        combined
            .policy
            .decide(&review, &CancellationSource::new().token()),
        Ok(ExecutionDecision::RunUnsandboxed { .. })
    ));
}
