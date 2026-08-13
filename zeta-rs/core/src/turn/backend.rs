use crate::CoreError;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;

/// Executes an already-created durable Turn without owning Thread persistence.
///
/// Implementations may run Zeta's provider-independent model/tool loop or delegate the complete
/// agent loop to another runtime. In both cases Core remains the authority for Thread state,
/// interactions, cancellation, and terminal outcomes. A backend must never reinterpret itself as
/// a raw model provider when it owns tool execution or approval requests.
pub trait TurnExecutionBackend: Send + Sync {
    /// Accepts a newly created running Turn for asynchronous execution.
    fn start(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError>;

    /// Continues a running Turn after a durable interaction response or recovery decision.
    fn resume(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError>;
}
