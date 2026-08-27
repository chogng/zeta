use crate::CoreError;
use crate::context::ContextCalibration;
use crate::context::next_context_calibrations;
use crate::multi_agent::validate_context_seed_digest;
use crate::multi_agent::validate_delegation_result_digest;
use crate::state::transition_turn_status;
use sha2::Digest;
use sha2::Sha256;
use std::collections::{BTreeMap, BTreeSet};
use zeta_history::StoredEvent;
use zeta_history::ThreadCommandReceipt;
use zeta_history::supports_stored_event_schema_version;
use zeta_protocol::AgentContextSeed;
use zeta_protocol::AgentJoin;
use zeta_protocol::AgentJoinId;
use zeta_protocol::AgentJoinPolicy;
use zeta_protocol::AgentJoinStatus;
use zeta_protocol::AgentMessage;
use zeta_protocol::AgentMessageId;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentResponse;
use zeta_protocol::ApprovalMode;
use zeta_protocol::CommandId;
use zeta_protocol::ContextCheckpoint;
use zeta_protocol::ContextCheckpointId;
use zeta_protocol::ContextSourceDigest;
use zeta_protocol::ContextSourceRange;
use zeta_protocol::DelegationId;
use zeta_protocol::DelegationResult;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::ItemId;
use zeta_protocol::ModelRef;
use zeta_protocol::ModelUsageSummary;
use zeta_protocol::PlanUpdate;
use zeta_protocol::RequestId;
use zeta_protocol::SessionId;
use zeta_protocol::StableTurnError;
use zeta_protocol::StableTurnErrorCode;
use zeta_protocol::Thread;
use zeta_protocol::ThreadCommand;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadStatus;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolMode;
use zeta_protocol::ToolProfileSnapshot;
use zeta_protocol::Turn;
use zeta_protocol::TurnExecutionBinding;
use zeta_protocol::TurnId;
use zeta_protocol::TurnInteraction;
use zeta_protocol::TurnStatus;

#[path = "thread_reducer_approval.rs"]
mod approval;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadSnapshot {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub title: String,
    pub turn_execution_binding: Option<TurnExecutionBinding>,
    pub sequence: u64,
    pub usage: ModelUsageSummary,
    pub goal: Option<zeta_protocol::ThreadGoal>,
    /// The Turn that crossed a Goal budget. This is derived from the event log and lets the
    /// remainder of that in-flight Turn be accounted without charging later Turns.
    pub(crate) goal_budget_limited_turn_id: Option<TurnId>,
    pub(crate) context_calibrations: Vec<ContextCalibration>,
    pub turns: Vec<TurnSnapshot>,
    pub items: Vec<ThreadItem>,
    pub context_checkpoints: Vec<ContextCheckpoint>,
    pub context_overflow_recoveries: BTreeMap<TurnId, ContextCheckpointId>,
    pub item_sequences: BTreeMap<ItemId, u64>,
    pub event_digests: BTreeMap<u64, String>,
    pub commands: Vec<ThreadCommandSnapshot>,
    pub steer_deliveries: BTreeMap<CommandId, u64>,
    pub seen_interaction_ids: BTreeSet<RequestId>,
    pub resolved_interactions: Vec<ResolvedTurnInteraction>,
    pub started_tool_calls: BTreeSet<ToolCallId>,
    pub tool_execution_starts: BTreeMap<ToolCallId, ToolExecutionStartSnapshot>,
    pub escalated_tool_calls: BTreeSet<ToolCallId>,
    pub agent_context_seed: Option<AgentContextSeed>,
    pub delegations: BTreeMap<DelegationId, DelegationSnapshot>,
    pub agent_cancellations_received: BTreeSet<DelegationId>,
    pub agent_joins: BTreeMap<AgentJoinId, AgentJoin>,
    pub produced_delegation_results: BTreeMap<DelegationId, DelegationResult>,
    pub received_delegation_results: BTreeMap<DelegationId, DelegationResult>,
    pub sent_agent_messages: BTreeMap<AgentMessageId, AgentMessage>,
    pub received_agent_messages: BTreeMap<AgentMessageId, AgentMessage>,
}

impl ThreadSnapshot {
    pub(crate) fn context_calibration(
        &self,
        model: &ModelRef,
        estimator_revision: &str,
    ) -> Option<&ContextCalibration> {
        self.context_calibrations
            .iter()
            .find(|calibration| calibration.matches(model, estimator_revision))
    }

    pub fn context_source_digest(
        &self,
        range: ContextSourceRange,
    ) -> Result<ContextSourceDigest, CoreError> {
        if range.start_sequence == 0 || range.start_sequence > range.end_sequence {
            return Err(CoreError::Context(
                "context source range must be a non-empty inclusive sequence range".into(),
            ));
        }
        let mut hasher = Sha256::new();
        for sequence in range.start_sequence..=range.end_sequence {
            let digest = self.event_digests.get(&sequence).ok_or_else(|| {
                CoreError::Context(format!(
                    "context source range references unavailable Thread sequence {sequence}"
                ))
            })?;
            hasher.update(sequence.to_be_bytes());
            hasher.update(digest.as_bytes());
        }
        ContextSourceDigest::new(format!("sha256:{:x}", hasher.finalize()))
            .map_err(|error| CoreError::Context(error.to_string()))
    }

    /// Builds the canonical public Thread projection without exposing command receipts.
    pub fn public_thread(&self) -> Thread {
        Thread {
            session_id: self.session_id.clone(),
            thread_id: self.thread_id.clone(),
            title: self.title.clone(),
            status: ThreadStatus::Active,
            sequence: self.sequence,
            usage: self.usage.clone(),
            goal: self.goal.clone(),
            turns: self
                .turns
                .iter()
                .map(|turn| Turn {
                    turn_id: turn.turn_id.clone(),
                    status: turn.status,
                    model: turn.model.clone(),
                    tool_profile: turn.tool_profile.clone(),
                    tool_mode: turn.tool_mode,
                    usage: turn.usage.clone(),
                    items: self
                        .items
                        .iter()
                        .filter(|item| item.turn_id() == &turn.turn_id)
                        .cloned()
                        .collect(),
                    plan: turn.plan.clone(),
                    pending_interaction: turn
                        .pending_interaction
                        .as_ref()
                        .map(TurnInteraction::pending_state),
                    error: turn.failure.clone(),
                })
                .collect(),
        }
    }

