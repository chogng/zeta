//! Product events delivered through the desktop application loop.

use crate::remote_connection_process::RemoteWindowLaunchEvent;
use crate::remote_tunnel_process::RemoteTunnelEvent;
use crate::session_host::SessionRuntimeEvent;
use crate::terminal_session::{TerminalSessionEventEnvelope, TerminalSessionReady};
use zeta_editor_host::FileEditorLanguageEvent;
use zeta_terminal_runtime::TerminalRuntimeEvent;

pub(crate) enum ProductEvent {
    Session(SessionRuntimeEvent),
    Terminal(TerminalSessionEventEnvelope),
    TerminalReady(TerminalSessionReady),
    EditorLanguage(FileEditorLanguageEvent),
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

impl From<TerminalRuntimeEvent> for ProductEvent {
    fn from(event: TerminalRuntimeEvent) -> Self {
        match event {
            TerminalRuntimeEvent::Session(event) => Self::Terminal(event),
            TerminalRuntimeEvent::Ready(event) => Self::TerminalReady(event),
        }
    }
}
