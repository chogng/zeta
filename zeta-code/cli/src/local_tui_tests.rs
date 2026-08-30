use zeta_app_server_client::ClientError;

use super::classify_error;
use super::recovery_command;
use crate::reconnect::Failure;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

#[test]
fn local_reconnect_retries_only_transport_failures() {
    assert!(matches!(
        classify_error(ClientError::Transport("closed".into())),
        Failure::Retryable(message) if message == "transport error: closed"
    ));
    assert!(matches!(
        classify_error(ClientError::Protocol("schema changed".into())),
        Failure::Terminal(message) if message.contains("schema changed")
    ));
    assert!(matches!(
        classify_error(ClientError::Server {
            code: -32000,
            message: "rejected".into(),
        }),
        Failure::Terminal(message) if message.contains("rejected")
    ));
}

#[test]
fn local_recovery_command_preserves_session_and_thread() {
    let recovery = zeta_tui::TuiRecoveryState::new(
        SessionId::new("session-1").unwrap(),
        ThreadId::new("thread-1").unwrap(),
    );

    assert_eq!(
        recovery_command(&recovery),
        ["zeta", "resume", "session-1", "thread-1"]
    );
}
