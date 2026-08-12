use std::collections::BTreeMap;
use std::sync::Arc;

use zeta_async_utils::CancellationSource;
use zeta_config::{
    ConfigGeneration, McpConfig, McpCredentialBinding, McpServerConfig, McpServerEnablement,
    McpServerId, McpTransportConfig, ResolvedConfig,
};
use zeta_mcp::{
    McpConnectFuture, McpPageCursor, McpServerDefinition, McpServerTransport, McpSession,
    McpSessionError, McpSessionFactory, McpSessionFuture,
};
use zeta_policy::{
    ActionDigest, ActionKind, ActionProvenance, ActionReviewRequest, ActionSource, CapabilitySet,
    ExecutionDecision, PolicyRevision, ResolvedAction, SandboxCompatibility,
};
use zeta_protocol::{ToolCall, ToolCallId};
use zeta_rmcp_client::{
    CallToolRequestParams, CallToolResult, JsonObject, ListToolsResult, RmcpClientOptions,
    ServerInfo, StdioServerCommand, Tool,
};

use super::{
    MCP_POLICY_REVISION, McpInvocationAuthority, McpInvocationTransport, compose_mcp_tools,
    start_mcp_tools,
};

struct FakeFactory;

impl McpSessionFactory for FakeFactory {
    fn connect(&self, _: McpServerDefinition, _: RmcpClientOptions) -> McpConnectFuture {
        Box::pin(async { Ok(Box::new(FakeSession) as Box<dyn McpSession>) })
    }
}

struct FakeSession;

impl McpSession for FakeSession {
    fn server_info(&self) -> ServerInfo {
        ServerInfo::default()
    }

    fn list_tools(&self, cursor: McpPageCursor) -> McpSessionFuture<'_, ListToolsResult> {
        Box::pin(async move {
            if cursor != McpPageCursor::First {
                return Err(McpSessionError::Transport("unexpected cursor".into()));
            }
            let mut schema = JsonObject::new();
            schema.insert("type".into(), serde_json::Value::String("object".into()));
            Ok(ListToolsResult {
                tools: vec![Tool::new("write", "Write remotely", Arc::new(schema))],
                ..Default::default()
            })
        })
    }

    fn call_tool(
        &self,
        _: CallToolRequestParams,
        _: &zeta_async_utils::CancellationToken,
    ) -> McpSessionFuture<'_, CallToolResult> {
        Box::pin(async { Ok(CallToolResult::success(Vec::new())) })
    }

    fn shutdown(self: Box<Self>) -> McpSessionFuture<'static, ()> {
        Box::pin(async { Ok(()) })
    }
}

#[test]
fn prepares_exact_mcp_provenance_and_requires_user_approval() {
    let server = McpServerId::new("user:mcp:test").unwrap();
    let definition = McpServerDefinition::new(
        server.clone(),
        "Test",
        McpServerTransport::Stdio(StdioServerCommand::new("unused")),
    )
    .unwrap();
    let composition = start_mcp_tools(
        vec![definition],
        BTreeMap::from([(
            server.clone(),
            McpInvocationAuthority {
                display_name: "Test".into(),
                transport: McpInvocationTransport::Stdio {
                    executable: "/test/mcp".into(),
                },
            },
        )]),
        ConfigGeneration::INITIAL,
        Some(Arc::new(FakeFactory)),
    )
    .expect("compose MCP tools");
    let definition = composition.tools.definitions().remove(0);
    let call = ToolCall {
        id: ToolCallId::new("call-1").unwrap(),
        name: definition.name,
        arguments: serde_json::json!({"value": 1}),
    };

    let review = composition.tools.prepare(&call).expect("prepare");
    assert_eq!(review.provenance().source(), &ActionSource::McpServer);
    assert_eq!(review.provenance().source_id(), server.as_str());
    assert!(matches!(
        composition
            .policy
            .decide(&review, &CancellationSource::new().token())
            .expect("policy"),
        ExecutionDecision::AskUser(_)
    ));

    let weaker = ActionReviewRequest::new(
        ResolvedAction::new(
            ActionDigest::from_canonical_bytes(b"weaker"),
            ActionKind::ExternalServiceMutation,
            "weaker",
            CapabilitySet::default(),
        ),
        ActionProvenance::new(ActionSource::McpServer, server.as_str()),
        SandboxCompatibility::NotApplicable {
            reason: "test".into(),
        },
        PolicyRevision::new(MCP_POLICY_REVISION),
    );
    assert!(
        composition
            .policy
            .decide(&weaker, &CancellationSource::new().token())
            .is_err()
    );
}

#[test]
fn rejects_relative_stdio_executable_before_starting_runtime() {
    let server = McpServerId::new("user:mcp:test").unwrap();
    let config = ResolvedConfig {
        mcp: McpConfig {
            servers: BTreeMap::from([(
                server.clone(),
                McpServerConfig {
                    id: server,
                    display_name: "Test".into(),
                    transport: McpTransportConfig::Stdio {
                        command: "npx".into(),
                        args: Vec::new(),
                    },
                    credential: McpCredentialBinding::Unauthenticated,
                    enablement: McpServerEnablement::Enabled,
                },
            )]),
        },
        ..ResolvedConfig::default()
    };

    let error = match compose_mcp_tools(&config, ConfigGeneration::INITIAL) {
        Ok(_) => panic!("relative executable must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("absolute executable path"));
}
