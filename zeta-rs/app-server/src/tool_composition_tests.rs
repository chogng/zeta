use std::sync::Arc;

use zeta_async_utils::CancellationToken;
use zeta_core::{CoreError, PolicyService, ToolAuthorization, ToolService};
use zeta_policy::{
    ActionDigest, ActionKind, ActionProvenance, ActionReviewRequest, ActionSource, ApprovalRequest,
    Capability, CapabilityKind, CapabilitySet, ExecutionDecision, PolicyRevision, ResolvedAction,
    SandboxCompatibility,
};
use zeta_protocol::{ToolCall, ToolDefinition, ToolExecutionOutput, ToolName};

use super::{ToolPort, combine_tool_ports};

struct FakeTools {
    definition: ToolDefinition,
    source: ActionSource,
    source_id: &'static str,
}

impl FakeTools {
    fn new(name: &str, source: ActionSource, source_id: &'static str) -> Self {
        Self {
            definition: ToolDefinition {
                name: ToolName::new(name).unwrap(),
                description: name.into(),
                parameters: serde_json::json!({"type": "object"}),
                strict: false,
            },
            source,
            source_id,
        }
    }
}

impl ToolService for FakeTools {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![self.definition.clone()]
    }

    fn prepare(&self, _: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(self.source_id),
                ActionKind::ExternalServiceMutation,
                self.source_id,
                CapabilitySet::new([Capability::new(
                    CapabilityKind::ExternalMutation,
                    self.source_id,
                )]),
            ),
            ActionProvenance::new(self.source.clone(), self.source_id),
            SandboxCompatibility::NotApplicable {
                reason: "test".into(),
            },
            PolicyRevision::new("test"),
        ))
    }

    fn execute(
        &self,
        _: &ToolCall,
        _: &ToolAuthorization,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        Ok(ToolExecutionOutput::Success(self.source_id.into()))
    }
}

struct AskPolicy;

impl PolicyService for AskPolicy {
    fn decide(
        &self,
        request: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        Ok(ExecutionDecision::AskUser(ApprovalRequest::new(
            request.action().digest().clone(),
            request.action().required_capabilities().clone(),
            "test",
        )))
    }
}

#[test]
fn routes_tool_and_policy_by_frozen_definition_and_provenance() {
    let combined = combine_tool_ports(vec![
        ToolPort::local(
            Arc::new(FakeTools::new(
                "local_tool",
                ActionSource::BuiltInTool,
                "local",
            )),
            Arc::new(AskPolicy),
        ),
        ToolPort::mcp(
            Arc::new(FakeTools::new(
                "mcp_tool",
                ActionSource::McpServer,
                "user:mcp:test",
            )),
            Arc::new(AskPolicy),
        ),
    ])
    .unwrap()
    .unwrap();
    let call = ToolCall {
        id: zeta_protocol::ToolCallId::new("call-1").unwrap(),
        name: ToolName::new("mcp_tool").unwrap(),
        arguments: serde_json::json!({}),
    };

    let review = combined.tools.prepare(&call).expect("route tool");
    assert_eq!(review.provenance().source(), &ActionSource::McpServer);
    assert!(matches!(
        combined
            .policy
            .decide(
                &review,
                &zeta_async_utils::CancellationSource::new().token()
            )
            .expect("route policy"),
        ExecutionDecision::AskUser(_)
    ));
}

#[test]
fn rejects_duplicate_model_tool_names() {
    let result = combine_tool_ports(vec![
        ToolPort::local(
            Arc::new(FakeTools::new(
                "duplicate",
                ActionSource::BuiltInTool,
                "local",
            )),
            Arc::new(AskPolicy),
        ),
        ToolPort::mcp(
            Arc::new(FakeTools::new(
                "duplicate",
                ActionSource::McpServer,
                "user:mcp:test",
            )),
            Arc::new(AskPolicy),
        ),
    ]);

    assert!(result.is_err());
}
