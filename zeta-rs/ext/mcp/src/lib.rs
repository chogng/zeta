//! MCP configuration projection, live session ownership, and agent-tool policy integration.

mod composition;
mod connector;
mod plugin;
mod runtime;
mod updates;

pub use composition::McpToolComposition;
pub use composition::McpToolCompositionError;
pub use composition::compose_mcp_tools;
pub use composition::compose_mcp_tools_at_generation_with_updates;
pub use composition::compose_mcp_tools_with_connectors;
pub use composition::compose_mcp_tools_with_connectors_and_updates;
pub use composition::compose_mcp_tools_with_updates;
pub use connector::ConnectorMcpRuntimeError;
pub use connector::ConnectorMcpRuntimeProvider;
pub use connector::RuntimeInvocationFence;
pub use connector::RuntimeInvocationLease;
pub use connector::StandaloneMcpServer;
pub use plugin::PluginConnectorMcpRuntimeProvider;
pub use updates::McpCatalogUpdateSubscription;
pub use updates::McpCatalogUpdates;
