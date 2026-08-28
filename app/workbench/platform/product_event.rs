//! Product events delivered through the desktop application loop.

use crate::language_service_host::remote::RemoteLanguageEvent;
use crate::remote_connection_process::RemoteWindowLaunchEvent;
use crate::remote_tunnel_process::RemoteTunnelEvent;
use crate::session_host::SessionRuntimeEvent;
use crate::terminal_session::{TerminalSessionEventEnvelope, TerminalSessionReady};
use zeta_lsp_manager::LanguageServiceEvent;

pub(crate) enum ProductEvent {
    Session(SessionRuntimeEvent),
    Terminal(TerminalSessionEventEnvelope),
    TerminalReady(TerminalSessionReady),
    LanguageService(LanguageServiceEvent),
    RemoteLanguage(RemoteLanguageEvent),
    RemoteWindowLaunch(RemoteWindowLaunchEvent),
    RemoteTunnel(RemoteTunnelEvent),
}

impl From<SessionRuntimeEvent> for ProductEvent {
    fn from(event: SessionRuntimeEvent) -> Self {
        Self::Session(event)
    }
}

impl From<TerminalSessionEventEnvelope> for ProductEvent {
    fn from(event: TerminalSessionEventEnvelope) -> Self {
        Self::Terminal(event)
    }
}

impl From<TerminalSessionReady> for ProductEvent {
    fn from(event: TerminalSessionReady) -> Self {
        Self::TerminalReady(event)
    }
}

impl From<LanguageServiceEvent> for ProductEvent {
    fn from(event: LanguageServiceEvent) -> Self {
        Self::LanguageService(event)
    }
}
