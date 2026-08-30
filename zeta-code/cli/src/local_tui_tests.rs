use zeta_app_server_client::ClientError;

use super::classify_error;
use crate::reconnect::Failure;

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
