use crate::CoreError;
use zeta_protocol::CommandId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;
use zeta_protocol::UserInput;

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

    /// Delivers one already-durable steering command to the active execution runtime.
    ///
    /// Local executors may treat this as a wake-free acknowledgement because every model safe
    /// point reads the canonical Thread snapshot. Delegated runtimes must not return success until
    /// the exact active remote Turn accepted the input.
    fn steer(
        &self,
        _: &ThreadId,
        _: &TurnId,
        _: &CommandId,
        _: &[UserInput],
    ) -> Result<(), CoreError> {
        Err(CoreError::Execution(
            "Turn execution backend does not support steering".into(),
        ))
    }
}
