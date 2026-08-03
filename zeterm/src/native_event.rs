use crate::agent_session::AgentSessionEvent;
use crate::terminal_session::TerminalSessionEvent;
use zeta_language_service::LanguageServiceEvent;

pub(crate) enum NativeEvent {
    Agent(AgentSessionEvent),
    Terminal(TerminalSessionEvent),
    LanguageService(LanguageServiceEvent),
}

impl From<AgentSessionEvent> for NativeEvent {
    fn from(event: AgentSessionEvent) -> Self {
        Self::Agent(event)
    }
}

impl From<TerminalSessionEvent> for NativeEvent {
    fn from(event: TerminalSessionEvent) -> Self {
        Self::Terminal(event)
    }
}

impl From<LanguageServiceEvent> for NativeEvent {
    fn from(event: LanguageServiceEvent) -> Self {
        Self::LanguageService(event)
    }
}
