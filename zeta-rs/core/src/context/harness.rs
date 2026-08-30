use super::HarnessInstructions;
use crate::CoreError;
use std::sync::Arc;
use zeta_agent_environment::AgentEnvironmentSnapshot;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;

/// Immutable host context captured for one model invocation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HarnessContext {
    instructions: HarnessInstructions,
    environment: Option<AgentEnvironmentSnapshot>,
}

impl HarnessContext {
    /// Starts a host-context snapshot with immutable instruction facts.
    pub fn new(instructions: HarnessInstructions) -> Self {
        Self {
            instructions,
            environment: None,
        }
    }

    /// Adds the environment facts visible to this model invocation.
    pub fn with_environment(mut self, environment: AgentEnvironmentSnapshot) -> Self {
        self.environment = Some(environment);
        self
    }

    /// Returns the immutable system and directory instructions.
    pub fn instructions(&self) -> &HarnessInstructions {
        &self.instructions
    }

    /// Returns the environment snapshot when the embedding host supplies one.
    pub fn environment(&self) -> Option<&AgentEnvironmentSnapshot> {
        self.environment.as_ref()
    }
}

/// Stable identities available when the host captures one harness-context snapshot.
pub struct HarnessContextRequest<'a> {
    /// Session whose runtime environment is being frozen.
    pub session_id: &'a SessionId,
    /// Thread about to invoke the model.
    pub thread_id: &'a ThreadId,
    /// Turn about to invoke the model.
    pub turn_id: &'a TurnId,
}

/// Supplies one immutable host-context snapshot at each model-invocation boundary.
///
/// Implementations collect host-owned instructions and environment facts. The returned value must
/// remain stable while Core plans and assembles that invocation.
pub trait HarnessContextProvider: Send + Sync {
    fn snapshot(
        &self,
        request: &HarnessContextRequest<'_>,
    ) -> Result<Arc<HarnessContext>, CoreError>;
}
