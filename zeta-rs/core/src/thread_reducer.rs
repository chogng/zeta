use crate::CoreError;
use crate::state::transition_turn_status;
use std::collections::BTreeSet;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentResponse;
use zeta_protocol::RequestId;
use zeta_protocol::SessionId;
use zeta_protocol::StableTurnError;
use zeta_protocol::Thread;
use zeta_protocol::ThreadCommand;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadStatus;
use zeta_protocol::Turn;
use zeta_protocol::TurnId;
use zeta_protocol::TurnInteraction;
use zeta_protocol::TurnStatus;
use zeta_thread_store::{CURRENT_STORED_EVENT_SCHEMA_VERSION, StoredEvent, ThreadCommandReceipt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadSnapshot {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub title: String,
    pub sequence: u64,
    pub turns: Vec<TurnSnapshot>,
    pub items: Vec<ThreadItem>,
    pub commands: Vec<ThreadCommandSnapshot>,
    pub seen_interaction_ids: BTreeSet<RequestId>,
}

impl ThreadSnapshot {
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnSnapshot {
    pub turn_id: TurnId,
    pub status: TurnStatus,
    pub failure: Option<StableTurnError>,
    pub pending_interaction: Option<TurnInteraction>,
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
    if envelope.schema_version != CURRENT_STORED_EVENT_SCHEMA_VERSION {
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
            } => Ok(ThreadSnapshot {
                session_id: session_id.clone(),
                thread_id: thread_id.clone(),
                title: title.clone(),
                sequence: envelope.sequence,
                turns: Vec::new(),
                items: Vec::new(),
                seen_interaction_ids: BTreeSet::new(),
                commands: {
                    require_no_command(envelope)?;
                    Vec::new()
                },
            }),
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
        ThreadEvent::TurnAccepted { turn_id, .. } => {
            create_turn(&mut snapshot, turn_id)?;
            let receipt = envelope.command.clone().ok_or_else(|| {
                CoreError::Journal("Turn acceptance requires a command receipt".into())
            })?;
            if !matches!(receipt.command, ThreadCommand::StartTurn { .. }) {
                return Err(CoreError::Journal(
                    "Turn acceptance requires a start-Turn command".into(),
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
                | ThreadItem::AgentMessage { .. }
                | ThreadItem::Reasoning { .. }
                | ThreadItem::Plan { .. } => {}
            }
            snapshot.items.push(item.clone());
        }
        ThreadEvent::InteractionRequested {
            turn_id,
            interaction,
            ..
        } => {
            require_no_command(envelope)?;
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
            snapshot.commands.push(ThreadCommandSnapshot {
                receipt,
                result: ThreadCommandResult::InteractionResolved {
                    turn_id: turn_id.clone(),
                    request_id: request_id.clone(),
                },
                response_sequence: envelope.sequence,
            });
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
    snapshot.sequence = envelope.sequence;
    Ok(snapshot)
}

fn create_turn(snapshot: &mut ThreadSnapshot, turn_id: &TurnId) -> Result<(), CoreError> {
    if snapshot.turns.iter().any(|turn| turn.turn_id == *turn_id) {
        return Err(CoreError::Journal(format!(
            "Turn already exists: {turn_id}"
        )));
    }
    snapshot.turns.push(TurnSnapshot {
        turn_id: turn_id.clone(),
        status: TurnStatus::Created,
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
