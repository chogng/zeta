use crate::CoreError;
use zeta_protocol::AgentEvent;
use zeta_protocol::ThreadId;

/// Persists the append-only events that establish a Thread's durable history.
///
/// Implementations must make `append` durable before returning success; callers only project
/// in-memory state or notify clients after that point.
pub trait EventJournal: Send + Sync {
    fn append(&self, event: &AgentEvent) -> Result<(), CoreError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdempotencyRecord {
    pub method: String,
    pub key: String,
    pub parameters: String,
    pub result: String,
}

/// Stores durable results for retry-safe side-effecting requests.
///
/// Implementations scope records to one state root, preserve the method/key/parameter binding, and
/// must return a conflict to callers that reuse a key with different canonical parameters.
pub trait IdempotencyLedger: Send + Sync {
    fn get(&self, method: &str, key: &str) -> Result<Option<IdempotencyRecord>, CoreError>;
    fn put(&self, record: IdempotencyRecord) -> Result<(), CoreError>;
}

/// Holds a process-local or inter-process write lock for a Thread.
///
/// Implementations release their underlying lease when the guard is dropped and must never let
/// two live guards represent concurrent writers for the same Thread.
pub trait LeaseGuard: Send {}

/// Arbitrates exclusive write access to durable Thread state.
///
/// Implementations are expected to scope leases by `ThreadId`, reject competing writers, and
/// return a guard that keeps the lease alive for the duration of a state-changing operation.
pub trait ThreadWriterLease: Send + Sync {
    fn acquire(&self, thread_id: &ThreadId) -> Result<Box<dyn LeaseGuard>, CoreError>;
}

/// Produces Agent text from a user prompt without exposing provider-specific transport details.
///
/// Implementations must honor their provider's cancellation and credential policies and return
/// only user-safe response text to the Agent service.
pub trait AgentModel: Send + Sync {
    fn respond(&self, prompt: &str) -> Result<String, CoreError>;
}

/// Decides whether a tool action needs an explicit user approval.
///
/// Implementations inspect the fully materialized action and must distinguish approval from a
/// denial; tool execution is allowed only after an explicit approval decision.
pub trait ApprovalPolicy: Send + Sync {
    fn requirement_for(&self, action_digest: &str) -> ApprovalRequirement;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalRequirement {
    NotRequired,
    Required,
    Denied,
}

impl ApprovalRequirement {
    pub fn allows_execution(self) -> bool {
        matches!(self, Self::NotRequired)
    }
}
