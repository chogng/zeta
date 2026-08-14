use zeta_app_server_client::AppServerEvent;
use zeta_app_server_client::ConnectionCloseReason;
use zeta_app_server_client::ServerNotification;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::ThreadUpdateEnvelope;

/// A connection-layer fact understood by the TUI event loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ClientEvent {
    AgentRequest(Box<AgentRequestEnvelope>),
    ConnectionClosed(ConnectionCloseReason),
    GitStatusChanged(GitStatusResult),
    ConnectorsChanged,
    SkillsChanged,
    ThreadUpdated(Box<ThreadUpdateEnvelope>),
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
        ServerNotification::PluginsChanged(_) => None,
        ServerNotification::SkillsChanged(_) => Some(ClientEvent::SkillsChanged),
        ServerNotification::GitStatusChanged(changed) => {
            Some(ClientEvent::GitStatusChanged(changed.status))
        }
        ServerNotification::SessionThreadUpdate(update) => Some(ClientEvent::ThreadUpdated(update)),
        _ => None,
    }
}

#[cfg(test)]
#[path = "notification_tests.rs"]
mod tests;
