use crate::agent_session::AgentSessionEvent;
use crate::terminal_session::{TerminalSessionEventEnvelope, TerminalSessionReady};
use zeta_language_service::LanguageServiceEvent;

pub(crate) enum NativeEvent {
    Agent(AgentSessionEvent),
    Terminal(TerminalSessionEventEnvelope),
    TerminalReady(TerminalSessionReady),
    LanguageService(LanguageServiceEvent),
}

impl From<AgentSessionEvent> for NativeEvent {
    fn from(event: AgentSessionEvent) -> Self {
        Self::Agent(event)
    }
}

impl From<TerminalSessionEventEnvelope> for NativeEvent {
    fn from(event: TerminalSessionEventEnvelope) -> Self {
        Self::Terminal(event)
    }
}

impl From<TerminalSessionReady> for NativeEvent {
    fn from(event: TerminalSessionReady) -> Self {
        Self::TerminalReady(event)
    }
}

impl From<LanguageServiceEvent> for NativeEvent {
    fn from(event: LanguageServiceEvent) -> Self {
        Self::LanguageService(event)
    }
}
