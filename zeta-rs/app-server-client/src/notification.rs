use crate::ClientError;
use serde::Deserialize;
use serde_json::Value;
use zeta_app_server_protocol::v1::notification::AgentMessageCompletedNotification;
use zeta_app_server_protocol::v1::notification::ThreadStartedNotification;
use zeta_app_server_protocol::v1::notification::TurnCompletedNotification;
use zeta_app_server_protocol::v1::notification::TurnInterruptedNotification;
use zeta_app_server_protocol::v1::notification::TurnStartedNotification;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerNotification {
    ThreadStarted(ThreadStartedNotification),
    TurnStarted(TurnStartedNotification),
    AgentMessageCompleted(AgentMessageCompletedNotification),
    TurnCompleted(TurnCompletedNotification),
    TurnInterrupted(TurnInterruptedNotification),
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
    match envelope.method.as_str() {
        "thread/started" => decode_params(envelope.params).map(ServerNotification::ThreadStarted),
        "turn/started" => decode_params(envelope.params).map(ServerNotification::TurnStarted),
        "item/agentMessage/completed" => {
            decode_params(envelope.params).map(ServerNotification::AgentMessageCompleted)
        }
        "turn/completed" => decode_params(envelope.params).map(ServerNotification::TurnCompleted),
        "turn/interrupted" => {
            decode_params(envelope.params).map(ServerNotification::TurnInterrupted)
        }
        _ => Ok(ServerNotification::Unknown {
            method: envelope.method,
            params: envelope.params,
        }),
    }
}

fn decode_params<T: for<'a> Deserialize<'a>>(params: Value) -> Result<T, ClientError> {
    serde_json::from_value(params).map_err(|error| ClientError::Protocol(error.to_string()))
}
