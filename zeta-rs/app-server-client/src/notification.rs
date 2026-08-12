use crate::ClientError;
use serde::Deserialize;
use serde_json::Value;
use zeta_app_server_protocol::protocol::collaboration::DocumentCollaborationPresenceSnapshot;
use zeta_app_server_protocol::protocol::collaboration::DocumentCollaborationUpdate;
use zeta_app_server_protocol::protocol::notification::ConfigChanged;
use zeta_app_server_protocol::protocol::notification::ConnectorsChanged;
use zeta_app_server_protocol::protocol::notification::FsChanged;
use zeta_app_server_protocol::protocol::notification::GitStatusChanged;
use zeta_app_server_protocol::protocol::notification::SessionUpdateEnvelope;
use zeta_app_server_protocol::protocol::notification::SkillsChanged;
use zeta_app_server_protocol::protocol::notification::ThreadUpdateEnvelope;
use zeta_app_server_protocol::protocol::registry::ServerNotificationMethod;
use zeta_app_server_protocol::protocol::registry::server_notification_method;
use zeta_protocol::AgentRequestEnvelope;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerNotification {
    AgentRequest(AgentRequestEnvelope),
    DocumentCollaborationUpdate(DocumentCollaborationUpdate),
    DocumentCollaborationPresence(DocumentCollaborationPresenceSnapshot),
    SessionUpdate(SessionUpdateEnvelope),
    SessionThreadUpdate(Box<ThreadUpdateEnvelope>),
    ConfigChanged(ConfigChanged),
    ConnectorsChanged(ConnectorsChanged),
    SkillsChanged(SkillsChanged),
    GitStatusChanged(GitStatusChanged),
    FsChanged(FsChanged),
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
        Some(ServerNotificationMethod::AgentRequest) => {
            decode_params(envelope.params).map(ServerNotification::AgentRequest)
        }
        Some(ServerNotificationMethod::DocumentCollaborationUpdate) => {
            decode_params(envelope.params).map(ServerNotification::DocumentCollaborationUpdate)
        }
        Some(ServerNotificationMethod::DocumentCollaborationPresence) => {
            decode_params(envelope.params).map(ServerNotification::DocumentCollaborationPresence)
        }
        Some(ServerNotificationMethod::SessionUpdate) => {
            decode_params(envelope.params).map(ServerNotification::SessionUpdate)
        }
        Some(ServerNotificationMethod::SessionThreadUpdate) => decode_params(envelope.params)
            .map(Box::new)
            .map(ServerNotification::SessionThreadUpdate),
        Some(ServerNotificationMethod::ConfigChanged) => {
            decode_params(envelope.params).map(ServerNotification::ConfigChanged)
        }
        Some(ServerNotificationMethod::ConnectorsChanged) => {
            decode_params(envelope.params).map(ServerNotification::ConnectorsChanged)
        }
        Some(ServerNotificationMethod::SkillsChanged) => {
            decode_params(envelope.params).map(ServerNotification::SkillsChanged)
        }
        Some(ServerNotificationMethod::GitStatusChanged) => {
            decode_params(envelope.params).map(ServerNotification::GitStatusChanged)
        }
        Some(ServerNotificationMethod::FsChanged) => {
            decode_params(envelope.params).map(ServerNotification::FsChanged)
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

#[cfg(test)]
#[path = "notification_tests.rs"]
mod tests;
