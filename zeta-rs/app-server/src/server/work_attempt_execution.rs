use super::turn_changes_runtime::TurnChangesRuntime;
use zeta_core::CoreError;
use zeta_core::InterruptTurnRequest;
use zeta_core::SequenceExpectation;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::TurnStatus;
use zeta_protocol::WorkRunId;
use zeta_thread_store::ThreadStoreError;
use zeta_work_coordination::WorkAttempt;

const MAX_STOP_RETRIES: usize = 8;

impl TurnChangesRuntime {
    /// Stops every in-flight Turn that could still use an execution scope before that scope is
    /// removed from the Thread.
    pub(super) fn stop_work_attempt_turns(
        &self,
        work_run_id: &WorkRunId,
        attempt: &WorkAttempt,
    ) -> Result<(), String> {
        for _ in 0..MAX_STOP_RETRIES {
            let active_matches = self
                .active_work_attempts
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&attempt.thread_id)
                .is_some_and(|active| {
                    &active.identity.work_run_id == work_run_id
                        && active.identity.attempt_id == attempt.attempt_id
                });
            if !active_matches {
                return Ok(());
            }
            let snapshot = self
                .threads
                .read_thread(&attempt.thread_id)
                .map_err(|error| error.to_string())?;
            let Some(turn) = snapshot
                .turns
                .iter()
                .rev()
                .find(|turn| is_interruptible(turn.status))
            else {
                return Ok(());
            };
            let after_sequence = snapshot.sequence;
            let command_id = stop_command_id(work_run_id, &attempt.attempt_id, &turn.turn_id)?;
            match self.threads.interrupt_turn(
                &snapshot.thread_id,
                InterruptTurnRequest {
                    command_id,
                    expected_sequence: SequenceExpectation::Exact(after_sequence),
                    turn_id: turn.turn_id.clone(),
                },
            ) {
                Ok(_) => {
                    let updates = self
                        .threads
                        .thread_updates_after(&snapshot.thread_id, after_sequence)
                        .map_err(|error| error.to_string())?;
                    self.updates.publish_thread(&snapshot.thread_id, &updates);
                }
                Err(CoreError::ThreadStore(ThreadStoreError::SequenceConflict { .. }))
                | Err(CoreError::InvalidTransition { .. }) => continue,
                Err(error) => return Err(error.to_string()),
            }
        }
        Err("WorkAttempt Turn could not be stopped at a stable Thread sequence".into())
    }
}

fn is_interruptible(status: TurnStatus) -> bool {
    matches!(
        status,
        TurnStatus::Created
            | TurnStatus::Running
            | TurnStatus::WaitingForApproval
            | TurnStatus::WaitingForUserInput
            | TurnStatus::WaitingForCapability
            | TurnStatus::Cancelling
    )
}

fn stop_command_id(
    work_run_id: &WorkRunId,
    attempt_id: &zeta_protocol::WorkAttemptId,
    turn_id: &zeta_protocol::TurnId,
) -> Result<CommandId, String> {
    let digest = ContentDigest::sha256(
        format!("work-attempt-stop:{work_run_id}:{attempt_id}:{turn_id}").as_bytes(),
    )
    .to_string()
    .replace(':', "-");
    CommandId::new(format!("work-attempt-stop-{digest}")).map_err(|error| error.to_string())
}
