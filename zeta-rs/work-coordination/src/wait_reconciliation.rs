use crate::ResolveWaitOutcome;
use crate::WorkAttempt;
use crate::WorkAttemptExecutionStatus;
use crate::WorkCoordinationError;
use crate::WorkRelationKind;
use crate::WorkRelationStatus;
use crate::WorkRun;
use crate::WorkRunCommand;
use crate::WorkWaitCondition;
use zeta_protocol::ContentDigest;

/// Derives at most one durable wait transition from exact WorkAttempt execution state.
pub fn next_wait_resolution(
    run: &WorkRun,
) -> Result<Option<WorkRunCommand>, WorkCoordinationError> {
    for relation in run
        .relations
        .values()
        .filter(|relation| relation.status == WorkRelationStatus::Waiting)
    {
        let WorkRelationKind::Wait {
            target_execution_id,
            condition,
        } = &relation.kind
        else {
            return Err(WorkCoordinationError::InvalidInput(
                "a waiting relation does not contain a wait condition".into(),
            ));
        };
        let target = run
            .attempts
            .get(&relation.target_attempt_id)
            .ok_or_else(|| {
                WorkCoordinationError::NotFound("a WorkAttempt wait target disappeared".into())
            })?;
        let Some(outcome) = expected_wait_outcome(condition, target, target_execution_id)? else {
            continue;
        };
        return Ok(Some(WorkRunCommand::ResolveWait {
            relation_id: relation.relation_id.clone(),
            target_attempt_id: target.attempt_id.clone(),
            target_execution_id: target_execution_id.clone(),
            outcome,
        }));
    }
    Ok(None)
}

pub(crate) fn expected_wait_outcome(
    condition: &WorkWaitCondition,
    target: &WorkAttempt,
    target_execution_id: &zeta_protocol::WorkExecutionId,
) -> Result<Option<ResolveWaitOutcome>, WorkCoordinationError> {
    if target.execution_id.as_ref() != Some(target_execution_id) {
        return Ok(Some(ResolveWaitOutcome::SourceStale));
    }
    let outcome = match target.execution_status {
        WorkAttemptExecutionStatus::Sealed => {
            let result = target.result.as_ref().ok_or_else(|| {
                WorkCoordinationError::InvalidInput(
                    "a sealed wait target omitted its result".into(),
                )
            })?;
            match condition {
                WorkWaitCondition::ExecutionFinished | WorkWaitCondition::AttemptSealed => {
                    ResolveWaitOutcome::Satisfied {
                        evidence_digest: result.result_digest.clone(),
                    }
                }
                WorkWaitCondition::ExactResult { result_digest }
                    if result_digest == &result.result_digest =>
                {
                    ResolveWaitOutcome::Satisfied {
                        evidence_digest: result.result_digest.clone(),
                    }
                }
                WorkWaitCondition::ExactResult { .. } => ResolveWaitOutcome::Failed {
                    reason: "wait target sealed a different exact result".into(),
                },
            }
        }
        WorkAttemptExecutionStatus::Failed
        | WorkAttemptExecutionStatus::Interrupted
        | WorkAttemptExecutionStatus::Cancelled => match condition {
            WorkWaitCondition::ExecutionFinished => ResolveWaitOutcome::Satisfied {
                evidence_digest: terminal_evidence_digest(target)?,
            },
            WorkWaitCondition::AttemptSealed | WorkWaitCondition::ExactResult { .. } => {
                ResolveWaitOutcome::Failed {
                    reason: format!(
                        "wait target finished as {:?} without the required sealed result",
                        target.execution_status
                    ),
                }
            }
        },
        WorkAttemptExecutionStatus::Planned
        | WorkAttemptExecutionStatus::Exploring
        | WorkAttemptExecutionStatus::Writing
        | WorkAttemptExecutionStatus::Waiting => return Ok(None),
    };
    Ok(Some(outcome))
}

fn terminal_evidence_digest(attempt: &WorkAttempt) -> Result<ContentDigest, WorkCoordinationError> {
    let encoded = serde_json::to_vec(&(
        1_u32,
        &attempt.attempt_id,
        &attempt.execution_id,
        attempt.execution_status,
        &attempt.failure,
    ))
    .map_err(|error| WorkCoordinationError::InvalidInput(error.to_string()))?;
    Ok(ContentDigest::sha256(&encoded))
}