    /// Returns whether a Turn owns an exact durable Tool Call that has no terminal result.
    pub fn has_resumable_tool_continuation(&self, turn_id: &TurnId) -> bool {
        self.items.iter().any(|item| {
            let ThreadItem::ToolCall {
                turn_id: item_turn_id,
                tool_call_id,
                ..
            } = item
            else {
                return false;
            };
            if item_turn_id != turn_id
                || self.items.iter().any(|candidate| {
                    matches!(
                        candidate,
                        ThreadItem::ToolResult {
                            tool_call_id: result_call_id,
                            ..
                        } if result_call_id == tool_call_id
                    )
                })
            {
                return false;
            }
            true
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnSnapshot {
    pub turn_id: TurnId,
    pub status: TurnStatus,
    pub model: Option<ModelRef>,
    pub policy_revision: String,
    pub approval_mode: ApprovalMode,
    pub tool_mode: ToolMode,
    pub activated_skills: Vec<FrozenSkillActivation>,
    pub failure: Option<StableTurnError>,
    pub pending_interaction: Option<TurnInteraction>,
    pub execution_backend_attempt: Option<String>,
    pub tool_profile: Option<ToolProfileSnapshot>,
    pub plan: Option<PlanUpdate>,
    pub usage: ModelUsageSummary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolExecutionStartSnapshot {
    pub action_digest: String,
    pub policy_revision: String,
    pub authority: zeta_protocol::ToolExecutionAuthority,
}

/// Durable parent-side projection for one child Agent delegation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelegationSnapshot {
    pub seed: AgentContextSeed,
    pub child_thread_id: Option<ThreadId>,
    pub cancellation_requested: bool,
}

/// A durable interaction response retained for exact continuation after a process restart.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedTurnInteraction {
    pub turn_id: TurnId,
    pub interaction: TurnInteraction,
    pub response: AgentResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadCommandSnapshot {
    pub receipt: ThreadCommandReceipt,
    pub result: ThreadCommandResult,
    pub response_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ThreadCommandResult {
    TurnAccepted {
        turn_id: TurnId,
    },
    TurnSteered {
        turn_id: TurnId,
    },
    TurnInterrupted {
        turn_id: TurnId,
    },
    InteractionResolved {
        turn_id: TurnId,
        request_id: RequestId,
    },
}

/// Applies one durable event to a Thread projection without performing I/O.
///
/// Callers use the returned projection only after the corresponding event append succeeds.
/// Recovery code uses the same reducer, ensuring live writes and rollout replay share transition
/// validation and sequence rules.
pub fn reduce_thread_event(
    snapshot: Option<ThreadSnapshot>,
    envelope: &StoredEvent,
) -> Result<ThreadSnapshot, CoreError> {
    if !supports_stored_event_schema_version(envelope.schema_version) {
        return Err(CoreError::Journal(format!(
            "unsupported Thread event schema version {}",
            envelope.schema_version
        )));
    }

    let Some(mut snapshot) = snapshot else {
        if envelope.sequence != 1 {
            return Err(CoreError::Journal(
                "first Thread event must have sequence 1".into(),
            ));
        }
        return match &envelope.event {
            ThreadEvent::ThreadCreated {
                session_id,
                title,
                thread_id,
            } => {
                let mut event_digests = BTreeMap::new();
                event_digests.insert(envelope.sequence, event_digest(&envelope.event)?);
                Ok(ThreadSnapshot {
                    session_id: session_id.clone(),
                    thread_id: thread_id.clone(),
                    title: title.clone(),
                    turn_execution_binding: None,
                    sequence: envelope.sequence,
                    usage: ModelUsageSummary::default(),
                    goal: None,
                    goal_budget_limited_turn_id: None,
                    context_calibrations: Vec::new(),
                    turns: Vec::new(),
                    items: Vec::new(),
                    context_checkpoints: Vec::new(),
                    context_overflow_recoveries: BTreeMap::new(),
                    item_sequences: BTreeMap::new(),
                    event_digests,
                    seen_interaction_ids: BTreeSet::new(),
                    resolved_interactions: Vec::new(),
                    started_tool_calls: BTreeSet::new(),
                    tool_execution_starts: BTreeMap::new(),
                    escalated_tool_calls: BTreeSet::new(),
                    agent_context_seed: None,
                    delegations: BTreeMap::new(),
                    agent_cancellations_received: BTreeSet::new(),
                    agent_joins: BTreeMap::new(),
                    produced_delegation_results: BTreeMap::new(),
                    received_delegation_results: BTreeMap::new(),
                    sent_agent_messages: BTreeMap::new(),
                    received_agent_messages: BTreeMap::new(),
                    commands: {
                        require_no_command(envelope)?;
                        Vec::new()
                    },
                    steer_deliveries: BTreeMap::new(),
                })
            }
            _ => Err(CoreError::Journal(
                "first Thread event must create the Thread".into(),
            )),
        };
    };

    if envelope.thread_id() != &snapshot.thread_id
        || envelope.sequence != snapshot.sequence.saturating_add(1)
    {
        return Err(CoreError::Journal(
            "invalid Thread rollout identity or sequence".into(),
        ));
    }

    match &envelope.event {
        ThreadEvent::ThreadCreated { .. } => {
            return Err(CoreError::Journal(
                "Thread cannot be created more than once".into(),
            ));
        }
        ThreadEvent::GoalCreated { thread_id, goal } => {
            require_no_command(envelope)?;
            validate_goal_identity(&snapshot, thread_id, goal)?;
            if snapshot.goal.is_some() {
                return Err(CoreError::Journal(
                    "Thread can own only one Goal at a time".into(),
                ));
            }
            if goal.tokens_used != 0 {
                return Err(CoreError::Journal(
                    "a newly created Goal must have zero tokens used".into(),
                ));
            }
            snapshot.goal = Some(goal.clone());
            snapshot.goal_budget_limited_turn_id = None;
        }
        ThreadEvent::GoalUpdated { thread_id, goal } => {
            require_no_command(envelope)?;
            validate_goal_identity(&snapshot, thread_id, goal)?;
            let current = snapshot.goal.as_ref().ok_or_else(|| {
                CoreError::Journal("cannot update a Thread without a Goal".into())
            })?;
            if current.goal_id != goal.goal_id {
                return Err(CoreError::Journal(
                    "Goal update ID does not match the current Thread Goal".into(),
                ));
            }
            if current.tokens_used != goal.tokens_used {
                return Err(CoreError::Journal(
                    "Goal updates cannot change the durable token counter".into(),
                ));
            }
            snapshot.goal = Some(goal.clone());
            snapshot.goal_budget_limited_turn_id = None;
        }
        ThreadEvent::GoalCleared {
            thread_id,
            goal_id,
        } => {
            require_no_command(envelope)?;
            if thread_id != &snapshot.thread_id {
                return Err(CoreError::Journal(
                    "Goal event Thread identity does not match the rollout".into(),
                ));
            }
            let current = snapshot
                .goal
                .as_ref()
                .ok_or_else(|| CoreError::Journal("cannot clear a missing Thread Goal".into()))?;
            if current.goal_id != *goal_id {
                return Err(CoreError::Journal(
                    "Goal clear ID does not match the current Thread Goal".into(),
                ));
            }
            snapshot.goal = None;
            snapshot.goal_budget_limited_turn_id = None;
        }
        ThreadEvent::TurnExecutionBound { binding, .. } => {
            require_no_command(envelope)?;
            if binding.backend.trim().is_empty()
                || binding.remote_thread_id.trim().is_empty()
                || binding.execution_scope.trim().is_empty()
            {
                return Err(CoreError::Journal(
                    "Turn execution binding identities and scope must not be empty".into(),
                ));
            }
            if !snapshot
                .turns
                .iter()
                .any(|turn| turn.status == TurnStatus::Completed)
            {
                return Err(CoreError::Journal(
                    "Turn execution binding requires a completed Turn".into(),
                ));
            }
            if snapshot.turn_execution_binding.is_some() {
                return Err(CoreError::Journal(
                    "Thread Turn execution binding is immutable".into(),
                ));
            }
            snapshot.turn_execution_binding = Some(binding.clone());
        }
        ThreadEvent::TurnExecutionAttempted {
            turn_id, backend, ..
        } => {
            require_no_command(envelope)?;
            if backend.trim().is_empty() {
                return Err(CoreError::Journal(
                    "Turn execution backend identity must not be empty".into(),
                ));
            }
            let turn = snapshot
                .turns
                .iter_mut()
                .find(|turn| &turn.turn_id == turn_id)
                .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
            if turn.status != TurnStatus::Running || turn.execution_backend_attempt.is_some() {
                return Err(CoreError::Journal(
                    "Turn execution can be attempted once while running".into(),
                ));
            }
            turn.execution_backend_attempt = Some(backend.clone());
        }
        ThreadEvent::ModelUsageRecorded {
            turn_id,
            usage,
            input_estimate,
            ..
        } => {
            require_no_command(envelope)?;
            let next_thread_usage =
                snapshot
                    .usage
                    .checked_record(usage.as_ref())
                    .ok_or_else(|| {
                        CoreError::Journal("Thread model usage aggregate overflowed".into())
                    })?;
            let turn_index = snapshot
                .turns
                .iter()
                .position(|turn| &turn.turn_id == turn_id)
                .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
            let turn = &snapshot.turns[turn_index];
            if turn.status == TurnStatus::Created {
                return Err(CoreError::Journal(
                    "model usage requires a Turn that has started execution".into(),
                ));
            }
            let next_turn_usage = turn.usage.checked_record(usage.as_ref()).ok_or_else(|| {
                CoreError::Journal("Turn model usage aggregate overflowed".into())
            })?;
            let next_calibrations = match input_estimate {
                Some(estimate) => {
                    let model = turn.model.as_ref().ok_or_else(|| {
                        CoreError::Journal(
                            "context calibration requires a frozen selected model".into(),
                        )
                    })?;
                    Some(
                        next_context_calibrations(
                            &snapshot.context_calibrations,
                            model,
                            estimate,
                            usage.as_ref(),
                        )
                        .map_err(|error| CoreError::Journal(error.to_string()))?,
                    )
                }
                None => None,
            };
            snapshot.turns[turn_index].usage = next_turn_usage;
            snapshot.usage = next_thread_usage;
            if let Some(goal) = snapshot.goal.as_mut() {
                let account_usage = goal.status.allows_usage_accounting()
                    && (goal.status != zeta_protocol::ThreadGoalStatus::BudgetLimited
                        || snapshot.goal_budget_limited_turn_id.as_ref() == Some(turn_id));
                if account_usage {
                    goal.tokens_used = goal
                        .tokens_used
                        .checked_add(goal_token_delta(usage.as_ref()))
                        .ok_or_else(|| {
                            CoreError::Journal("Thread Goal token usage overflowed".into())
                        })?;
                    if goal
                        .token_budget
                        .is_some_and(|budget| goal.tokens_used >= budget)
                    {
                        goal.status = zeta_protocol::ThreadGoalStatus::BudgetLimited;
                        snapshot.goal_budget_limited_turn_id = Some(turn_id.clone());
                    }
                }
            }
            if let Some(next_calibrations) = next_calibrations {
                snapshot.context_calibrations = next_calibrations;
            }
        }
        ThreadEvent::AgentContextSeedCommitted { seed, .. } => {
            require_no_command(envelope)?;
            if snapshot.sequence != 1
                || snapshot.agent_context_seed.is_some()
                || seed.parent_thread_id == snapshot.thread_id
                || seed.parent_sequence == 0
            {
                return Err(CoreError::Journal(
                    "Agent context seed must be committed once immediately after child creation"
                        .into(),
                ));
            }
            validate_agent_context_seed(seed)?;
            snapshot.agent_context_seed = Some(seed.as_ref().clone());
        }
        ThreadEvent::HistoryImported {
            source_thread_id,
            before_turn_id,
            turns,
            ..
        } => {
            require_no_command(envelope)?;
            import_history(&mut snapshot, source_thread_id, before_turn_id, turns)?;
            for item in &snapshot.items {
                snapshot
                    .item_sequences
                    .insert(item.item_id().clone(), envelope.sequence);
            }
        }
        ThreadEvent::ContextCheckpointCommitted { checkpoint, .. } => {
            require_no_command(envelope)?;
            validate_context_checkpoint(&snapshot, checkpoint)?;
            snapshot.context_checkpoints.push(checkpoint.clone());
        }
        ThreadEvent::ContextOverflowRecoveryCommitted {
            turn_id,
            checkpoint,
            ..
        } => {
            require_no_command(envelope)?;
            let turn = snapshot
                .turns
                .iter()
                .find(|turn| &turn.turn_id == turn_id)
                .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
            if turn.status != TurnStatus::Running
                || snapshot.context_overflow_recoveries.contains_key(turn_id)
            {
                return Err(CoreError::Journal(
                    "context overflow recovery can be committed once for a running Turn".into(),
                ));
            }
            let current_turn_start = snapshot
                .items
                .iter()
                .filter(|item| item.turn_id() == turn_id)
                .filter_map(|item| snapshot.item_sequences.get(item.item_id()))
                .copied()
                .min()
                .ok_or_else(|| {
                    CoreError::Journal(
                        "context overflow recovery requires durable current-Turn input".into(),
                    )
                })?;
            if checkpoint.covered.end_sequence >= current_turn_start {
                return Err(CoreError::Journal(
                    "context overflow recovery checkpoint cannot absorb the current Turn".into(),
                ));
            }
            validate_context_checkpoint(&snapshot, checkpoint)?;
            snapshot
                .context_overflow_recoveries
                .insert(turn_id.clone(), checkpoint.checkpoint_id.clone());
            snapshot.context_checkpoints.push(checkpoint.clone());
        }
        ThreadEvent::TurnAccepted {
            turn_id,
            model,
            policy_revision,
            approval_mode,
            tool_mode,
            activated_skills,
            tool_profile,
            ..
        } => {
            if policy_revision.trim().is_empty() {
                return Err(CoreError::Journal(
                    "Turn policy revision must not be empty".into(),
                ));
            }
            if let Some(tool_profile) = tool_profile {
                crate::tool_profile::validate_tool_profile_snapshot(tool_profile)
                    .map_err(CoreError::Journal)?;
            }
            create_turn(
                &mut snapshot,
                TurnSnapshot {
                    turn_id: turn_id.clone(),
                    status: TurnStatus::Created,
                    model: model.clone(),
                    policy_revision: policy_revision.clone(),
                    approval_mode: *approval_mode,
                    tool_mode: *tool_mode,
                    activated_skills: activated_skills.clone(),
                    failure: None,
                    pending_interaction: None,
                    execution_backend_attempt: None,
                    tool_profile: tool_profile.clone(),
                    plan: None,
                    usage: ModelUsageSummary::default(),
                },
            )?;
            let receipt = envelope.command.clone().ok_or_else(|| {
                CoreError::Journal("Turn acceptance requires a command receipt".into())
            })?;
            let matching_start = match &receipt.command {
                ThreadCommand::StartTurn {
                    model: command_model,
                    activated_skills: command_skills,
                    approval_mode: command_approval_mode,
                    tool_mode: command_tool_mode,
                    tool_profile: command_tool_profile,
                    input,
                    ..
                } => {
                    command_model == model
                        && command_skills == activated_skills
                        && command_approval_mode == approval_mode
                        && command_tool_mode == tool_mode
                        && command_tool_profile.as_deref() == tool_profile.as_ref()
                        && turn_skill_activations_match(input, activated_skills)
                }
                ThreadCommand::StartShellTurn {
                    approval_mode: command_approval_mode,
                    ..
                } => {
                    model.is_none()
                        && tool_profile.is_none()
                        && *tool_mode == ToolMode::Direct
                        && command_approval_mode == approval_mode
                        && activated_skills.is_empty()
                }
                ThreadCommand::CompactContext {
                    model: command_model,
                    ..
                } => {
                    command_model == model
                        && tool_profile.is_none()
                        && *tool_mode == ToolMode::Direct
                        && *approval_mode == ApprovalMode::AskPermissions
                        && activated_skills.is_empty()
                }
                _ => false,
            };
            if !matching_start {
                return Err(CoreError::Journal(
                    "Turn acceptance requires a matching start-Turn command".into(),
                ));
            }
            if snapshot
                .commands
                .iter()
                .any(|existing| existing.receipt.command_id == receipt.command_id)
            {
                return Err(CoreError::Journal(
                    "Thread command ID is already registered".into(),
                ));
            }
            snapshot.commands.push(ThreadCommandSnapshot {
                receipt,
                result: ThreadCommandResult::TurnAccepted {
                    turn_id: turn_id.clone(),
                },
                response_sequence: envelope.sequence,
            });
        }
        ThreadEvent::TurnStarted { turn_id, .. } => {
            require_no_command(envelope)?;
            transition_turn(&mut snapshot, turn_id, TurnStatus::Running, None)?;
            if let Some(command) = snapshot.commands.iter_mut().find(|command| {
                matches!(
                    &command.result,
                    ThreadCommandResult::TurnAccepted {
                        turn_id: command_turn_id,
                    } if command_turn_id == turn_id
                )
            }) {
                command.response_sequence = envelope.sequence;
            }
        }
        ThreadEvent::TurnSteered {
            turn_id, item_ids, ..
        } => {
            let turn = find_turn(&snapshot, turn_id)?;
            if !matches!(
                turn.status,
                TurnStatus::Running
                    | TurnStatus::WaitingForApproval
                    | TurnStatus::WaitingForUserInput
            ) {
                return Err(CoreError::Journal(format!(
                    "cannot steer a {:?} Turn",
                    turn.status
                )));
            }
            let receipt = envelope.command.clone().ok_or_else(|| {
                CoreError::Journal("Turn steering requires a command receipt".into())
            })?;
            let input = match &receipt.command {
                ThreadCommand::SteerTurn {
                    turn_id: command_turn_id,
                    input,
                } if command_turn_id == turn_id => input,
                _ => {
                    return Err(CoreError::Journal(
                        "Turn steering command does not match its event".into(),
                    ));
                }
            };
            validate_steered_items(&snapshot, turn_id, input, item_ids, envelope.sequence)?;
            if snapshot
                .commands
                .iter()
                .any(|existing| existing.receipt.command_id == receipt.command_id)
            {
                return Err(CoreError::Journal(
                    "Thread command ID is already registered".into(),
                ));
            }
            snapshot.commands.push(ThreadCommandSnapshot {
                receipt,
                result: ThreadCommandResult::TurnSteered {
                    turn_id: turn_id.clone(),
                },
                response_sequence: envelope.sequence,
            });
        }
        ThreadEvent::TurnSteerDelivered {
            turn_id,
            command_id,
            ..
        } => {
            require_no_command(envelope)?;
            if !snapshot.commands.iter().any(|command| {
                command.receipt.command_id == *command_id
                    && matches!(
                        &command.result,
                        ThreadCommandResult::TurnSteered {
                            turn_id: command_turn_id,
                        } if command_turn_id == turn_id
                    )
            }) {
                return Err(CoreError::Journal(
                    "Turn steer delivery must reference its accepted command".into(),
                ));
            }
            if snapshot
                .steer_deliveries
                .insert(command_id.clone(), envelope.sequence)
                .is_some()
            {
                return Err(CoreError::Journal(
                    "Turn steer command is already marked delivered".into(),
                ));
            }
        }
        ThreadEvent::ItemCompleted { turn_id, item, .. } => {
            require_no_command(envelope)?;
            if item.turn_id() != turn_id {
                return Err(CoreError::Journal(
                    "Item Turn identity does not match its event".into(),
                ));
            }
            let turn_status = snapshot
                .turns
                .iter()
                .find(|turn| turn.turn_id == *turn_id)
                .map(|turn| turn.status)
                .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
            if matches!(
                turn_status,
                TurnStatus::Cancelling
                    | TurnStatus::Completed
                    | TurnStatus::Failed
                    | TurnStatus::Interrupted
            ) {
                return Err(CoreError::Journal(format!(
                    "cannot append an Item to a {turn_status:?} Turn"
                )));
            }
            if snapshot
                .items
                .iter()
                .any(|existing| existing.item_id() == item.item_id())
            {
                return Err(CoreError::Journal(format!(
                    "Item already exists: {}",
                    item.item_id()
                )));
            }
            match item {
                ThreadItem::ToolCall { tool_call_id, .. } => {
                    if snapshot.items.iter().any(|existing| {
                        matches!(
                            existing,
                            ThreadItem::ToolCall {
                                tool_call_id: existing_id,
                                ..
                            } if existing_id == tool_call_id
                        )
                    }) {
                        return Err(CoreError::Journal(format!(
                            "Tool Call already exists: {tool_call_id}"
                        )));
                    }
                }
                ThreadItem::ToolResult { tool_call_id, .. } => {
                    let has_call = snapshot.items.iter().any(|existing| {
                        matches!(
                            existing,
                            ThreadItem::ToolCall {
                                turn_id: existing_turn_id,
                                tool_call_id: existing_id,
                                ..
                            } if existing_turn_id == turn_id && existing_id == tool_call_id
                        )
                    });
                    let has_result = snapshot.items.iter().any(|existing| {
                        matches!(
                            existing,
                            ThreadItem::ToolResult {
                                tool_call_id: existing_id,
                                ..
                            } if existing_id == tool_call_id
                        )
                    });
                    if !has_call {
                        return Err(CoreError::Journal(format!(
                            "Tool Result references an unknown Tool Call: {tool_call_id}"
                        )));
                    }
                    if has_result {
                        return Err(CoreError::Journal(format!(
                            "Tool Result already exists: {tool_call_id}"
                        )));
                    }
                }
                ThreadItem::UserMessage { .. }
                | ThreadItem::UserContext { .. }
                | ThreadItem::UserImage { .. }
                | ThreadItem::UserImageAttachment { .. }
                | ThreadItem::AgentMessage { .. }
                | ThreadItem::Reasoning { .. }
                | ThreadItem::Plan { .. } => {}
            }
            snapshot.items.push(item.clone());
            snapshot
                .item_sequences
                .insert(item.item_id().clone(), envelope.sequence);
        }
        ThreadEvent::PlanUpdated { turn_id, plan, .. } => {
            require_no_command(envelope)?;
            crate::turn::validate_plan_update(plan).map_err(CoreError::Journal)?;
            let turn = find_turn_mut(&mut snapshot, turn_id)?;
            if turn.status != TurnStatus::Running {
                return Err(CoreError::Journal(format!(
                    "cannot update the plan for a {:?} Turn",
                    turn.status
                )));
            }
            if turn.plan.as_ref() == Some(plan) {
                return Err(CoreError::Journal(
                    "duplicate plan update must not be appended".into(),
                ));
            }
            turn.plan = Some(plan.clone());
        }
        ThreadEvent::InteractionRequested {
            turn_id,
            interaction,
            ..
        } => {
            require_no_command(envelope)?;
            validate_agent_request(&interaction.request).map_err(CoreError::Journal)?;
            if !snapshot
                .seen_interaction_ids
                .insert(interaction.request_id.clone())
            {
                return Err(CoreError::Journal(format!(
                    "interaction request ID is already registered: {}",
                    interaction.request_id
                )));
            }
            let turn = snapshot
                .turns
                .iter()
                .find(|turn| turn.turn_id == *turn_id)
                .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
            if turn.pending_interaction.is_some() {
                return Err(CoreError::Journal(
                    "a Turn cannot wait for more than one interaction".into(),
                ));
            }
            transition_turn(
                &mut snapshot,
                turn_id,
                waiting_status_for(&interaction.request),
                None,
            )?;
            find_turn_mut(&mut snapshot, turn_id)?.pending_interaction = Some(interaction.clone());
        }
        ThreadEvent::InteractionResolved {
            turn_id,
            request_id,
            response,
            ..
        } => {
            let interaction = pending_interaction(&snapshot, turn_id, request_id)?;
            if interaction.request.kind() != response.kind() {
                return Err(CoreError::Journal(
                    "interaction response does not match the outstanding request kind".into(),
                ));
            }
            let receipt = envelope.command.clone().ok_or_else(|| {
                CoreError::Journal("interaction resolution requires a command receipt".into())
            })?;
            if receipt.command != resolution_command(turn_id, request_id, response) {
                return Err(CoreError::Journal(
                    "interaction resolution command does not match its event".into(),
                ));
            }
            if snapshot
                .commands
                .iter()
                .any(|existing| existing.receipt.command_id == receipt.command_id)
            {
                return Err(CoreError::Journal(
                    "Thread command ID is already registered".into(),
                ));
            }
            transition_turn(&mut snapshot, turn_id, TurnStatus::Running, None)?;
            find_turn_mut(&mut snapshot, turn_id)?.pending_interaction = None;
            snapshot
                .resolved_interactions
                .push(ResolvedTurnInteraction {
                    turn_id: turn_id.clone(),
                    interaction,
                    response: response.clone(),
                });
            snapshot.commands.push(ThreadCommandSnapshot {
                receipt,
                result: ThreadCommandResult::InteractionResolved {
                    turn_id: turn_id.clone(),
                    request_id: request_id.clone(),
                },
                response_sequence: envelope.sequence,
            });
        }
        ThreadEvent::ToolExecutionStarted {
            turn_id,
            tool_call_id,
            action_digest,
            policy_revision,
            authority,
            ..
        } => {
            require_no_command(envelope)?;
            if action_digest.trim().is_empty() || policy_revision.trim().is_empty() {
                return Err(CoreError::Journal(
                    "tool execution marker requires action and policy identities".into(),
                ));
            }
            let turn = snapshot
                .turns
                .iter()
                .find(|turn| turn.turn_id == *turn_id)
                .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
            if turn.status != TurnStatus::Running {
                return Err(CoreError::Journal(
                    "tool execution can start only while its Turn is running".into(),
                ));
            }
            let has_call = snapshot.items.iter().any(|item| {
                matches!(
                    item,
                    ThreadItem::ToolCall {
                        turn_id: item_turn_id,
                        tool_call_id: item_call_id,
                        ..
                    } if item_turn_id == turn_id && item_call_id == tool_call_id
                )
            });
            let has_result = snapshot.items.iter().any(|item| {
                matches!(
                    item,
                    ThreadItem::ToolResult {
                        tool_call_id: item_call_id,
                        ..
                    } if item_call_id == tool_call_id
                )
            });
            if !has_call || has_result {
                return Err(CoreError::Journal(
                    "tool execution marker must reference an unresolved Tool Call".into(),
                ));
            }
            if !snapshot.started_tool_calls.insert(tool_call_id.clone()) {
                return Err(CoreError::Journal(format!(
                    "tool execution already started: {tool_call_id}"
                )));
            }
            snapshot.tool_execution_starts.insert(
                tool_call_id.clone(),
                ToolExecutionStartSnapshot {
                    action_digest: action_digest.clone(),
                    policy_revision: policy_revision.clone(),
                    authority: authority.clone(),
                },
            );
        }
        ThreadEvent::ToolExecutionEscalated {
            turn_id,
            tool_call_id,
            action_digest,
            policy_revision,
            denial,
            authority,
            ..
        } => {
            require_no_command(envelope)?;
            if action_digest.trim().is_empty()
                || policy_revision.trim().is_empty()
                || denial.reason().trim().is_empty()
            {
                return Err(CoreError::Journal(
                    "tool escalation requires action, policy, and denial identities".into(),
                ));
            }
            let turn = snapshot
                .turns
                .iter()
                .find(|turn| turn.turn_id == *turn_id)
                .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
            if turn.status != TurnStatus::Running {
                return Err(CoreError::Journal(
                    "tool execution can escalate only while its Turn is running".into(),
                ));
            }
            let has_result = snapshot.items.iter().any(|item| {
                matches!(
                    item,
                    ThreadItem::ToolResult {
                        tool_call_id: item_call_id,
                        ..
                    } if item_call_id == tool_call_id
                )
            });
            let Some(start) = snapshot.tool_execution_starts.get(tool_call_id) else {
                return Err(CoreError::Journal(
                    "tool escalation must reference a started Tool Call".into(),
                ));
            };
            if has_result {
                return Err(CoreError::Journal(
                    "tool escalation must reference a started unresolved Tool Call".into(),
                ));
            }
            if start.action_digest != *action_digest
                || start.policy_revision != *policy_revision
                || !matches!(
                    start.authority,
                    zeta_protocol::ToolExecutionAuthority::Sandboxed
                )
            {
                return Err(CoreError::Journal(
                    "tool escalation must preserve the sandboxed start binding".into(),
                ));
            }
            if denial.replay_safety() != zeta_protocol::ToolReplaySafety::SafeToRetry {
                return Err(CoreError::Journal(
                    "tool escalation requires a safe-to-retry sandbox denial".into(),
                ));
            }
            approval::validate_escalation_authority(
                &snapshot,
                turn_id,
                tool_call_id,
                action_digest,
                policy_revision,
                denial,
                authority,
            )?;
            if !snapshot.escalated_tool_calls.insert(tool_call_id.clone()) {
                return Err(CoreError::Journal(format!(
                    "tool execution already escalated: {tool_call_id}"
                )));
            }
        }
        ThreadEvent::InteractionCancelled {
            turn_id,
            request_id,
            ..
        } => {
            if let Some(receipt) = envelope.command.clone() {
                if receipt.command
                    != (ThreadCommand::InterruptTurn {
                        turn_id: turn_id.clone(),
                    })
                {
                    return Err(CoreError::Journal(
                        "interaction cancellation command does not match its event".into(),
                    ));
                }
                if snapshot
                    .commands
                    .iter()
                    .any(|existing| existing.receipt.command_id == receipt.command_id)
                {
                    return Err(CoreError::Journal(
                        "Thread command ID is already registered".into(),
                    ));
                }
                snapshot.commands.push(ThreadCommandSnapshot {
                    receipt,
                    result: ThreadCommandResult::TurnInterrupted {
                        turn_id: turn_id.clone(),
                    },
                    response_sequence: envelope.sequence,
                });
            }
            pending_interaction(&snapshot, turn_id, request_id)?;
            transition_turn(&mut snapshot, turn_id, TurnStatus::Running, None)?;
            find_turn_mut(&mut snapshot, turn_id)?.pending_interaction = None;
        }
        ThreadEvent::TurnCompleted { turn_id, .. } => {
            require_no_command(envelope)?;
            transition_turn(&mut snapshot, turn_id, TurnStatus::Completed, None)?;
        }
        ThreadEvent::TurnFailed { turn_id, error, .. } => {
            require_no_command(envelope)?;
            transition_turn(
                &mut snapshot,
                turn_id,
                TurnStatus::Failed,
                Some(error.clone()),
            )?;
            if let Some(goal) = snapshot.goal.as_mut() {
                if !goal.status.is_terminal() {
                    goal.status = if error.code == StableTurnErrorCode::UsageLimited {
                        zeta_protocol::ThreadGoalStatus::UsageLimited
                    } else {
                        zeta_protocol::ThreadGoalStatus::Blocked
                    };
                }
            }
        }
        ThreadEvent::TurnCancelling { turn_id, .. } => {
            if let Some(receipt) = envelope.command.clone() {
                if receipt.command
                    != (ThreadCommand::InterruptTurn {
                        turn_id: turn_id.clone(),
                    })
                {
                    return Err(CoreError::Journal(
                        "Turn cancellation command does not match its event".into(),
                    ));
                }
                if snapshot
                    .commands
                    .iter()
                    .any(|existing| existing.receipt.command_id == receipt.command_id)
                {
                    return Err(CoreError::Journal(
                        "Thread command ID is already registered".into(),
                    ));
                }
                snapshot.commands.push(ThreadCommandSnapshot {
                    receipt,
                    result: ThreadCommandResult::TurnInterrupted {
                        turn_id: turn_id.clone(),
                    },
                    response_sequence: envelope.sequence,
                });
            }
            if find_turn(&snapshot, turn_id)?.pending_interaction.is_some() {
                return Err(CoreError::Journal(
                    "Turn cancellation must close its outstanding interaction first".into(),
                ));
            }
            transition_turn(&mut snapshot, turn_id, TurnStatus::Cancelling, None)?;
        }
        ThreadEvent::TurnInterrupted { turn_id, .. } => {
            require_no_command(envelope)?;
            transition_turn(&mut snapshot, turn_id, TurnStatus::Interrupted, None)?;
            if let Some(command) = snapshot.commands.iter_mut().find(|command| {
                matches!(
                    &command.result,
                    ThreadCommandResult::TurnInterrupted {
                        turn_id: command_turn_id,
                    } if command_turn_id == turn_id
                )
            }) {
                command.response_sequence = envelope.sequence;
            }
        }
        ThreadEvent::DelegationRequested { seed, .. } => {
            require_no_command(envelope)?;
            validate_agent_context_seed(seed)?;
            if seed.parent_thread_id != snapshot.thread_id
                || seed.parent_sequence != snapshot.sequence
            {
                return Err(CoreError::Journal(
                    "delegation seed must anchor the current parent Thread sequence".into(),
                ));
            }
            let turn = find_turn(&snapshot, &seed.parent_turn_id)?;
            if !matches!(
                turn.status,
                TurnStatus::Running
                    | TurnStatus::WaitingForApproval
                    | TurnStatus::WaitingForUserInput
                    | TurnStatus::WaitingForCapability
            ) {
                return Err(CoreError::Journal(
                    "delegation parent Turn must still be active".into(),
                ));
            }
            if snapshot
                .delegations
                .insert(
                    seed.delegation_id.clone(),
                    DelegationSnapshot {
                        seed: seed.as_ref().clone(),
                        child_thread_id: None,
                        cancellation_requested: false,
                    },
                )
                .is_some()
            {
                return Err(CoreError::Journal(format!(
                    "delegation already exists: {}",
                    seed.delegation_id
                )));
            }
        }
        ThreadEvent::DelegationStarted {
            delegation_id,
            child_thread_id,
            ..
        } => {
            require_no_command(envelope)?;
            if child_thread_id == &snapshot.thread_id {
                return Err(CoreError::Journal(
                    "delegation child must be a different Thread".into(),
                ));
            }
            let delegation = snapshot
                .delegations
                .get_mut(delegation_id)
                .ok_or_else(|| CoreError::NotFound(delegation_id.to_string()))?;
            if delegation.child_thread_id.is_some() {
                return Err(CoreError::Journal(format!(
                    "delegation already started: {delegation_id}"
                )));
            }
            delegation.child_thread_id = Some(child_thread_id.clone());
        }
        ThreadEvent::DelegationCancellationRequested { delegation_id, .. } => {
            require_no_command(envelope)?;
            let delegation = snapshot
                .delegations
                .get_mut(delegation_id)
                .ok_or_else(|| CoreError::NotFound(delegation_id.to_string()))?;
            if delegation.cancellation_requested {
                return Err(CoreError::Journal(format!(
                    "delegation cancellation was already requested: {delegation_id}"
                )));
            }
            delegation.cancellation_requested = true;
        }
        ThreadEvent::AgentCancellationReceived {
            delegation_id,
            parent_thread_id,
            ..
        } => {
            require_no_command(envelope)?;
            let seed = snapshot.agent_context_seed.as_ref().ok_or_else(|| {
                CoreError::Journal("only an Agent child can receive tree cancellation".into())
            })?;
            if seed.delegation_id != *delegation_id
                || seed.parent_thread_id != *parent_thread_id
                || !snapshot
                    .agent_cancellations_received
                    .insert(delegation_id.clone())
            {
                return Err(CoreError::Journal(
                    "Agent cancellation does not match the immutable child seed".into(),
                ));
            }
        }
        ThreadEvent::DelegationResultProduced { result, .. } => {
            require_no_command(envelope)?;
            let seed = snapshot.agent_context_seed.as_ref().ok_or_else(|| {
                CoreError::Journal(
                    "only an Agent child Thread can produce a delegation result".into(),
                )
            })?;
            validate_delegation_result(&snapshot, result)?;
            if result.delegation_id != seed.delegation_id
                || result.child_thread_id != snapshot.thread_id
            {
                return Err(CoreError::Journal(
                    "delegation result does not match the child context seed".into(),
                ));
            }
            if snapshot
                .produced_delegation_results
                .insert(result.delegation_id.clone(), result.as_ref().clone())
                .is_some()
            {
                return Err(CoreError::Journal(format!(
                    "delegation result was already produced: {}",
                    result.delegation_id
                )));
            }
        }
        ThreadEvent::DelegationResultReceived { result, .. } => {
            require_no_command(envelope)?;
            let delegation = snapshot
                .delegations
                .get(&result.delegation_id)
                .ok_or_else(|| CoreError::NotFound(result.delegation_id.to_string()))?;
            if delegation.child_thread_id.as_ref() != Some(&result.child_thread_id) {
                return Err(CoreError::Journal(
                    "delegation result child does not match the started delegation".into(),
                ));
            }
            if snapshot
                .received_delegation_results
                .insert(result.delegation_id.clone(), result.as_ref().clone())
                .is_some()
            {
                return Err(CoreError::Journal(format!(
                    "delegation result was already received: {}",
                    result.delegation_id
                )));
            }
        }
        ThreadEvent::AgentMessageSent { message, .. } => {
            require_no_command(envelope)?;
            validate_agent_message(message)?;
            if message.sender_thread_id != snapshot.thread_id
                || message.sender_sequence != snapshot.sequence
            {
                return Err(CoreError::Journal(
                    "sent Agent message must anchor the current sender Thread sequence".into(),
                ));
            }
            if snapshot
                .sent_agent_messages
                .insert(message.message_id.clone(), message.as_ref().clone())
                .is_some()
            {
                return Err(CoreError::Journal(format!(
                    "Agent message was already sent: {}",
                    message.message_id
                )));
            }
        }
        ThreadEvent::AgentMessageReceived { message, .. } => {
            require_no_command(envelope)?;
            validate_agent_message(message)?;
            if message.receiver_thread_id != snapshot.thread_id {
                return Err(CoreError::Journal(
                    "received Agent message targets another Thread".into(),
                ));
            }
            if snapshot
                .received_agent_messages
                .insert(message.message_id.clone(), message.as_ref().clone())
                .is_some()
            {
                return Err(CoreError::Journal(format!(
                    "Agent message was already received: {}",
                    message.message_id
                )));
            }
        }
        ThreadEvent::AgentJoinRequested { join, .. } => {
            require_no_command(envelope)?;
            validate_agent_join(&snapshot, join)?;
            if snapshot
                .agent_joins
                .insert(join.join_id.clone(), join.as_ref().clone())
                .is_some()
            {
                return Err(CoreError::Journal(format!(
                    "Agent join already exists: {}",
                    join.join_id
                )));
            }
        }
        ThreadEvent::AgentJoinSatisfied {
            join_id,
            satisfied_by,
            ..
        } => {
            require_no_command(envelope)?;
            let expected = {
                let join = snapshot
                    .agent_joins
                    .get(join_id)
                    .ok_or_else(|| CoreError::NotFound(join_id.to_string()))?;
                if join.status != AgentJoinStatus::Waiting {
                    return Err(CoreError::Journal(format!(
                        "Agent join is already terminal: {join_id}"
                    )));
                }
                satisfied_agent_join(&snapshot, join).ok_or_else(|| {
                    CoreError::Journal(format!("Agent join is not yet satisfied: {join_id}"))
                })?
            };
            if expected != *satisfied_by {
                return Err(CoreError::Journal(
                    "Agent join satisfaction does not match durable delegation results".into(),
                ));
            }
            let join = snapshot
                .agent_joins
                .get_mut(join_id)
                .expect("join was validated above");
            join.status = AgentJoinStatus::Satisfied;
            join.satisfied_by = satisfied_by.clone();
        }
    }
    snapshot
        .event_digests
        .insert(envelope.sequence, event_digest(&envelope.event)?);
    snapshot.sequence = envelope.sequence;
    Ok(snapshot)
}

fn validate_agent_context_seed(seed: &AgentContextSeed) -> Result<(), CoreError> {
    if seed.parent_sequence == 0
        || seed.task.title.trim().is_empty()
        || seed.task.instructions.trim().is_empty()
        || seed.role.name.trim().is_empty()
        || seed.role.instructions.trim().is_empty()
        || seed.policy_ceiling.policy_revision.trim().is_empty()
    {
        return Err(CoreError::Journal(
            "Agent context seed contains an empty required field".into(),
        ));
    }
    if let zeta_protocol::AgentContextMode::ForkedPrefix {
        selection: zeta_protocol::ForkedAgentContext::LastTurns { count },
    } = seed.inheritance
        && count == 0
    {
        return Err(CoreError::Journal(
            "forked Agent context must select at least one Turn".into(),
        ));
    }
    match &seed.inheritance {
        zeta_protocol::AgentContextMode::Fresh if !seed.materialized_context.is_empty() => {
            return Err(CoreError::Journal(
                "Fresh Agent context cannot contain inherited material".into(),
            ));
        }
        zeta_protocol::AgentContextMode::Selected { sources }
            if sources.len() != seed.materialized_context.len()
                || sources
                    .iter()
                    .zip(&seed.materialized_context)
                    .any(|(source, materialized)| source != &materialized.source) =>
        {
            return Err(CoreError::Journal(
                "Selected Agent sources do not match their materialized context".into(),
            ));
        }
        zeta_protocol::AgentContextMode::Fresh
        | zeta_protocol::AgentContextMode::Selected { .. }
        | zeta_protocol::AgentContextMode::ForkedPrefix { .. } => {}
    }
    for materialized in &seed.materialized_context {
        let encoded = serde_json::to_vec(&materialized.content).map_err(|error| {
            CoreError::Journal(format!("cannot encode materialized Agent context: {error}"))
        })?;
        if zeta_protocol::ContentDigest::sha256(&encoded) != materialized.content_digest {
            return Err(CoreError::Journal(
                "materialized Agent context content digest does not match".into(),
            ));
        }
    }
    validate_context_seed_digest(seed)
}

fn validate_agent_join(snapshot: &ThreadSnapshot, join: &AgentJoin) -> Result<(), CoreError> {
    if join.parent_thread_id != snapshot.thread_id
        || join.status != AgentJoinStatus::Waiting
        || !join.satisfied_by.is_empty()
        || join.delegations.is_empty()
    {
        return Err(CoreError::Journal(
            "Agent join must start as a non-empty waiting parent-side fact".into(),
        ));
    }
    let mut unique = BTreeSet::new();
    if join.delegations.iter().any(|delegation_id| {
        !unique.insert(delegation_id.clone()) || !snapshot.delegations.contains_key(delegation_id)
    }) {
        return Err(CoreError::Journal(
            "Agent join targets must be unique delegations owned by the parent".into(),
        ));
    }
    match &join.policy {
        AgentJoinPolicy::Quorum { count }
            if *count == 0
                || usize::try_from(*count).map_or(true, |count| count > unique.len()) =>
        {
            Err(CoreError::Journal(
                "Agent join quorum must fit its frozen target set".into(),
            ))
        }
        AgentJoinPolicy::Explicit { delegations } if delegations != &join.delegations => {
            Err(CoreError::Journal(
                "explicit Agent join policy must match its frozen target set".into(),
            ))
        }
        AgentJoinPolicy::All
        | AgentJoinPolicy::Any
        | AgentJoinPolicy::Quorum { .. }
        | AgentJoinPolicy::Explicit { .. } => Ok(()),
    }
}

pub(crate) fn satisfied_agent_join(
    snapshot: &ThreadSnapshot,
    join: &AgentJoin,
) -> Option<Vec<DelegationId>> {
    let completed = join
        .delegations
        .iter()
        .filter(|delegation_id| {
            snapshot
                .received_delegation_results
                .contains_key(*delegation_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let required = match &join.policy {
        AgentJoinPolicy::All | AgentJoinPolicy::Explicit { .. } => join.delegations.len(),
        AgentJoinPolicy::Any => 1,
        AgentJoinPolicy::Quorum { count } => usize::try_from(*count).ok()?,
    };
    (completed.len() >= required).then(|| completed.into_iter().take(required).collect())
}

fn validate_delegation_result(
    snapshot: &ThreadSnapshot,
    result: &DelegationResult,
) -> Result<(), CoreError> {
    if result.summary.trim().is_empty()
        || result.source_range.start_sequence == 0
        || result.source_range.start_sequence > result.source_range.end_sequence
        || result.source_range.end_sequence > snapshot.sequence
    {
        return Err(CoreError::Journal(
            "delegation result must contain a bounded summary and available source range".into(),
        ));
    }
    validate_delegation_result_digest(result)
}

fn validate_agent_message(message: &AgentMessage) -> Result<(), CoreError> {
    if message.sender_thread_id == message.receiver_thread_id || message.sender_sequence == 0 {
        return Err(CoreError::Journal(
            "Agent message must cross two Threads from an available sender sequence".into(),
        ));
    }
    match &message.content {
        zeta_protocol::AgentMessageContent::Instruction { text } if text.trim().is_empty() => Err(
            CoreError::Journal("Agent instruction message must not be empty".into()),
        ),
        zeta_protocol::AgentMessageContent::Instruction { .. }
        | zeta_protocol::AgentMessageContent::Result { .. } => Ok(()),
    }
}

fn turn_skill_activations_match(
    input: &[zeta_protocol::UserInput],
    activated_skills: &[FrozenSkillActivation],
) -> bool {
    let selected = input.iter().filter_map(|input| match input {
        zeta_protocol::UserInput::Skill { skill } => Some(skill),
        zeta_protocol::UserInput::Text { .. }
        | zeta_protocol::UserInput::Context { .. }
        | zeta_protocol::UserInput::ImageAttachment { .. }
        | zeta_protocol::UserInput::Image { .. }
        | zeta_protocol::UserInput::LocalImage { .. }
        | zeta_protocol::UserInput::Mention { .. } => None,
    });
    selected
        .zip(activated_skills.iter().filter(|activation| {
            activation.reason == zeta_protocol::SkillActivationReason::Explicit
        }))
        .all(|(selected, activated)| {
            selected.id == activated.id
                && activated.reason == zeta_protocol::SkillActivationReason::Explicit
                && match &selected.version {
                    zeta_protocol::SkillVersionSelector::FollowLatest => true,
                    zeta_protocol::SkillVersionSelector::PinnedDigest { digest } => {
                        digest == &activated.content_digest
                    }
                }
        })
        && input
            .iter()
            .filter(|input| matches!(input, zeta_protocol::UserInput::Skill { .. }))
            .count()
            == activated_skills
                .iter()
                .filter(|activation| {
                    activation.reason == zeta_protocol::SkillActivationReason::Explicit
                })
                .count()
}

fn validate_steered_items(
    snapshot: &ThreadSnapshot,
    turn_id: &TurnId,
    input: &[zeta_protocol::UserInput],
    item_ids: &[ItemId],
    marker_sequence: u64,
) -> Result<(), CoreError> {
    if input.is_empty() || input.len() != item_ids.len() {
        return Err(CoreError::Journal(
            "Turn steering must bind every non-empty input to one durable Item".into(),
        ));
    }
    let first_sequence = marker_sequence
        .checked_sub(item_ids.len() as u64)
        .ok_or_else(|| CoreError::Journal("Turn steering Item sequence underflow".into()))?;
    let mut unique = BTreeSet::new();
    for (index, (input, item_id)) in input.iter().zip(item_ids).enumerate() {
        if !unique.insert(item_id) {
            return Err(CoreError::Journal(
                "Turn steering Item IDs must be unique".into(),
            ));
        }
        let item = snapshot
            .items
            .iter()
            .find(|item| item.item_id() == item_id)
            .ok_or_else(|| CoreError::Journal(format!("steered Item does not exist: {item_id}")))?;
        if item.turn_id() != turn_id
            || snapshot.item_sequences.get(item_id).copied() != Some(first_sequence + index as u64)
        {
            return Err(CoreError::Journal(
                "Turn steering Items must be the immediately preceding ordered Turn Items".into(),
            ));
        }
        let matches = match (input, item) {
            (
                zeta_protocol::UserInput::Text { text: input_text },
                ThreadItem::UserMessage {
                    text: item_text, ..
                },
            ) => input_text == item_text,
            (
                zeta_protocol::UserInput::ImageAttachment {
                    attachment: input_attachment,
                },
                ThreadItem::UserImageAttachment {
                    attachment: item_attachment,
                    ..
                },
            ) => input_attachment == item_attachment,
            _ => false,
        };
        if !matches {
            return Err(CoreError::Journal(
                "Turn steering command input does not match its durable Items".into(),
            ));
        }
    }
    Ok(())
}

fn import_history(
    snapshot: &mut ThreadSnapshot,
    source_thread_id: &ThreadId,
    before_turn_id: &TurnId,
    turns: &[Turn],
) -> Result<(), CoreError> {
    if source_thread_id == &snapshot.thread_id {
        return Err(CoreError::Journal(
            "imported Thread history must come from another Thread".into(),
        ));
    }
    if snapshot.sequence != 1 || !snapshot.turns.is_empty() || !snapshot.items.is_empty() {
        return Err(CoreError::Journal(
            "Thread history can only be imported immediately after creation".into(),
        ));
    }

    let mut turn_ids = BTreeSet::new();
    let mut item_ids = BTreeSet::new();
    let mut tool_calls = BTreeSet::new();
    let mut tool_results = BTreeSet::new();
    for turn in turns {
        if &turn.turn_id == before_turn_id {
            return Err(CoreError::Journal(
                "rewind checkpoint must be excluded from imported history".into(),
            ));
        }
        if !matches!(
            turn.status,
            TurnStatus::Completed | TurnStatus::Failed | TurnStatus::Interrupted
        ) || turn.pending_interaction.is_some()
        {
            return Err(CoreError::Journal(
                "only terminal Turns can be imported into rewound history".into(),
            ));
        }
        if !turn_ids.insert(turn.turn_id.clone()) {
            return Err(CoreError::Journal(format!(
                "imported Turn already exists: {}",
                turn.turn_id
            )));
        }
        if let Some(tool_profile) = &turn.tool_profile {
            crate::tool_profile::validate_tool_profile_snapshot(tool_profile)
                .map_err(CoreError::Journal)?;
        }
        if let Some(plan) = &turn.plan {
            crate::turn::validate_plan_update(plan).map_err(CoreError::Journal)?;
        }
        for item in &turn.items {
            if item.turn_id() != &turn.turn_id {
                return Err(CoreError::Journal(
                    "imported Item Turn identity does not match its Turn".into(),
                ));
            }
            if !item_ids.insert(item.item_id().clone()) {
                return Err(CoreError::Journal(format!(
                    "imported Item already exists: {}",
                    item.item_id()
                )));
            }
            match item {
                ThreadItem::ToolCall { tool_call_id, .. } => {
                    if !tool_calls.insert((turn.turn_id.clone(), tool_call_id.clone())) {
                        return Err(CoreError::Journal(format!(
                            "imported Tool Call already exists: {tool_call_id}"
                        )));
                    }
                }
                ThreadItem::ToolResult { tool_call_id, .. } => {
                    let identity = (turn.turn_id.clone(), tool_call_id.clone());
                    if !tool_calls.contains(&identity) {
                        return Err(CoreError::Journal(format!(
                            "imported Tool Result references an unknown Tool Call: {tool_call_id}"
                        )));
                    }
                    if !tool_results.insert(identity) {
                        return Err(CoreError::Journal(format!(
                            "imported Tool Result already exists: {tool_call_id}"
                        )));
                    }
                }
                ThreadItem::UserMessage { .. }
                | ThreadItem::UserContext { .. }
                | ThreadItem::UserImage { .. }
                | ThreadItem::UserImageAttachment { .. }
                | ThreadItem::AgentMessage { .. }
                | ThreadItem::Reasoning { .. }
                | ThreadItem::Plan { .. } => {}
            }
        }
    }

    snapshot.turns = turns
        .iter()
        .map(|turn| TurnSnapshot {
            turn_id: turn.turn_id.clone(),
            status: turn.status,
            model: turn.model.clone(),
            policy_revision: "imported-history-policy".into(),
            approval_mode: ApprovalMode::AskPermissions,
            tool_mode: turn.tool_mode,
            activated_skills: Vec::new(),
            failure: turn.error.clone(),
            pending_interaction: None,
            execution_backend_attempt: None,
            tool_profile: turn.tool_profile.clone(),
            plan: turn.plan.clone(),
            usage: ModelUsageSummary::default(),
        })
        .collect();
    snapshot.items = turns
        .iter()
        .flat_map(|turn| turn.items.iter().cloned())
        .collect();
    snapshot.context_overflow_recoveries.clear();
    Ok(())
}

fn validate_context_checkpoint(
    snapshot: &ThreadSnapshot,
    checkpoint: &ContextCheckpoint,
) -> Result<(), CoreError> {
    if checkpoint.source_thread_id != snapshot.thread_id {
        return Err(CoreError::Journal(
            "context checkpoint source Thread does not match its event stream".into(),
        ));
    }
    if checkpoint.covered.start_sequence != 1
        || checkpoint.covered.end_sequence < checkpoint.covered.start_sequence
        || checkpoint.covered.end_sequence > snapshot.sequence
    {
        return Err(CoreError::Journal(
            "context checkpoint must cover an available non-empty Thread prefix".into(),
        ));
    }
    if checkpoint.summary.trim().is_empty()
        || checkpoint.schema_revision.trim().is_empty()
        || checkpoint.prompt_revision.trim().is_empty()
        || checkpoint.context_policy_revision.trim().is_empty()
    {
        return Err(CoreError::Journal(
            "context checkpoint summary and revision identities must not be empty".into(),
        ));
    }
    if snapshot
        .context_checkpoints
        .iter()
        .any(|existing| existing.checkpoint_id == checkpoint.checkpoint_id)
        || snapshot
            .context_checkpoints
            .last()
            .is_some_and(|existing| existing.covered.end_sequence > checkpoint.covered.end_sequence)
    {
        return Err(CoreError::Journal(
            "context checkpoints must have unique identities and not retreat the durable prefix"
                .into(),
        ));
    }

    let expected_items = snapshot
        .items
        .iter()
        .filter(|item| {
            snapshot
                .item_sequences
                .get(item.item_id())
                .is_some_and(|sequence| *sequence <= checkpoint.covered.end_sequence)
        })
        .map(|item| item.item_id().clone())
        .collect::<Vec<_>>();
    if checkpoint.referenced_items != expected_items {
        return Err(CoreError::Journal(
            "context checkpoint Item provenance does not match its covered Thread prefix".into(),
        ));
    }

    let expected_digest = snapshot.context_source_digest(checkpoint.covered)?;
    if checkpoint.source_digest != expected_digest {
        return Err(CoreError::Journal(
            "context checkpoint source digest does not match its covered Thread prefix".into(),
        ));
    }
    Ok(())
}

fn event_digest(event: &ThreadEvent) -> Result<String, CoreError> {
    let mut value = serde_json::to_value(event).map_err(|error| {
        CoreError::Journal(format!("failed to serialize Thread event: {error}"))
    })?;
    canonicalize_json(&mut value);
    let encoded = serde_json::to_vec(&value).map_err(|error| {
        CoreError::Journal(format!("failed to encode canonical Thread event: {error}"))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(encoded)))
}

fn canonicalize_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                canonicalize_json(value);
            }
        }
        serde_json::Value::Object(object) => {
            let mut entries = std::mem::take(object).into_iter().collect::<Vec<_>>();
            for (_, value) in &mut entries {
                canonicalize_json(value);
            }
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            object.extend(entries);
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
    }
}

pub(crate) fn validate_agent_request(request: &AgentRequest) -> Result<(), String> {
    if let AgentRequest::DynamicTool { call } = request {
        if !is_sha256_hex(&call.definition_digest) {
            return Err("dynamic tool definition digest must be a SHA-256 hex digest".into());
        }
        return Ok(());
    }
    let AgentRequest::Approval { request } = request else {
        return Ok(());
    };
    if !is_sha256_hex(&request.action_digest) {
        return Err("approval action digest must be a SHA-256 hex digest".into());
    }
    if request.policy_revision.trim().is_empty() {
        return Err("approval policy revision must not be empty".into());
    }
    if request.reason.trim().is_empty() {
        return Err("approval reason must not be empty".into());
    }
    if let Some(denial) = &request.sandbox_denial
        && (denial.replay_safety() != zeta_protocol::ToolReplaySafety::SafeToRetry
            || denial.reason().trim().is_empty())
    {
        return Err("sandbox escalation approval requires a safe-to-retry denial".into());
    }
    if request.capabilities.is_empty() {
        return Err("approval capabilities must not be empty".into());
    }
    if request
        .capabilities
        .iter()
        .any(|capability| capability.scope.trim().is_empty())
    {
        return Err("approval capability scope must not be empty".into());
    }
    let unique = request.capabilities.iter().collect::<BTreeSet<_>>();
    if unique.len() != request.capabilities.len() {
        return Err("approval capabilities must not contain duplicates".into());
    }
    if !request
        .capabilities
        .windows(2)
        .all(|pair| pair[0] < pair[1])
    {
        return Err("approval capabilities must use canonical order".into());
    }
    Ok(())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn create_turn(snapshot: &mut ThreadSnapshot, turn: TurnSnapshot) -> Result<(), CoreError> {
    if snapshot
        .turns
        .iter()
        .any(|existing| existing.turn_id == turn.turn_id)
    {
        return Err(CoreError::Journal(format!(
            "Turn already exists: {}",
            turn.turn_id
        )));
    }
    snapshot.turns.push(turn);
    Ok(())
}

fn validate_goal_identity(
    snapshot: &ThreadSnapshot,
    thread_id: &ThreadId,
    goal: &zeta_protocol::ThreadGoal,
) -> Result<(), CoreError> {
    if thread_id != &snapshot.thread_id || goal.thread_id != snapshot.thread_id {
        return Err(CoreError::Journal(
            "Goal event Thread identity does not match the rollout".into(),
        ));
    }
    goal.validate().map_err(CoreError::Journal)
}

fn goal_token_delta(usage: Option<&zeta_protocol::ModelUsage>) -> u64 {
    let Some(usage) = usage else {
        return 0;
    };
    let uncached_input = match (usage.input_tokens, usage.cached_input_tokens) {
        (Some(input), Some(cached)) => input.saturating_sub(cached),
        // Without both provider values, the uncached input amount is unknown. Do not invent a
        // precise number; output tokens, when present, remain independently attributable.
        _ => 0,
    };
    uncached_input.saturating_add(usage.output_tokens.unwrap_or_default())
}

fn require_no_command(envelope: &StoredEvent) -> Result<(), CoreError> {
    if envelope.command.is_some() {
        Err(CoreError::Journal(
            "this Thread event must not carry a command receipt".into(),
        ))
    } else {
        Ok(())
    }
}

fn transition_turn(
    snapshot: &mut ThreadSnapshot,
    turn_id: &TurnId,
    next: TurnStatus,
    failure: Option<StableTurnError>,
) -> Result<(), CoreError> {
    let turn = find_turn_mut(snapshot, turn_id)?;
    turn.status = transition_turn_status(turn.status, next)?;
    turn.failure = failure;
    Ok(())
}

fn find_turn<'a>(
    snapshot: &'a ThreadSnapshot,
    turn_id: &TurnId,
) -> Result<&'a TurnSnapshot, CoreError> {
    snapshot
        .turns
        .iter()
        .find(|turn| turn.turn_id == *turn_id)
        .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))
}

fn find_turn_mut<'a>(
    snapshot: &'a mut ThreadSnapshot,
    turn_id: &TurnId,
) -> Result<&'a mut TurnSnapshot, CoreError> {
    snapshot
        .turns
        .iter_mut()
        .find(|turn| turn.turn_id == *turn_id)
        .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))
}

