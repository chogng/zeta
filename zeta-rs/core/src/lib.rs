//! Zeta's Agent lifecycle, Turn execution, context assembly, and outbound service ports.

mod action_policy_service;
mod attachment_model_service;
mod capabilities;
mod context;
mod context_manager;
mod error;
mod hooks;
mod image_preparation;
mod multi_agent;
mod services;
mod session_coordinator;
mod session_reducer;
mod state;
#[cfg(test)]
mod test_image;
mod thread_controller;
mod thread_reducer;
mod tool_profile;
mod tool_repetition;
mod turn;

pub use action_policy_service::{ActionPolicyService, durable_approval_request};
pub(crate) use context::ContextAssembler;
pub use context::ContextBudget;
pub use context::ContextCompactionLimit;
pub use context::ContextCompactionRequest;
pub use context::ContextCompactionResult;
pub use context::ContextCompactionService;
pub use context::ContextTokenCount;
pub use context::ContextTokenMeasurementCapability;
pub use context::ContextTokenMeasurementOutcome;
pub use context::HarnessContext;
pub use context::HarnessContextProvider;
pub use context::HarnessContextRequest;
pub use context::HarnessInstructions;
pub use context::ResolvedContextBudget;
pub use error::CoreError;
pub use hooks::AfterToolHookRequest;
pub use hooks::BeforeToolHookDecision;
pub use hooks::BeforeToolHookRequest;
pub use hooks::HookOutcome;
pub use hooks::HookService;
pub use hooks::NoHooks;
pub use hooks::TurnCompletedHookRequest;
pub use multi_agent::AgentTreeLimits;
pub use multi_agent::CompleteDelegationRequest;
pub use multi_agent::DeliveredAgentMessage;
pub use multi_agent::JoinAgentsRequest;
pub use multi_agent::JoinedAgents;
pub use multi_agent::MultiAgentCoordinator;
pub use multi_agent::SendAgentMessageRequest;
pub use multi_agent::SpawnAgentRequest;
pub use multi_agent::SpawnedAgent;
pub use multi_agent::project_agent_tree;
pub use services::AutoReviewedToolGrant;
pub use services::ContextEvidence;
pub use services::ContextSource;
pub use services::ContextSourceRequest;
pub use services::ExecPolicyToolGrant;
pub use services::LeaseGuard;
pub use services::ModelImageInputLimits;
pub use services::ModelImageInputPolicy;
pub use services::ModelSelection;
pub use services::ModelService;
pub use services::ModelStreamSink;
pub use services::ModelToolCatalogSnapshot;
pub use services::NoContextSource;
pub use services::NoThreadUpdates;
pub use services::NoTools;
pub use services::OneTimeToolGrant;
pub use services::PermissionBypassToolGrant;
pub use services::ThreadUpdateSink;
pub use services::ToolAuthorization;
pub use services::ToolExecutionFacts;
pub use services::ToolExecutionIdentity;
pub use services::ToolInteractionService;
pub use services::ToolOutputSink;
pub use services::ToolService;
pub use services::ToolUserInputOutcome;
pub use services::WriterLease;
pub use session_coordinator::{
    CommandDisposition, CreateSessionRequest, CreateSessionResult, CreateSessionThreadRequest,
    ForkSessionThreadRequest, InMemorySessionStore, RewindSessionThreadRequest,
    RewriteSessionThreadRequest, SequenceExpectation, SessionCoordinator, SessionLifecycleRequest,
    SessionMutationResult, SessionThreadResult, SetSessionCurrentThreadRequest,
    SetSessionModelRequest, SetSessionNextApprovalModeRequest, SpawnAgentThreadRequest,
    StartSessionShellTurnRequest, StartSessionTurnRequest,
};
pub use session_reducer::{
    SessionCommandResult, SessionCommandSnapshot, SessionSnapshot, SessionThreadSnapshot,
    reduce_session_event,
};
pub use state::ItemStatus;
pub use state::ToolCallStatus;
pub use thread_controller::CancelTurnInteractionRequest;
pub use thread_controller::CancelledTurnInteraction;
pub use thread_controller::CompletedTurn;
pub use thread_controller::CreateAgentThreadRequest;
pub use thread_controller::CreateForkedThreadRequest;
pub use thread_controller::CreateRewoundThreadRequest;
pub use thread_controller::CreateThreadRequest;
pub use thread_controller::InMemoryThreadStore;
pub use thread_controller::InterruptTurnDisposition;
pub use thread_controller::InterruptTurnRequest;
pub use thread_controller::InterruptTurnResult;
pub use thread_controller::RecordToolCallRequest;
pub use thread_controller::RecordToolResultRequest;
pub use thread_controller::RecordedToolCall;
pub use thread_controller::RecordedToolResult;
pub use thread_controller::RequestTurnInteraction;
pub use thread_controller::RequestedTurnInteraction;
pub use thread_controller::ResolveTurnInteractionDisposition;
pub use thread_controller::ResolveTurnInteractionRequest;
pub use thread_controller::ResolveTurnInteractionResult;
pub use thread_controller::SetGoalRequest;
pub use thread_controller::SetGoalResult;
pub use thread_controller::StartContextCompactionRequest;
pub use thread_controller::StartGoalTurnRequest;
pub use thread_controller::StartTurnDisposition;
pub use thread_controller::StartTurnResult;
pub use thread_controller::SteerTurnDisposition;
pub use thread_controller::SteerTurnRequest;
pub use thread_controller::SteerTurnResult;
pub use thread_controller::ThreadController;
pub use thread_controller::ThreadExecutionContext;
pub use thread_controller::ToolCallOutput;
pub use thread_controller::UpdatePlanDisposition;
pub use thread_controller::UpdatePlanResult;
pub use thread_controller::{ShellTurnInvocation, StartShellTurnRequest, StartTurnRequest};
pub use thread_reducer::DelegationSnapshot;
pub use thread_reducer::ResolvedTurnInteraction;
pub use thread_reducer::ThreadCommandResult;
pub use thread_reducer::ThreadCommandSnapshot;
pub use thread_reducer::ThreadSnapshot;
pub use thread_reducer::ToolExecutionStartSnapshot;
pub use thread_reducer::TurnSnapshot;
pub use thread_reducer::reduce_thread_event;
pub use turn::TurnExecutionBackend;
pub use turn::TurnExecutionOutcome;
pub use turn::TurnExecutor;
pub use zeta_protocol::TurnStatus;
pub use zeta_protocol::{
    ProcessExecutionOutput, ProcessExitStatus, SandboxDenialOutput, ToolExecutionOutput,
    ToolReplaySafety,
};
pub use zeta_thread_store::AppendBatchResult;
pub use zeta_thread_store::ThreadEventBatch;
pub use zeta_thread_store::ThreadStore;
pub use zeta_thread_store::ThreadStoreError;
pub use zeta_thread_store::validate_append_batch;

#[cfg(test)]
#[path = "thread_controller_tests.rs"]
mod tests;
pub use capabilities::BrowserAction;
pub use capabilities::BrowserActionResult;
pub use capabilities::BrowserCapability;
pub use capabilities::BrowserError;
pub use capabilities::BrowserObservation;
pub use capabilities::BrowserObserveRequest;
pub use capabilities::BrowserTargetId;
pub use capabilities::CreateBrowserTargetRequest;
pub use capabilities::CreateBrowserTargetResult;
pub use capabilities::ElementTarget;
pub use capabilities::MediaResource;
pub use capabilities::TextInputTarget;
pub use capabilities::UnsupportedBrowserCapability;
