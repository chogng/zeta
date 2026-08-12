//! MCP configuration projection, live session ownership, and agent-tool policy integration.

mod composition;
mod runtime;

pub use composition::McpToolComposition;
pub use composition::McpToolCompositionError;
pub use composition::compose_mcp_tools;
