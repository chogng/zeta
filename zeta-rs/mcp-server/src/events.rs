use zeta_protocol::{AgentRequestEnvelope, AgentResponse};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AgentProgress {
    pub(crate) message: String,
}

pub(crate) enum InteractionResolution {
    Respond(AgentResponse),
    Unavailable,
}

/// Projects one Agent invocation's transient updates and durable interaction requests.
///
/// Implementations must keep progress bounded, bind responses to the exact request envelope, and
/// return [`InteractionResolution::Unavailable`] instead of inventing approval or user input.
pub(crate) trait AgentEvents: Send + Sync {
    fn progress(&self, progress: AgentProgress);

    fn resolve_interaction(&self, request: &AgentRequestEnvelope) -> InteractionResolution;
}

#[cfg(test)]
pub(crate) struct IgnoreAgentEvents;

#[cfg(test)]
impl AgentEvents for IgnoreAgentEvents {
    fn progress(&self, _: AgentProgress) {}

    fn resolve_interaction(&self, _: &AgentRequestEnvelope) -> InteractionResolution {
        InteractionResolution::Unavailable
    }
}
