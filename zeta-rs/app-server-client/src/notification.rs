use crate::ClientError;
use serde::Deserialize;
use serde_json::Value;
use zeta_app_server_protocol::protocol::notification::{
    SessionUpdateEnvelope, ThreadUpdateEnvelope,
};
use zeta_app_server_protocol::protocol::registry::{
    ServerNotificationMethod, server_notification_method,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerNotification {
    SessionUpdate(SessionUpdateEnvelope),
    ThreadUpdate(ThreadUpdateEnvelope),
    Unknown { method: String, params: Value },
}

#[derive(Deserialize)]
struct NotificationEnvelope {
    method: String,
    params: Value,
}

pub(crate) fn decode(raw: &str) -> Result<ServerNotification, ClientError> {
    let envelope: NotificationEnvelope =
        serde_json::from_str(raw).map_err(|error| ClientError::Protocol(error.to_string()))?;
    match server_notification_method(&envelope.method) {
        Some(ServerNotificationMethod::SessionUpdate) => {
            decode_params(envelope.params).map(ServerNotification::SessionUpdate)
        }
        Some(ServerNotificationMethod::ThreadUpdate) => {
            decode_params(envelope.params).map(ServerNotification::ThreadUpdate)
        }
        None => Ok(ServerNotification::Unknown {
            method: envelope.method,
            params: envelope.params,
        }),
    }
}

fn decode_params<T: for<'a> Deserialize<'a>>(params: Value) -> Result<T, ClientError> {
    serde_json::from_value(params).map_err(|error| ClientError::Protocol(error.to_string()))
}
