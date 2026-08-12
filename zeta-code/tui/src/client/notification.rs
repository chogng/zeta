use zeta_app_server_client::AppServerEvent;
use zeta_app_server_client::ServerNotification;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::ThreadUpdateEnvelope;

/// A connection-layer fact understood by the TUI event loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ClientEvent {
    AgentRequest(Box<AgentRequestEnvelope>),
    Failed(String),
    GitStatusChanged(GitStatusResult),
    ConnectorsChanged,
    SkillsChanged,
    ThreadUpdated(Box<ThreadUpdateEnvelope>),
}

pub(crate) fn map_event(event: AppServerEvent) -> Option<ClientEvent> {
    match event {
        AppServerEvent::Notification(ServerNotification::AgentRequest(request)) => {
            Some(ClientEvent::AgentRequest(Box::new(request)))
        }
        AppServerEvent::Notification(ServerNotification::SkillsChanged(_)) => {
            Some(ClientEvent::SkillsChanged)
        }
        AppServerEvent::Notification(ServerNotification::ConnectorsChanged(_)) => {
            Some(ClientEvent::ConnectorsChanged)
        }
        AppServerEvent::Notification(ServerNotification::GitStatusChanged(changed)) => {
            Some(ClientEvent::GitStatusChanged(changed.status))
        }
        AppServerEvent::Notification(ServerNotification::SessionThreadUpdate(update)) => {
            Some(ClientEvent::ThreadUpdated(update))
        }
        AppServerEvent::Notification(
            ServerNotification::DocumentCollaborationUpdate(_)
            | ServerNotification::DocumentCollaborationPresence(_)
            | ServerNotification::SessionUpdate(_)
            | ServerNotification::ConfigChanged(_)
            | ServerNotification::FsChanged(_)
            | ServerNotification::Unknown { .. },
        ) => None,
        AppServerEvent::ConnectionClosed(reason) => Some(ClientEvent::Failed(format!(
            "App Server connection closed: {reason:?}"
        ))),
    }
}

#[cfg(test)]
#[path = "notification_tests.rs"]
mod tests;
