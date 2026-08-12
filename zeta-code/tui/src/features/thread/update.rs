use zeta_protocol::Thread;
use zeta_protocol::ThreadUpdateEnvelope;

/// A presentation-level fact consumed by the active Thread feature.
pub(crate) enum ThreadPresentationEvent {
    SnapshotReceived(Thread),
    HistoryPageReceived(Thread),
    TransientStreamReset,
    TransientUpdateReceived(Box<ThreadUpdateEnvelope>),
    UserSubmitted(String),
    CommandStarted(String),
    CommandCompleted { command: String, result: String },
    NoticeReceived(String),
    FailureReported(String),
    Interrupted,
    Cleared,
}
