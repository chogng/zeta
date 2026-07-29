use super::{AgentOutcome, AgentOutcomeStatus};
use zeta_protocol::{SessionId, ThreadId, ThreadItem, Turn, TurnStatus};

pub(super) fn terminal_outcome(
    invocation_id: &str,
    session_id: &SessionId,
    thread_id: &ThreadId,
    turn: &Turn,
) -> Option<AgentOutcome> {
    let (status, content) = match turn.status {
        TurnStatus::Completed => (
            AgentOutcomeStatus::Completed,
            turn.items
                .iter()
                .rev()
                .find_map(|item| match item {
                    ThreadItem::AgentMessage { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
        ),
        TurnStatus::Failed => (
            AgentOutcomeStatus::Failed,
            turn.error
                .as_ref()
                .map(|error| error.message.clone())
                .unwrap_or_else(|| "Turn failed".into()),
        ),
        TurnStatus::Interrupted => (
            AgentOutcomeStatus::Interrupted,
            "Turn was interrupted".into(),
        ),
        TurnStatus::Created
        | TurnStatus::Running
        | TurnStatus::WaitingForApproval
        | TurnStatus::WaitingForUserInput
        | TurnStatus::WaitingForCapability
        | TurnStatus::Cancelling => return None,
    };
    Some(AgentOutcome {
        invocation_id: invocation_id.into(),
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        turn_id: turn.turn_id.clone(),
        status,
        content,
    })
}

pub(super) fn waiting_outcome(
    invocation_id: &str,
    session_id: &SessionId,
    thread_id: &ThreadId,
    turn: &Turn,
) -> Option<AgentOutcome> {
    let (status, content) = match turn.status {
        TurnStatus::WaitingForApproval => (
            AgentOutcomeStatus::WaitingForApproval,
            "Turn is waiting for approval",
        ),
        TurnStatus::WaitingForUserInput => (
            AgentOutcomeStatus::WaitingForUserInput,
            "Turn is waiting for user input",
        ),
        TurnStatus::WaitingForCapability => (
            AgentOutcomeStatus::WaitingForCapability,
            "Turn is waiting for a capability",
        ),
        _ => return None,
    };
    Some(AgentOutcome {
        invocation_id: invocation_id.into(),
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        turn_id: turn.turn_id.clone(),
        status,
        content: content.into(),
    })
}
