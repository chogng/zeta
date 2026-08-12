use std::sync::Arc;

use zeta_async_utils::CancellationSource;
use zeta_config::McpServerId;
use zeta_mcp::{
    McpConnectFuture, McpPageCursor, McpRuntimeOptions, McpServerDefinition, McpServerTransport,
    McpSession, McpSessionError, McpSessionFactory, McpSessionFuture,
};
use zeta_rmcp_client::{
    CallToolRequestParams, CallToolResult, ContentBlock, JsonObject, ListToolsResult,
    RmcpClientOptions, ServerInfo, StdioServerCommand, Tool,
};
use zeta_tools::{ToolContent, ToolOutputStatus};

use super::McpRuntimeOwner;

struct FakeFactory;

impl McpSessionFactory for FakeFactory {
    fn connect(&self, definition: McpServerDefinition, _: RmcpClientOptions) -> McpConnectFuture {
        let server = definition.id().clone();
        Box::pin(async move { Ok(Box::new(FakeSession(server)) as Box<dyn McpSession>) })
    }
}

struct FakeSession(McpServerId);

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
                tools: vec![Tool::new("echo", "Echo server identity", Arc::new(schema))],
                ..Default::default()
            })
        })
    }

    fn call_tool(
        &self,
        _: CallToolRequestParams,
        _: &zeta_async_utils::CancellationToken,
    ) -> McpSessionFuture<'_, CallToolResult> {
        Box::pin(async move {
            Ok(CallToolResult::success(vec![ContentBlock::text(
                self.0.to_string(),
            )]))
        })
    }

    fn shutdown(self: Box<Self>) -> McpSessionFuture<'static, ()> {
        Box::pin(async { Ok(()) })
    }
}

#[test]
fn bridges_sync_calls_to_continuously_running_async_runtime() {
    let server = McpServerId::new("user:mcp:test").unwrap();
    let definition = McpServerDefinition::new(
        server.clone(),
        "Test",
        McpServerTransport::Stdio(StdioServerCommand::new("unused")),
    )
    .unwrap();
    let owner = McpRuntimeOwner::start_with_factory(
        vec![definition],
        McpRuntimeOptions::new("app-server-test", "0"),
        Arc::new(FakeFactory),
    )
    .expect("start owner");

    assert_eq!(owner.definitions().len(), 1);
    let binding = owner
        .resolve(&owner.definitions()[0].name)
        .expect("binding")
        .clone();
    let output = owner
        .call(
            binding,
            serde_json::json!({}),
            CancellationSource::new().token(),
        )
        .expect("call tool");

    assert_eq!(output.status(), ToolOutputStatus::Success);
    assert_eq!(output.content(), &[ToolContent::Text(server.to_string())]);
}
