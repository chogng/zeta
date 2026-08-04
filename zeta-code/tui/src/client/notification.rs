use zeta_app_server_client::AppServerEvent;
use zeta_app_server_client::ServerNotification;
use zeta_protocol::ThreadUpdateEnvelope;

/// A connection-layer fact understood by the TUI event loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ClientEvent {
    Failed(String),
    SkillsChanged,
    ThreadUpdated(Box<ThreadUpdateEnvelope>),
}

pub(crate) fn map_event(event: AppServerEvent) -> Option<ClientEvent> {
    match event {
        AppServerEvent::Notification(ServerNotification::SkillsChanged(_)) => {
            Some(ClientEvent::SkillsChanged)
        }
        AppServerEvent::Notification(ServerNotification::SessionThreadUpdate(update)) => {
            Some(ClientEvent::ThreadUpdated(update))
        }
        AppServerEvent::Notification(
            ServerNotification::SessionUpdate(_)
            | ServerNotification::ConfigChanged(_)
            | ServerNotification::GitStatusChanged(_)
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
