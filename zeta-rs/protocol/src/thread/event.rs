use crate::AgentContextSeed;
use crate::AgentJoin;
use crate::AgentJoinId;
use crate::AgentMessage;
use crate::AgentResponse;
use crate::ApprovalMode;
use crate::CommandId;
use crate::ContextCheckpoint;
use crate::DelegationId;
use crate::DelegationResult;
use crate::FrozenSkillActivation;
use crate::InteractionCancelReason;
use crate::ItemId;
use crate::ModelRef;
use crate::ModelUsage;
use crate::RequestId;
use crate::SandboxDenialOutput;
use crate::SessionId;
use crate::StableTurnError;
use crate::ThreadId;
use crate::ThreadItem;
use crate::ToolCallId;
use crate::Turn;
use crate::TurnExecutionBinding;
use crate::TurnId;
use crate::TurnInteraction;
use crate::TurnResourceBudget;
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
    UnsandboxedGrant {
        grant_id: String,
    },
    ExecPolicyGranted {
        layer_id: String,
        rule_id: String,
        exec_policy_revision: String,
    },
    AutoReviewed {
        assessment_id: String,
    },
    PermissionBypassed,
    ApprovedOnce {
        request_id: RequestId,
    },
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
    TurnExecutionBound {
        thread_id: ThreadId,
        binding: TurnExecutionBinding,
    },
    AgentContextSeedCommitted {
        thread_id: ThreadId,
        seed: Box<AgentContextSeed>,
    },
    HistoryImported {
        thread_id: ThreadId,
        source_thread_id: ThreadId,
        before_turn_id: TurnId,
        turns: Vec<Turn>,
    },
    ContextCheckpointCommitted {
        thread_id: ThreadId,
        checkpoint: ContextCheckpoint,
    },
    ContextOverflowRecoveryCommitted {
        thread_id: ThreadId,
        turn_id: TurnId,
        checkpoint: ContextCheckpoint,
    },
    TurnAccepted {
        thread_id: ThreadId,
        turn_id: TurnId,
        #[serde(default = "legacy_turn_action_policy_revision")]
        policy_revision: String,
        #[serde(default)]
        approval_mode: ApprovalMode,
        #[serde(default)]
        activated_skills: Vec<FrozenSkillActivation>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        model: Option<ModelRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        resource_budget: Option<TurnResourceBudget>,
    },
    TurnStarted {
        thread_id: ThreadId,
        turn_id: TurnId,
    },
    TurnSteered {
        thread_id: ThreadId,
        turn_id: TurnId,
        item_ids: Vec<ItemId>,
    },
    TurnSteerDelivered {
        thread_id: ThreadId,
        turn_id: TurnId,
        command_id: CommandId,
    },
    TurnExecutionAttempted {
        thread_id: ThreadId,
        turn_id: TurnId,
        backend: String,
    },
    ModelUsageRecorded {
        thread_id: ThreadId,
        turn_id: TurnId,
        usage: Option<ModelUsage>,
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
    ToolExecutionEscalated {
        thread_id: ThreadId,
        turn_id: TurnId,
        tool_call_id: ToolCallId,
        action_digest: String,
        policy_revision: String,
        denial: SandboxDenialOutput,
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
    DelegationRequested {
        thread_id: ThreadId,
        seed: Box<AgentContextSeed>,
    },
    DelegationStarted {
        thread_id: ThreadId,
        delegation_id: DelegationId,
        child_thread_id: ThreadId,
    },
    DelegationCancellationRequested {
        thread_id: ThreadId,
        delegation_id: DelegationId,
    },
    AgentCancellationReceived {
        thread_id: ThreadId,
        delegation_id: DelegationId,
        parent_thread_id: ThreadId,
    },
    DelegationResultProduced {
        thread_id: ThreadId,
        result: Box<DelegationResult>,
    },
    DelegationResultReceived {
        thread_id: ThreadId,
        result: Box<DelegationResult>,
    },
    AgentMessageSent {
        thread_id: ThreadId,
        message: Box<AgentMessage>,
    },
    AgentMessageReceived {
        thread_id: ThreadId,
        message: Box<AgentMessage>,
    },
    AgentJoinRequested {
        thread_id: ThreadId,
        join: Box<AgentJoin>,
    },
    AgentJoinSatisfied {
        thread_id: ThreadId,
        join_id: AgentJoinId,
        satisfied_by: Vec<DelegationId>,
    },
}

fn legacy_turn_action_policy_revision() -> String {
    "legacy-unversioned-policy".into()
}

impl ThreadEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::ThreadCreated { .. } => "thread.created",
            Self::TurnExecutionBound { .. } => "turn.execution_bound",
            Self::AgentContextSeedCommitted { .. } => "agent.context_seed_committed",
            Self::HistoryImported { .. } => "thread.history_imported",
            Self::ContextCheckpointCommitted { .. } => "context.checkpoint_committed",
            Self::ContextOverflowRecoveryCommitted { .. } => "context.overflow_recovery_committed",
            Self::TurnAccepted { .. } => "turn.accepted",
            Self::TurnStarted { .. } => "turn.started",
            Self::TurnSteered { .. } => "turn.steered",
            Self::TurnSteerDelivered { .. } => "turn.steer_delivered",
            Self::TurnExecutionAttempted { .. } => "turn.execution_attempted",
            Self::ModelUsageRecorded { .. } => "model.usage_recorded",
            Self::ItemCompleted { .. } => "item.completed",
            Self::InteractionRequested { .. } => "interaction.requested",
            Self::InteractionResolved { .. } => "interaction.resolved",
            Self::ToolExecutionStarted { .. } => "tool.execution_started",
            Self::ToolExecutionEscalated { .. } => "tool.execution_escalated",
            Self::InteractionCancelled { .. } => "interaction.cancelled",
            Self::TurnCompleted { .. } => "turn.completed",
            Self::TurnFailed { .. } => "turn.failed",
            Self::TurnCancelling { .. } => "turn.cancelling",
            Self::TurnInterrupted { .. } => "turn.interrupted",
            Self::DelegationRequested { .. } => "agent.delegation_requested",
            Self::DelegationStarted { .. } => "agent.delegation_started",
            Self::DelegationCancellationRequested { .. } => {
                "agent.delegation_cancellation_requested"
            }
            Self::AgentCancellationReceived { .. } => "agent.cancellation_received",
            Self::DelegationResultProduced { .. } => "agent.delegation_result_produced",
            Self::DelegationResultReceived { .. } => "agent.delegation_result_received",
            Self::AgentMessageSent { .. } => "agent.message_sent",
            Self::AgentMessageReceived { .. } => "agent.message_received",
            Self::AgentJoinRequested { .. } => "agent.join_requested",
            Self::AgentJoinSatisfied { .. } => "agent.join_satisfied",
        }
    }

    pub fn thread_id(&self) -> &ThreadId {
        match self {
            Self::ThreadCreated { thread_id, .. }
            | Self::TurnExecutionBound { thread_id, .. }
            | Self::AgentContextSeedCommitted { thread_id, .. }
            | Self::HistoryImported { thread_id, .. }
            | Self::ContextCheckpointCommitted { thread_id, .. }
            | Self::ContextOverflowRecoveryCommitted { thread_id, .. }
            | Self::TurnAccepted { thread_id, .. }
            | Self::TurnStarted { thread_id, .. }
            | Self::TurnSteered { thread_id, .. }
            | Self::TurnSteerDelivered { thread_id, .. }
            | Self::TurnExecutionAttempted { thread_id, .. }
            | Self::ModelUsageRecorded { thread_id, .. }
            | Self::ItemCompleted { thread_id, .. }
            | Self::InteractionRequested { thread_id, .. }
            | Self::InteractionResolved { thread_id, .. }
            | Self::ToolExecutionStarted { thread_id, .. }
            | Self::ToolExecutionEscalated { thread_id, .. }
            | Self::InteractionCancelled { thread_id, .. }
            | Self::TurnCompleted { thread_id, .. }
            | Self::TurnFailed { thread_id, .. }
            | Self::TurnCancelling { thread_id, .. }
            | Self::TurnInterrupted { thread_id, .. }
            | Self::DelegationRequested { thread_id, .. }
            | Self::DelegationStarted { thread_id, .. }
            | Self::DelegationCancellationRequested { thread_id, .. }
            | Self::AgentCancellationReceived { thread_id, .. }
            | Self::DelegationResultProduced { thread_id, .. }
            | Self::DelegationResultReceived { thread_id, .. }
            | Self::AgentMessageSent { thread_id, .. }
            | Self::AgentMessageReceived { thread_id, .. }
            | Self::AgentJoinRequested { thread_id, .. }
            | Self::AgentJoinSatisfied { thread_id, .. } => thread_id,
        }
    }
}
