//! Provider-independent data contracts shared by Zeta runtimes, processes, and adapters.

mod config;
mod error;
mod ids;
mod interaction;
mod item;
mod model;
mod session;
mod stream;
mod thread;
mod tool_execution;
mod tool_name;
mod turn;

pub use config::{ApprovalMode, Patch, Personality, SandboxMode, Theme, WebSearchMode};
pub use error::{StableTurnError, StableTurnErrorCode};
pub use ids::{
    CommandId, InvalidIdentifier, ItemId, RequestId, SessionId, ThreadId, ToolCallId, TurnId,
};
pub use interaction::{
    ActionApprovalCapability, ActionApprovalCapabilityKind, ActionApprovalDecision,
    ActionApprovalRequest, ActionApprovalResponse, AgentInteractionKind, AgentRequest,
    AgentRequestEnvelope, AgentResponse, AgentResponseEnvelope, DynamicToolCall, DynamicToolOutput,
    DynamicToolResponse, DynamicToolSpec, InteractionCancelReason, InteractionDeadline,
    PendingInteraction, RequestUserInput, RequestUserInputResponse, TurnInteraction, UserInput,
    UserInputAnswer, UserInputOption, UserInputQuestion,
};
pub use item::{PlanStep, PlanStepStatus, PlanUpdate, ThreadItem};
pub use model::{
    CapabilitySupport, ContentPart, ContextWindow, ImageDetail, InputItem, InvalidModelIdentity,
    Message, MessageRole, Model, ModelCapabilities, ModelId, ModelInfo, ModelPreset, ModelRef,
    ModelRequest, ModelResponse, ModelStreamEvent, ModelUsage, ProviderId, ReasoningConfig,
    ReasoningEffort, ResponseItem, StopReason, ToolCall, ToolChoice, ToolDefinition, ToolResult,
};
pub use session::{
    Session, SessionCommand, SessionEvent, SessionStatus, SessionThread, SessionThreadStatus,
    SessionUpdate, SessionUpdateEnvelope, ThreadOrigin,
};
pub use stream::{StreamCursor, StreamInstanceId};
pub use thread::{
    ItemDelta, Thread, ThreadCommand, ThreadEvent, ThreadStatus, ThreadUpdate,
    ThreadUpdateEnvelope, ToolExecutionAuthority,
};
pub use tool_execution::{
    ProcessExecutionOutput, ProcessExitStatus, SandboxDenialOutput, ToolExecutionOutput,
    ToolReplaySafety,
};
pub use tool_name::{InvalidToolName, ToolName};
pub use turn::{Turn, TurnStatus};

#[cfg(test)]
#[path = "contract_tests.rs"]
mod contract_tests;
