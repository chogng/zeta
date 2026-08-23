//! Provider-independent data contracts shared by Zeta runtimes, processes, and adapters.

mod attachment;
mod config;
mod error;
mod ids;
mod interaction;
mod item;
mod model;
mod multi_agent;
mod session;
mod skill;
mod stream;
mod thread;
mod tool_binding;
mod tool_execution;
mod tool_name;
mod turn;
mod turn_execution;

pub use attachment::{ImageAttachmentRef, ImageMediaType};
pub use config::{ApprovalMode, Patch, Personality, SandboxMode, Theme, WebSearchMode};
pub use error::{StableTurnError, StableTurnErrorCode};
pub use ids::{
    AgentJoinId, AgentMessageId, CommandId, ContextCheckpointId, DelegationId, InvalidIdentifier,
    ItemId, RequestId, SessionId, ThreadId, ToolCallId, TurnId,
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
    CapabilitySupport, ContentPart, ContextWindow, ImageDetail, ImageDetailDecision,
    ImageDetailDecisionReason, InputItem, InvalidModelIdentity, Message, MessageRole, Model,
    ModelAccess, ModelAvailability, ModelCapabilities, ModelCatalogFreshness, ModelId, ModelInfo,
    ModelInputEstimate, ModelLifecycle, ModelMetadataQuality, ModelOutputTransport, ModelPreset,
    ModelRef, ModelRequest, ModelResponse, ModelStreamEvent, ModelUsage, ModelUsageSummary,
    ModelUsageTotal, ProviderId, ReasoningConfig, ReasoningEffort, ResponseItem, StopReason,
    ToolCall, ToolChoice, ToolDefinition, ToolResult,
};
pub use multi_agent::{
    AgentContextContent, AgentContextMode, AgentContextSeed, AgentContextSource,
    AgentDefinitionSelectionReason, AgentJoin, AgentJoinPolicy, AgentJoinStatus,
    AgentMaterializedContext, AgentMessage, AgentMessageContent, AgentMessageProvenance,
    AgentRoleSnapshot, AgentTreeExecutionStatus, AgentTreeNodeProjection, AgentTreeProjection,
    AgentTreeWaitingReason, ContextSeedDigest, DelegatedCapabilityScope, DelegatedPolicyCeiling,
    DelegatedTask, DelegationArtifactRef, DelegationResult, DelegationResultDigest,
    DelegationResultStatus, ForkedAgentContext, FrozenAgentDefinitionRef, InvalidContextSeedDigest,
    InvalidDelegationResultDigest, ThreadSequenceRange,
};
pub use session::{
    Session, SessionCommand, SessionEvent, SessionStatus, SessionThread, SessionThreadStatus,
    SessionUpdate, SessionUpdateEnvelope, ThreadOrigin,
};
pub use skill::{
    ContentDigest, FrozenSkillActivation, InvalidContentDigest, InvalidSkillName,
    InvalidSkillSourceId, SkillActivationReason, SkillId, SkillName, SkillRef, SkillSourceId,
    SkillVersionSelector,
};
pub use stream::{StreamCursor, StreamInstanceId};
pub use thread::{
    ContextCheckpoint, ContextCheckpointVerification, ContextSourceDigest, ContextSourceRange,
    InvalidContextSourceDigest, ItemDelta, Thread, ThreadCommand, ThreadEvent, ThreadStatus,
    ThreadUpdate, ThreadUpdateEnvelope, ToolExecutionAuthority, ToolOutputStream,
};
pub use tool_binding::{ToolCallBinding, ToolCallCaller, ToolSourceProvenance};
pub use tool_execution::{
    ProcessExecutionOutput, ProcessExitStatus, SandboxDenialOutput, ToolExecutionOutput,
    ToolReplaySafety,
};
pub use tool_name::{InvalidToolName, ToolName};
pub use turn::{ModelPriceSnapshot, ToolProfileSnapshot, Turn, TurnResourceBudget, TurnStatus};
pub use turn_execution::TurnExecutionBinding;
pub use zeta_workspace::WorkspaceBinding;
pub use zeta_workspace::WorkspaceTrustId;

#[cfg(test)]
#[path = "contract_tests.rs"]
mod contract_tests;
