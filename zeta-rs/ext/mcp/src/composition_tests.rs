use std::collections::BTreeMap;
use std::sync::Arc;

use zeta_async_utils::CancellationSource;
use zeta_config::{
    ConfigGeneration, McpConfig, McpCredentialBinding, McpServerConfig, McpServerEnablement,
    McpServerId, McpTransportConfig, ResolvedConfig,
};
use zeta_connectors::ConnectorAccountId;
use zeta_connectors::ConnectorConnectionGeneration;
use zeta_connectors::ConnectorDefinition;
use zeta_connectors::ConnectorId;
use zeta_connectors::ConnectorRuntimeBinding;
use zeta_connectors_extension::ConnectorApiTokenConnectRequest;
use zeta_connectors_extension::ConnectorAuthority;
use zeta_connectors_extension::ConnectorCommandId;
use zeta_connectors_extension::ConnectorCredentialService;
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
use zeta_secrets::MemorySecretStore;
use zeta_secrets::SecretStore;
use zeta_secrets::SecretValue;

use super::{
    MCP_POLICY_REVISION, McpInvocationAuthority, McpInvocationTransport, compose_mcp_tools,
    start_mcp_tools,
};
use crate::connector::ConnectorMcpRuntimeError;
use crate::connector::ConnectorMcpRuntimeProvider;
use crate::connector::materialize_connector_servers;

struct FakeFactory;

impl McpSessionFactory for FakeFactory {
    fn connect(&self, _: McpServerDefinition, _: RmcpClientOptions) -> McpConnectFuture {
        Box::pin(async { Ok(Box::new(FakeSession) as Box<dyn McpSession>) })
    }
}

struct FakeSession;

struct TokenStdioProvider;

impl ConnectorMcpRuntimeProvider for TokenStdioProvider {
    fn materialize(
        &self,
        _: &ConnectorDefinition,
        credential: SecretValue,
    ) -> Result<McpServerTransport, ConnectorMcpRuntimeError> {
        assert_eq!(credential.expose(), b"secret-token");
        Ok(McpServerTransport::Stdio(StdioServerCommand::new("unused")))
    }
}

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
                connector_fence: None,
            },
        )]),
        ConfigGeneration::INITIAL.get() + 1,
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

#[test]
fn connector_tool_is_materialized_only_while_exact_connection_is_authorized() {
    let connector = ConnectorDefinition::new(
        ConnectorId::new("acme/github:connector:account").unwrap(),
        "GitHub",
        "GitHub tools",
        ConnectorRuntimeBinding::mcp_server("plugin:acme/github:mcp:github").unwrap(),
    )
    .unwrap();
    let connector_id = connector.id().clone();
    let authority = ConnectorAuthority::in_memory([connector]).unwrap();
    let secrets = Arc::new(MemorySecretStore::default());
    let secret_store: Arc<dyn SecretStore> = secrets.clone();
    let service = ConnectorCredentialService::new(authority.clone(), secret_store);
    service
        .connect_api_token(ConnectorApiTokenConnectRequest {
            command_id: ConnectorCommandId::new("connect-github").unwrap(),
            expected_generation: authority.snapshot().generation(),
            connector_id: connector_id.clone(),
            connection_generation: ConnectorConnectionGeneration::new(1),
            account_id: ConnectorAccountId::new("octocat").unwrap(),
            account_display_name: "Octocat".into(),
            token: SecretValue::new(b"secret-token".to_vec()),
        })
        .unwrap();
    let materialized =
        materialize_connector_servers(authority.clone(), secrets.as_ref(), &TokenStdioProvider)
            .unwrap();
    let composition = start_mcp_tools(
        materialized.definitions,
        materialized.authorities,
        1,
        Some(Arc::new(FakeFactory)),
    )
    .unwrap();
    let tool = composition.tools.definitions().remove(0);
    let call = ToolCall {
        id: ToolCallId::new("connector-call").unwrap(),
        name: tool.name,
        arguments: serde_json::json!({}),
    };
    composition.tools.prepare(&call).unwrap();

    let connected_generation = authority.snapshot().generation();
    service
        .disconnect(
            ConnectorCommandId::new("disconnect-github").unwrap(),
            connected_generation,
            connector_id,
        )
        .unwrap();

    assert!(composition.tools.prepare(&call).is_err());
}
