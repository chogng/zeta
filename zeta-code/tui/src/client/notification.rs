use zeta_app_server_client::AppServerEvent;
use zeta_app_server_client::ConnectionCloseReason;
use zeta_app_server_client::ServerNotification;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptUpdateEnvelope;
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadUpdateEnvelope;

/// A connection-layer fact understood by the TUI event loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ClientEvent {
    AgentRequest(Box<AgentRequestEnvelope>),
    ConfigChanged,
    ConnectionClosed(ConnectionCloseReason),
    GitStatusChanged(GitStatusResult),
    ConnectorsChanged,
    PackageSourcesChanged,
    SkillsChanged,
    SessionChanged(SessionId),
    ThreadUpdated(Box<ThreadUpdateEnvelope>),
    ThreadTranscriptUpdated(Box<ThreadTranscriptUpdateEnvelope>),
}

pub(crate) fn map_event(event: AppServerEvent) -> Option<ClientEvent> {
    match event {
        AppServerEvent::Notification(notification) => project_notification(notification),
        AppServerEvent::ConnectionClosed(reason) => Some(ClientEvent::ConnectionClosed(reason)),
    }
}

fn project_notification(notification: ServerNotification) -> Option<ClientEvent> {
    match notification {
        ServerNotification::AgentRequest(request) => {
            Some(ClientEvent::AgentRequest(Box::new(request)))
        }
        ServerNotification::ConnectorsChanged(_) => Some(ClientEvent::ConnectorsChanged),
        ServerNotification::ConfigChanged(_) => Some(ClientEvent::ConfigChanged),
        ServerNotification::MarketplaceChanged(_) | ServerNotification::PluginsChanged(_) => {
            Some(ClientEvent::PackageSourcesChanged)
        }
        ServerNotification::SkillsChanged(_) => Some(ClientEvent::SkillsChanged),
        ServerNotification::GitStatusChanged(changed) => {
            Some(ClientEvent::GitStatusChanged(changed.status))
        }
        ServerNotification::SessionThreadUpdate(update) => Some(ClientEvent::ThreadUpdated(update)),
        ServerNotification::SessionChanged(changed) => {
            Some(ClientEvent::SessionChanged(changed.session_id))
        }
        ServerNotification::SessionThreadTranscriptUpdate(update) => {
            Some(ClientEvent::ThreadTranscriptUpdated(Box::new(update)))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "notification_tests.rs"]
mod tests;
