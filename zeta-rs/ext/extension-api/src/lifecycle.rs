use crate::ExtensionError;
use extension_items::ExtensionItem;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ThreadLifecycle {
    Created,
    Archived,
    Restored,
    TurnStarted(TurnId),
    TurnCompleted(TurnId),
    TurnFailed(TurnId),
    TurnInterrupted(TurnId),
}

#[derive(Clone, Copy)]
pub struct ThreadContext<'a> {
    pub session_id: &'a SessionId,
    pub thread_id: &'a ThreadId,
    pub sequence: u64,
}

/// Receives committed lifecycle facts, never provisional state or replayed history.
/// Callbacks cannot re-enter Core or veto a committed mutation. Durable side effects must deduplicate
/// by Thread and sequence; an observer is not a second Thread-state owner.
pub trait LifecycleObserver: Send + Sync {
    fn thread_changed(&self, context: ThreadContext<'_>, event: &ThreadLifecycle);
    fn config_changed(&self, generation: u64);
}

/// Schedules extension-owned idle work after a Turn's terminal fact commits.
/// Implementations may wake their domain scheduler, but cannot directly start a Turn or re-enter Core.
/// Hosts must also reconcile durable work at startup; callbacks are not a durable delivery channel.
pub trait IdleContributor: Send + Sync {
    fn contribute(&self, context: ThreadContext<'_>);
}

/// Supplies read-only, text-only items for one authorized Thread view.
/// The host validates bounds and uniqueness; extensions retain ownership of the underlying state.
pub trait ItemContributor: Send + Sync {
    fn contribute(&self, context: ThreadContext<'_>) -> Result<Vec<ExtensionItem>, ExtensionError>;
}
