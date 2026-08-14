use zeta_app_server_client::ConnectionCloseReason;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

use super::continue_or_exit;
use crate::TuiConnectionLossKind;
use crate::TuiExit;
use crate::client::ClientEvent;

#[test]
fn connection_loss_returns_only_durable_identity_and_diagnostic() {
    let session_id = SessionId::new("session-recovery").unwrap();
    let thread_id = ThreadId::new("thread-recovery").unwrap();

    let exit = continue_or_exit(
        ClientEvent::ConnectionClosed(ConnectionCloseReason::DriverStopped),
        &session_id,
        &thread_id,
    )
    .unwrap_err();

    let TuiExit::ConnectionLost {
        kind,
        recovery,
        reason,
    } = exit
    else {
        panic!("expected connection loss");
    };
    assert_eq!(kind, TuiConnectionLossKind::Transport);
    assert_eq!(recovery.session_id(), &session_id);
    assert_eq!(recovery.thread_id(), &thread_id);
    assert_eq!(reason, "App Server connection closed: DriverStopped");
}

#[test]
fn protocol_failure_remains_terminally_classified() {
    let session_id = SessionId::new("session-recovery").unwrap();
    let thread_id = ThreadId::new("thread-recovery").unwrap();

    let exit = continue_or_exit(
        ClientEvent::ConnectionClosed(ConnectionCloseReason::ProtocolFailure(
            "malformed frame".into(),
        )),
        &session_id,
        &thread_id,
    )
    .unwrap_err();

    assert!(matches!(
        exit,
        TuiExit::ConnectionLost {
            kind: TuiConnectionLossKind::Protocol,
            ..
        }
    ));
}