fn pending_interaction(
    snapshot: &ThreadSnapshot,
    turn_id: &TurnId,
    request_id: &RequestId,
) -> Result<TurnInteraction, CoreError> {
    let interaction = find_turn(snapshot, turn_id)?
        .pending_interaction
        .as_ref()
        .ok_or_else(|| CoreError::Journal("Turn has no outstanding interaction".into()))?;
    if interaction.request_id != *request_id {
        return Err(CoreError::Journal(
            "interaction response does not match the outstanding request ID".into(),
        ));
    }
    Ok(interaction.clone())
}

fn waiting_status_for(request: &AgentRequest) -> TurnStatus {
    match request {
        AgentRequest::Approval { .. } => TurnStatus::WaitingForApproval,
        AgentRequest::UserInput { .. } => TurnStatus::WaitingForUserInput,
        AgentRequest::DynamicTool { .. } => TurnStatus::WaitingForCapability,
    }
}

fn resolution_command(
    turn_id: &TurnId,
    request_id: &RequestId,
    response: &AgentResponse,
) -> ThreadCommand {
    match response {
        AgentResponse::Approval { response } => ThreadCommand::ResolveApproval {
            turn_id: turn_id.clone(),
            request_id: request_id.clone(),
            response: response.clone(),
        },
        AgentResponse::UserInput { response } => ThreadCommand::ResolveUserInput {
            turn_id: turn_id.clone(),
            request_id: request_id.clone(),
            response: response.clone(),
        },
        AgentResponse::DynamicTool { response } => ThreadCommand::ResolveDynamicTool {
            turn_id: turn_id.clone(),
            request_id: request_id.clone(),
            response: response.clone(),
        },
    }
}

#[cfg(test)]
#[path = "thread_reducer_tests.rs"]
mod tests;
