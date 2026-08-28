use super::RpcError;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::turn::TurnStartResult;
use zeta_core::ThreadCommandResult;
use zeta_core::ThreadSnapshot;
use zeta_protocol::CommandId;
use zeta_protocol::ThreadCommand;
use zeta_protocol::ToolMode;
use zeta_protocol::TurnStatus;
use zeta_protocol::UserInput;

/// Replays an accepted start-Turn command without consulting mutable model or Skill authority.
pub(super) fn replayed_result(
    snapshot: &ThreadSnapshot,
    command_id: &CommandId,
    tool_mode: ToolMode,
    input: &[UserInput],
) -> Result<Option<TurnStartResult>, RpcError> {
    let Some(command) = snapshot
        .commands
        .iter()
        .find(|command| &command.receipt.command_id == command_id)
    else {
        return Ok(None);
    };
    let ThreadCommand::StartTurn {
        tool_mode: accepted_tool_mode,
        input: accepted_input,
        ..
    } = &command.receipt.command
    else {
        return Err(RpcError::new(-32004, AppServerErrorName::CommandConflict));
    };
    if *accepted_tool_mode != tool_mode || accepted_input != input {
        return Err(RpcError::new(-32004, AppServerErrorName::CommandConflict));
    }
    accepted_result(snapshot, command)
}

/// Replays the start phase of a Session rewrite using its already frozen Thread command.
///
/// The outer Session command has already checked the caller-owned input and requested tool mode.
/// Mutable server defaults must not turn a valid retry into a conflict after the start committed.
pub(super) fn replayed_rewrite_result(
    snapshot: &ThreadSnapshot,
    command_id: &CommandId,
    input: &[UserInput],
) -> Result<Option<TurnStartResult>, RpcError> {
    let Some(command) = snapshot
        .commands
        .iter()
        .find(|command| &command.receipt.command_id == command_id)
    else {
        return Ok(None);
    };
    let ThreadCommand::StartTurn {
        input: accepted_input,
        ..
    } = &command.receipt.command
    else {
        return Err(RpcError::new(-32004, AppServerErrorName::CommandConflict));
    };
    if accepted_input != input {
        return Err(RpcError::new(-32004, AppServerErrorName::CommandConflict));
    }
    accepted_result(snapshot, command)
}

fn accepted_result(
    snapshot: &ThreadSnapshot,
    command: &zeta_core::ThreadCommandSnapshot,
) -> Result<Option<TurnStartResult>, RpcError> {
    let ThreadCommandResult::TurnAccepted { turn_id } = &command.result else {
        return Err(RpcError::new(-32000, AppServerErrorName::InternalError));
    };
    let turn = snapshot
        .turns
        .iter()
        .find(|turn| &turn.turn_id == turn_id)
        .ok_or_else(|| RpcError::new(-32000, AppServerErrorName::InternalError))?;
    match turn.status {
        TurnStatus::Created
        | TurnStatus::Running
        | TurnStatus::WaitingForApproval
        | TurnStatus::WaitingForUserInput
        | TurnStatus::WaitingForCapability
        | TurnStatus::Completed => Ok(Some(TurnStartResult {
            turn_id: turn_id.clone(),
            sequence: command.response_sequence,
        })),
        TurnStatus::Failed | TurnStatus::Interrupted => Err(RpcError::new(
            -32010,
            AppServerErrorName::CoreOperationFailed,
        )),
        TurnStatus::Cancelling => Err(RpcError::new(-32000, AppServerErrorName::ServerOverloaded)),
    }
}
