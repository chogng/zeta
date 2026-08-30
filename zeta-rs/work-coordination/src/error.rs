use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkRunId;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WorkCoordinationError {
    #[error("invalid work coordination input: {0}")]
    InvalidInput(String),
    #[error("work coordination record was not found: {0}")]
    NotFound(String),
    #[error("work coordination record already exists: {0}")]
    AlreadyExists(String),
    #[error("work run is no longer active")]
    WorkRunClosed,
    #[error("work coordination transition is invalid: {0}")]
    InvalidTransition(String),
    #[error("work command ID was already used for different parameters")]
    CommandConflict,
    #[error("work-run revision conflict: expected {expected}, actual {actual}")]
    RevisionConflict { expected: u64, actual: u64 },
    #[error(
        "Thread {thread_id} is already executing attempt {attempt_id} in WorkRun {work_run_id}"
    )]
    ThreadBusy {
        thread_id: ThreadId,
        work_run_id: WorkRunId,
        attempt_id: WorkAttemptId,
    },
    #[error("work coordination storage failed: {0}")]
    Storage(String),
}
