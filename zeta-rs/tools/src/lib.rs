//! Shared host-side tool definitions, validation, and source adapters.
//!
//! This crate intentionally does not schedule tools, own durable Thread state,
//! manage MCP sessions, or perform provider wire encoding. Those responsibilities
//! remain with Core, source runtimes, and provider adapters respectively.

mod binding;
mod definition;
mod dynamic;
mod error;
mod execution;
mod identity;
mod mcp;
mod output;
mod protocol_adapter;
mod schema;

pub use binding::ToolBinding;
pub use definition::{
    FreeformFormat, ToolDefinition, ToolDefinitionDigest, ToolInvocationKind, ToolLoading,
    ToolOutputSchema, ToolSchemaMode,
};
pub use dynamic::from_dynamic_tool_spec;
pub use error::{
    DynamicToolAdapterError, McpToolAdapterError, ProtocolToolAdapterError, ToolDefinitionError,
    ToolSchemaError,
};
pub use execution::{
    ToolConcurrency, ToolConflictClass, ToolExecutionContext, ToolExecutionFuture,
    ToolExecutionOutcome, ToolExecutor, ToolExposure, ToolInvocation, ToolPayload,
    ToolRuntimeAuthority, ToolStartFailure, ToolUncertainOutcome,
};
pub use identity::{
    ToolBindingId, ToolEnvironmentId, ToolIdentityError, ToolOperationId, ToolRegistryGeneration,
    ToolRuntimeKey,
};
pub use mcp::{McpOutputSchemaProjection, McpToolProjection, from_mcp_tool_projection};
pub use output::{ToolContent, ToolOutput, ToolOutputStatus};
pub use protocol_adapter::{to_protocol_tool_definition, to_protocol_tool_result};
pub use schema::{ToolInputSchema, ToolSchema, ToolSchemaDigest};
pub use zeta_protocol::{ImageDetail, ToolCallId, ToolName};
pub use zeta_protocol::{
    ProcessExecutionOutput, ProcessExitStatus, SandboxDenialOutput, ToolReplaySafety,
};
