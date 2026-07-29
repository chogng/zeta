//! Zeta's Agent lifecycle, Turn execution, context assembly, and outbound service ports.

mod capabilities;
mod context;
mod error;
mod policy_service;
mod services;
mod session_coordinator;
mod session_reducer;
mod state;
mod thread_controller;
mod thread_reducer;
mod turn;

pub(crate) use context::ContextAssembler;
pub use error::CoreError;
pub use policy_service::{PolicyService, durable_approval_request};
pub use services::AutoReviewedToolGrant;
pub use services::LeaseGuard;
pub use services::ModelSelection;
pub use services::ModelService;
pub use services::ModelStreamSink;
pub use services::NoThreadUpdates;
pub use services::NoTools;
pub use services::OneTimeToolGrant;
pub use services::ThreadUpdateSink;
pub use services::ToolAuthorization;
pub use services::ToolService;
pub use services::WriterLease;
pub use session_coordinator::{
    ArchiveSessionThreadRequest, CommandDisposition, CreateSessionRequest, CreateSessionResult,
    CreateSessionThreadRequest, ForkSessionThreadRequest, InMemorySessionStore,
    SequenceExpectation, SessionCoordinator, SessionLifecycleRequest, SessionMutationResult,
    SessionThreadResult, SetSessionModelRequest,
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
pub use thread_controller::StartTurnDisposition;
pub use thread_controller::StartTurnRequest;
pub use thread_controller::StartTurnResult;
pub use thread_controller::ThreadController;
pub use thread_controller::ToolCallOutput;
pub use thread_reducer::ResolvedTurnInteraction;
pub use thread_reducer::ThreadCommandResult;
pub use thread_reducer::ThreadCommandSnapshot;
pub use thread_reducer::ThreadSnapshot;
pub use thread_reducer::ToolExecutionStartSnapshot;
pub use thread_reducer::TurnSnapshot;
pub use thread_reducer::reduce_thread_event;
pub use turn::TurnExecutionOutcome;
pub use turn::TurnExecutor;
pub use zeta_protocol::TurnStatus;
pub use zeta_protocol::{
    ProcessExecutionOutput, ProcessExitStatus, SandboxDenialOutput, ToolExecutionOutput,
    ToolReplaySafety,
};
pub use zeta_thread_store::AppendBatchResult;
pub use zeta_thread_store::StoredEvent;
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
pub use capabilities::ElementTarget;
pub use capabilities::GetPdfRequest;
pub use capabilities::PdfResource;
pub use capabilities::TextInputTarget;
pub use capabilities::UnsupportedBrowserCapability;
