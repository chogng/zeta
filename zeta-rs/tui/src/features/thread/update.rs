use zeta_protocol::Thread;

/// A presentation-level fact consumed by the active Thread feature.
pub(crate) enum ThreadPresentationEvent {
    SnapshotReceived(Thread),
    UserSubmitted(String),
    NoticeReceived(String),
    FailureReported(String),
    Interrupted,
    Cleared,
}
