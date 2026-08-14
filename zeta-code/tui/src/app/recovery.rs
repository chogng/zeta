use zeta_app_server_client::ConnectionCloseReason;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

use crate::TuiConnectionLossKind;
use crate::TuiExit;
use crate::TuiRecoveryState;
use crate::client::ClientEvent;

pub(super) fn continue_or_exit(
    event: ClientEvent,
    session_id: &SessionId,
    thread_id: &ThreadId,
) -> Result<ClientEvent, TuiExit> {
    match event {
        ClientEvent::ConnectionClosed(reason) => Err(TuiExit::ConnectionLost {
            kind: connection_loss_kind(&reason),
            recovery: TuiRecoveryState::new(session_id.clone(), thread_id.clone()),
            reason: format!("App Server connection closed: {reason:?}"),
        }),
        event => Ok(event),
    }
}

fn connection_loss_kind(reason: &ConnectionCloseReason) -> TuiConnectionLossKind {
    match reason {
        ConnectionCloseReason::DriverStopped => TuiConnectionLossKind::Transport,
        ConnectionCloseReason::Shutdown => TuiConnectionLossKind::ServerShutdown,
        ConnectionCloseReason::ProtocolFailure(_) => TuiConnectionLossKind::Protocol,
    }
}

#[cfg(test)]
#[path = "recovery_tests.rs"]
mod tests;
