use crate::WorkCoordinationError;
use crate::WorkRun;
use crate::WorkRunCommandRequest;
use crate::WorkRunCommit;
use crate::WorkRunStore;
use crate::WorkRunStoreError;
use crate::WorkRunStoreOutcome;
use std::sync::Arc;
use zeta_protocol::CommandId;
use zeta_protocol::WorkRunId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkCommandDisposition {
    Committed,
    Replayed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkCommandResult {
    pub work_run: WorkRun,
    pub disposition: WorkCommandDisposition,
}

/// Applies deterministic WorkRun commands and delegates only atomic persistence to its store.
pub struct WorkCoordinator {
    store: Arc<dyn WorkRunStore>,
}

impl WorkCoordinator {
    pub fn new(store: Arc<dyn WorkRunStore>) -> Self {
        Self { store }
    }

    pub fn list(&self) -> Result<Vec<WorkRun>, WorkCoordinationError> {
        let runs = self.store.list().map_err(map_store_error)?;
        for run in &runs {
            run.validate()?;
        }
        Ok(runs)
    }

    pub fn read(&self, work_run_id: &WorkRunId) -> Result<WorkRun, WorkCoordinationError> {
        let run = self.store.load(work_run_id).map_err(map_store_error)?;
        run.validate()?;
        Ok(run)
    }

    /// Reads one exact durable command receipt for a higher-level host operation.
    pub fn command_receipt(
        &self,
        command_id: &CommandId,
    ) -> Result<Option<WorkRunCommit>, WorkCoordinationError> {
        let receipt = self
            .store
            .load_command(command_id)
            .map_err(map_store_error)?;
        if let Some(receipt) = &receipt {
            receipt.result.validate()?;
        }
        Ok(receipt)
    }

    pub fn apply(
        &self,
        request: WorkRunCommandRequest,
    ) -> Result<WorkCommandResult, WorkCoordinationError> {
        if let Some(replayed) = self.replay(&request)? {
            return Ok(replayed);
        }
        let current = match self.store.load(&request.work_run_id) {
            Ok(run) => {
                run.validate()?;
                Some(run)
            }
            Err(WorkRunStoreError::NotFound(_)) => None,
            Err(error) => return Err(map_store_error(error)),
        };
        let result = crate::reducer::apply(current, &request)?;
        result.validate()?;
        let commit = WorkRunCommit { request, result };
        match self.store.commit(&commit).map_err(map_store_error)? {
            WorkRunStoreOutcome::Applied => Ok(WorkCommandResult {
                work_run: commit.result,
                disposition: WorkCommandDisposition::Committed,
            }),
            WorkRunStoreOutcome::Replayed(work_run) => Ok(WorkCommandResult {
                work_run,
                disposition: WorkCommandDisposition::Replayed,
            }),
        }
    }

    /// Returns an exact durable command result without re-running host side-effect validation.
    pub fn replay(
        &self,
        request: &WorkRunCommandRequest,
    ) -> Result<Option<WorkCommandResult>, WorkCoordinationError> {
        let Some(receipt) = self
            .store
            .load_command(&request.command_id)
            .map_err(map_store_error)?
        else {
            return Ok(None);
        };
        if receipt.request != *request {
            return Err(WorkCoordinationError::CommandConflict);
        }
        receipt.result.validate()?;
        Ok(Some(WorkCommandResult {
            work_run: receipt.result,
            disposition: WorkCommandDisposition::Replayed,
        }))
    }
}

fn map_store_error(error: WorkRunStoreError) -> WorkCoordinationError {
    match error {
        WorkRunStoreError::NotFound(identity) => WorkCoordinationError::NotFound(identity),
        WorkRunStoreError::AlreadyExists(identity) => {
            WorkCoordinationError::AlreadyExists(identity)
        }
        WorkRunStoreError::CommandConflict => WorkCoordinationError::CommandConflict,
        WorkRunStoreError::RevisionConflict { expected, actual } => {
            WorkCoordinationError::RevisionConflict { expected, actual }
        }
        WorkRunStoreError::ThreadBusy {
            thread_id,
            work_run_id,
            attempt_id,
        } => WorkCoordinationError::ThreadBusy {
            thread_id,
            work_run_id,
            attempt_id,
        },
        WorkRunStoreError::Storage(message) => WorkCoordinationError::Storage(message),
    }
}
