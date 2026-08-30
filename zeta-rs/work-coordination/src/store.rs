use crate::WorkRun;
use crate::WorkRunCommandRequest;
use serde::Deserialize;
use serde::Serialize;
use zeta_protocol::CommandId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkRunId;

/// Complete result and request persisted atomically for retry-safe replay.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunCommit {
    pub request: WorkRunCommandRequest,
    pub result: WorkRun,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkRunStoreOutcome {
    Applied,
    Replayed(WorkRun),
}

/// Persistence boundary for complete WorkRun records and exact command receipts.
///
/// Implementations compare the aggregate revision and commit the next record plus its receipt in
/// one transaction. The same transaction enforces at most one active WorkAttempt writer for each
/// Thread across every WorkRun. Implementations never merge fields from competing writers.
pub trait WorkRunStore: Send + Sync {
    fn list(&self) -> Result<Vec<WorkRun>, WorkRunStoreError>;

    fn load(&self, work_run_id: &WorkRunId) -> Result<WorkRun, WorkRunStoreError>;

    fn load_command(
        &self,
        command_id: &CommandId,
    ) -> Result<Option<WorkRunCommit>, WorkRunStoreError>;

    fn commit(&self, commit: &WorkRunCommit) -> Result<WorkRunStoreOutcome, WorkRunStoreError>;
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WorkRunStoreError {
    #[error("work run was not found: {0}")]
    NotFound(String),
    #[error("work run already exists: {0}")]
    AlreadyExists(String),
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
