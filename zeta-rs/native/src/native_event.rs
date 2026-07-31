use crate::agent_session::AgentSessionEvent;
use crate::terminal_session::TerminalSessionEvent;

pub(crate) enum NativeEvent {
    Agent(AgentSessionEvent),
    Terminal(TerminalSessionEvent),
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
