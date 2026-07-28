use crate::{
    AgentResponse, InteractionCancelReason, RequestId, SessionId, StableTurnError, ThreadId,
    ThreadItem, ToolCallId, TurnId, TurnInteraction,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Durable authority selected before a Tool Call crosses its side-effect boundary.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ToolExecutionAuthority {
    Sandboxed,
    UnsandboxedGrant { grant_id: String },
    AutoReviewed { assessment_id: String },
    ApprovedOnce { request_id: RequestId },
}

/// A durable fact in one Thread's authoritative event stream.
///
/// This enum deliberately excludes token deltas, delivery notifications, and client requests so
/// persistence adapters cannot accidentally append transient runtime messages.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ThreadEvent {
    ThreadCreated {
        session_id: SessionId,
        thread_id: ThreadId,
        title: String,
    },
    TurnAccepted {
        thread_id: ThreadId,
        turn_id: TurnId,
    },
    TurnStarted {
        thread_id: ThreadId,
        turn_id: TurnId,
    },
    ItemCompleted {
        thread_id: ThreadId,
        turn_id: TurnId,
        item: ThreadItem,
    },
    InteractionRequested {
        thread_id: ThreadId,
        turn_id: TurnId,
        interaction: TurnInteraction,
    },
    InteractionResolved {
        thread_id: ThreadId,
        turn_id: TurnId,
        request_id: RequestId,
        response: AgentResponse,
    },
    ToolExecutionStarted {
        thread_id: ThreadId,
        turn_id: TurnId,
        tool_call_id: ToolCallId,
        action_digest: String,
        policy_revision: String,
        authority: ToolExecutionAuthority,
    },
    InteractionCancelled {
        thread_id: ThreadId,
        turn_id: TurnId,
        request_id: RequestId,
        reason: InteractionCancelReason,
    },
    TurnCompleted {
        thread_id: ThreadId,
        turn_id: TurnId,
    },
    TurnFailed {
        thread_id: ThreadId,
        turn_id: TurnId,
        error: StableTurnError,
    },
    TurnCancelling {
        thread_id: ThreadId,
        turn_id: TurnId,
    },
    TurnInterrupted {
        thread_id: ThreadId,
        turn_id: TurnId,
    },
}

impl ThreadEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::ThreadCreated { .. } => "thread.created",
            Self::TurnAccepted { .. } => "turn.accepted",
            Self::TurnStarted { .. } => "turn.started",
            Self::ItemCompleted { .. } => "item.completed",
            Self::InteractionRequested { .. } => "interaction.requested",
            Self::InteractionResolved { .. } => "interaction.resolved",
            Self::ToolExecutionStarted { .. } => "tool.execution_started",
            Self::InteractionCancelled { .. } => "interaction.cancelled",
            Self::TurnCompleted { .. } => "turn.completed",
            Self::TurnFailed { .. } => "turn.failed",
            Self::TurnCancelling { .. } => "turn.cancelling",
            Self::TurnInterrupted { .. } => "turn.interrupted",
        }
    }

    pub fn thread_id(&self) -> &ThreadId {
        match self {
            Self::ThreadCreated { thread_id, .. }
            | Self::TurnAccepted { thread_id, .. }
            | Self::TurnStarted { thread_id, .. }
            | Self::ItemCompleted { thread_id, .. }
            | Self::InteractionRequested { thread_id, .. }
            | Self::InteractionResolved { thread_id, .. }
            | Self::ToolExecutionStarted { thread_id, .. }
            | Self::InteractionCancelled { thread_id, .. }
            | Self::TurnCompleted { thread_id, .. }
            | Self::TurnFailed { thread_id, .. }
            | Self::TurnCancelling { thread_id, .. }
            | Self::TurnInterrupted { thread_id, .. } => thread_id,
        }
    }
}
