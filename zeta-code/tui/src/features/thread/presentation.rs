use zeta_protocol::StableTurnError;
use zeta_protocol::StableTurnErrorCode;
use zeta_protocol::ThreadItem;
use zeta_protocol::Turn;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TurnActivity {
    Working,
    WaitingForApproval,
    WaitingForUserInput,
    WaitingForCapability,
    Cancelling,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ActiveTurnUpdate {
    ActivityChanged(TurnActivity),
    Completed,
    Failed(String),
    Interrupted,
    Unchanged,
}

pub(crate) fn evaluate_active_turn(
    active_turn: &mut Option<TurnId>,
    turns: &[Turn],
) -> ActiveTurnUpdate {
    let Some(turn_id) = active_turn.as_ref() else {
        return ActiveTurnUpdate::Unchanged;
    };
    let Some(turn) = turns.iter().find(|turn| &turn.turn_id == turn_id) else {
        return ActiveTurnUpdate::Unchanged;
    };

    match turn.status {
        TurnStatus::Completed => {
            *active_turn = None;
            if turn
                .items
                .iter()
                .any(|item| matches!(item, ThreadItem::AgentMessage { .. }))
            {
                ActiveTurnUpdate::Completed
            } else {
                ActiveTurnUpdate::Failed("turn completed without an agent message".into())
            }
        }
        TurnStatus::Failed => {
            *active_turn = None;
            ActiveTurnUpdate::Failed(turn.error.as_ref().map(present_turn_error).unwrap_or_else(
                || "The request stopped before Zeta could finish. Please try again.".into(),
            ))
        }
        TurnStatus::Interrupted => {
            *active_turn = None;
            ActiveTurnUpdate::Interrupted
        }
        TurnStatus::WaitingForApproval => {
            ActiveTurnUpdate::ActivityChanged(TurnActivity::WaitingForApproval)
        }
        TurnStatus::WaitingForUserInput => {
            ActiveTurnUpdate::ActivityChanged(TurnActivity::WaitingForUserInput)
        }
        TurnStatus::WaitingForCapability => {
            ActiveTurnUpdate::ActivityChanged(TurnActivity::WaitingForCapability)
        }
        TurnStatus::Created | TurnStatus::Running => {
            ActiveTurnUpdate::ActivityChanged(TurnActivity::Working)
        }
        TurnStatus::Cancelling => ActiveTurnUpdate::ActivityChanged(TurnActivity::Cancelling),
    }
}

pub(crate) fn present_turn_error(error: &StableTurnError) -> String {
    match error.code {
        StableTurnErrorCode::ModelInvocationFailed => {
            "Zeta couldn't reach the configured model. Check the model provider and credentials, \
             then try again."
                .into()
        }
        StableTurnErrorCode::CompletionPersistenceFailed => {
            "Zeta generated a response but couldn't save it. Please try again.".into()
        }
    }
}
