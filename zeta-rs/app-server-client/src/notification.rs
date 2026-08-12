use crate::ClientError;
use serde::Deserialize;
use serde_json::Value;
pub use zeta_app_server_protocol::protocol::notification::ServerNotification;
use zeta_app_server_protocol::protocol::notification::decode_server_notification;

#[derive(Deserialize)]
struct NotificationEnvelope {
    method: String,
    params: Value,
}

pub(crate) fn decode(raw: &str) -> Result<ServerNotification, ClientError> {
    let envelope: NotificationEnvelope =
        serde_json::from_str(raw).map_err(|error| ClientError::Protocol(error.to_string()))?;
    decode_server_notification(envelope.method, envelope.params)
        .map_err(|error| ClientError::Protocol(error.to_string()))
}

#[cfg(test)]
#[path = "notification_tests.rs"]
mod tests;
