use crate::CoreError;
use zeta_protocol::ItemId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;
use zeta_protocol::WorkspaceCheckpoint;

/// Snapshot reservation kept alive until its message batch commits or is abandoned.
pub trait CheckpointCapture: Send + Sync {
    fn workspace(&self) -> &WorkspaceCheckpoint;
    fn commit(&self);
}

/// Captures file evidence without reading or mutating Core's locked Thread state.
pub trait MessageCheckpointSource: Send + Sync {
    fn capture(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        item_id: &ItemId,
        event_id: &str,
    ) -> Result<Box<dyn CheckpointCapture>, CoreError>;
    fn release(&self, checkpoint: &zeta_protocol::RepositoryCheckpoint) -> Result<(), CoreError>;
}

pub(crate) struct NoFilesCapture(pub(crate) WorkspaceCheckpoint);
impl CheckpointCapture for NoFilesCapture {
    fn workspace(&self) -> &WorkspaceCheckpoint {
        &self.0
    }
    fn commit(&self) {}
}

pub(crate) struct PreparedThreadBatch {
    pub(crate) data: zeta_thread_store::ThreadEventBatch,
    pub(crate) captures: Vec<Box<dyn CheckpointCapture>>,
}

impl std::ops::Deref for PreparedThreadBatch {
    type Target = zeta_thread_store::ThreadEventBatch;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
