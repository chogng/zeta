//! MCP configuration projection, live session ownership, and agent-tool policy integration.

mod composition;
mod connector;
mod runtime;

pub use composition::McpToolComposition;
pub use composition::McpToolCompositionError;
pub use composition::compose_mcp_tools;
pub use composition::compose_mcp_tools_with_connectors;
pub use connector::ConnectorMcpRuntimeError;
pub use connector::ConnectorMcpRuntimeProvider;
