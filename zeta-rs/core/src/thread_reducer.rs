use crate::CoreError;
use crate::state::transition_turn_status;
use sha2::Digest;
use sha2::Sha256;
use std::collections::{BTreeMap, BTreeSet};
use zeta_history::StoredEvent;
use zeta_history::ThreadCommandReceipt;
use zeta_history::supports_stored_event_schema_version;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentResponse;
use zeta_protocol::ContextCheckpoint;
use zeta_protocol::ContextSourceDigest;
use zeta_protocol::ContextSourceRange;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::ItemId;
use zeta_protocol::ModelRef;
use zeta_protocol::RequestId;
use zeta_protocol::SessionId;
use zeta_protocol::StableTurnError;
use zeta_protocol::Thread;
use zeta_protocol::ThreadCommand;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadStatus;
use zeta_protocol::ToolCallId;
use zeta_protocol::Turn;
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
    pub sequence: u64,
    pub turns: Vec<TurnSnapshot>,
    pub items: Vec<ThreadItem>,
    pub context_checkpoints: Vec<ContextCheckpoint>,
    pub item_sequences: BTreeMap<ItemId, u64>,
    pub event_digests: BTreeMap<u64, String>,
    pub commands: Vec<ThreadCommandSnapshot>,
    pub seen_interaction_ids: BTreeSet<RequestId>,
    pub resolved_interactions: Vec<ResolvedTurnInteraction>,
    pub started_tool_calls: BTreeSet<ToolCallId>,
    pub tool_execution_starts: BTreeMap<ToolCallId, ToolExecutionStartSnapshot>,
    pub escalated_tool_calls: BTreeSet<ToolCallId>,
}

impl ThreadSnapshot {
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
            turns: self
                .turns
                .iter()
                .map(|turn| Turn {
                    turn_id: turn.turn_id.clone(),
                    status: turn.status,
                    model: turn.model.clone(),
                    items: self
                        .items
                        .iter()
                        .filter(|item| item.turn_id() == &turn.turn_id)
                        .cloned()
                        .collect(),
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
    pub activated_skills: Vec<FrozenSkillActivation>,
    pub failure: Option<StableTurnError>,
    pub pending_interaction: Option<TurnInteraction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolExecutionStartSnapshot {
    pub action_digest: String,
    pub policy_revision: String,
    pub authority: zeta_protocol::ToolExecutionAuthority,
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
                    sequence: envelope.sequence,
                    turns: Vec::new(),
                    items: Vec::new(),
                    context_checkpoints: Vec::new(),
                    item_sequences: BTreeMap::new(),
                    event_digests,
                    seen_interaction_ids: BTreeSet::new(),
                    resolved_interactions: Vec::new(),
                    started_tool_calls: BTreeSet::new(),
                    tool_execution_starts: BTreeMap::new(),
                    escalated_tool_calls: BTreeSet::new(),
                    commands: {
                        require_no_command(envelope)?;
                        Vec::new()
                    },
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
        ThreadEvent::TurnAccepted {
            turn_id,
            model,
            policy_revision,
            activated_skills,
            ..
        } => {
            if policy_revision.trim().is_empty() {
                return Err(CoreError::Journal(
                    "Turn policy revision must not be empty".into(),
                ));
            }
            create_turn(
                &mut snapshot,
                turn_id,
                model.clone(),
                policy_revision.clone(),
                activated_skills.clone(),
            )?;
            let receipt = envelope.command.clone().ok_or_else(|| {
                CoreError::Journal("Turn acceptance requires a command receipt".into())
            })?;
            let matching_start = match &receipt.command {
                ThreadCommand::StartTurn {
                    model: command_model,
                    input,
                    ..
                } => {
                    command_model == model && turn_skill_activations_match(input, activated_skills)
                }
                ThreadCommand::StartShellTurn { .. } => {
                    model.is_none() && activated_skills.is_empty()
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
                | ThreadItem::UserImage { .. }
                | ThreadItem::AgentMessage { .. }
                | ThreadItem::Reasoning { .. }
                | ThreadItem::Plan { .. } => {}
            }
            snapshot.items.push(item.clone());
            snapshot
                .item_sequences
                .insert(item.item_id().clone(), envelope.sequence);
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
    }
    snapshot
        .event_digests
        .insert(envelope.sequence, event_digest(&envelope.event)?);
    snapshot.sequence = envelope.sequence;
    Ok(snapshot)
}

fn turn_skill_activations_match(
    input: &[zeta_protocol::UserInput],
    activated_skills: &[FrozenSkillActivation],
) -> bool {
    let selected = input.iter().filter_map(|input| match input {
        zeta_protocol::UserInput::Skill { skill } => Some(skill),
        zeta_protocol::UserInput::Text { .. }
        | zeta_protocol::UserInput::Image { .. }
        | zeta_protocol::UserInput::LocalImage { .. }
        | zeta_protocol::UserInput::Mention { .. } => None,
    });
    selected.zip(activated_skills).all(|(selected, activated)| {
        selected.id == activated.id
            && activated.reason == zeta_protocol::SkillActivationReason::Explicit
            && match &selected.version {
                zeta_protocol::SkillVersionSelector::FollowLatest => true,
                zeta_protocol::SkillVersionSelector::PinnedDigest { digest } => {
                    digest == &activated.content_digest
                }
            }
    }) && input
        .iter()
        .filter(|input| matches!(input, zeta_protocol::UserInput::Skill { .. }))
        .count()
        == activated_skills.len()
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
                | ThreadItem::UserImage { .. }
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
            activated_skills: Vec::new(),
            failure: turn.error.clone(),
            pending_interaction: None,
        })
        .collect();
    snapshot.items = turns
        .iter()
        .flat_map(|turn| turn.items.iter().cloned())
        .collect();
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
    let AgentRequest::Approval { request } = request else {
        return Ok(());
    };
    if request.action_digest.len() != 64
        || !request
            .action_digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
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

fn create_turn(
    snapshot: &mut ThreadSnapshot,
    turn_id: &TurnId,
    model: Option<ModelRef>,
    policy_revision: String,
    activated_skills: Vec<FrozenSkillActivation>,
) -> Result<(), CoreError> {
    if snapshot.turns.iter().any(|turn| turn.turn_id == *turn_id) {
        return Err(CoreError::Journal(format!(
            "Turn already exists: {turn_id}"
        )));
    }
    snapshot.turns.push(TurnSnapshot {
        turn_id: turn_id.clone(),
        status: TurnStatus::Created,
        model,
        policy_revision,
        activated_skills,
        failure: None,
        pending_interaction: None,
    });
    Ok(())
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
