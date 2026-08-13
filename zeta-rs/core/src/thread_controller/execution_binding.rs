use super::ThreadController;
use crate::CoreError;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnExecutionBinding;

/// Result of idempotently binding a Thread to an external Turn runtime.
pub struct BoundTurnExecution {
    pub binding: TurnExecutionBinding,
    pub sequence: u64,
}

impl ThreadController {
    /// Persists the immutable external conversation binding after a delegated Turn completes.
    pub fn bind_turn_execution(
        &self,
        thread_id: &ThreadId,
        binding: TurnExecutionBinding,
    ) -> Result<BoundTurnExecution, CoreError> {
        if binding.backend.trim().is_empty()
            || binding.remote_thread_id.trim().is_empty()
            || binding.execution_scope.trim().is_empty()
        {
            return Err(CoreError::InvalidInput(
                "Turn execution binding identities and scope must not be empty".into(),
            ));
        }
        self.mutate_thread(thread_id, |snapshot| {
            if let Some(existing) = &snapshot.turn_execution_binding {
                if existing == &binding {
                    return Ok(BoundTurnExecution {
                        binding,
                        sequence: snapshot.sequence,
                    });
                }
                return Err(CoreError::CommandConflict);
            }
            self.record_batch(
                snapshot,
                vec![ThreadEvent::TurnExecutionBound {
                    thread_id: thread_id.clone(),
                    binding: binding.clone(),
                }],
            )?;
            Ok(BoundTurnExecution {
                binding,
                sequence: snapshot.sequence,
            })
        })
    }
}
