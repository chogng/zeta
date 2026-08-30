use crate::ResolveWaitOutcome;
use crate::WorkAttempt;
use crate::WorkAttemptCoordinationStatus;
use crate::WorkAttemptExecutionStatus;
use crate::WorkAttemptVerificationStatus;
use crate::WorkConflict;
use crate::WorkConflictStatus;
use crate::WorkCoordinationError;
use crate::WorkRelation;
use crate::WorkRelationKind;
use crate::WorkRelationStatus;
use crate::WorkRun;
use crate::WorkWaitCondition;
use crate::attempt_reducer;
use crate::validation;
use std::collections::BTreeSet;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkConflictId;
use zeta_protocol::WorkDecisionId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRelationId;

pub(crate) fn record_conflict(
    run: &mut WorkRun,
    conflict_id: &WorkConflictId,
    attempt_ids: &[WorkAttemptId],
    resource: &str,
    evidence: &[String],
) -> Result<(), WorkCoordinationError> {
    if run.conflicts.contains_key(conflict_id) {
        return Err(WorkCoordinationError::AlreadyExists(
            conflict_id.to_string(),
        ));
    }
    if attempt_ids.is_empty() {
        return Err(WorkCoordinationError::InvalidInput(
            "a conflict requires at least one attempt".into(),
        ));
    }
    validation::text("conflict resource", resource)?;
    validation::non_empty_texts("conflict evidence", evidence)?;
    let mut unique = BTreeSet::new();
    if !attempt_ids
        .iter()
        .all(|attempt_id| unique.insert(attempt_id))
    {
        return Err(WorkCoordinationError::InvalidInput(
            "a conflict cannot repeat a WorkAttempt".into(),
        ));
    }
    for attempt_id in attempt_ids {
        attempt_reducer::stop_for_coordination(
            run,
            attempt_id,
            WorkAttemptCoordinationStatus::Conflict,
            "an unresolved coordination conflict stopped this WorkAttempt",
        )?;
    }
    run.conflicts.insert(
        conflict_id.clone(),
        WorkConflict {
            conflict_id: conflict_id.clone(),
            attempt_ids: attempt_ids.into(),
            resource: resource.into(),
            evidence: evidence.into(),
            status: WorkConflictStatus::Open,
            resolution_decision_id: None,
        },
    );
    crate::verification_reducer::mark_coordination_changed(
        run,
        attempt_ids,
        "a coordination conflict changed the verification inputs",
    );
    Ok(())
}

pub(crate) fn resolve_conflict(
    run: &mut WorkRun,
    conflict_id: &WorkConflictId,
    decision_id: &WorkDecisionId,
) -> Result<(), WorkCoordinationError> {
    if !run.decisions.contains_key(decision_id) {
        return Err(WorkCoordinationError::NotFound(decision_id.to_string()));
    }
    let conflict = run
        .conflicts
        .get_mut(conflict_id)
        .ok_or_else(|| WorkCoordinationError::NotFound(conflict_id.to_string()))?;
    if conflict.status != WorkConflictStatus::Open {
        return Err(WorkCoordinationError::InvalidTransition(
            "only an open conflict can be resolved".into(),
        ));
    }
    conflict.status = WorkConflictStatus::Resolved;
    conflict.resolution_decision_id = Some(decision_id.clone());
    let attempt_ids = conflict.attempt_ids.clone();
    for attempt_id in attempt_ids {
        attempt_reducer::mark_stale(
            run,
            &attempt_id,
            "conflict resolution requires a new WorkAttempt",
        )?;
    }
    Ok(())
}

pub(crate) fn create_relation(
    run: &mut WorkRun,
    relation_id: &WorkRelationId,
    source_attempt_id: &WorkAttemptId,
    target_attempt_id: &WorkAttemptId,
    kind: &WorkRelationKind,
) -> Result<(), WorkCoordinationError> {
    if source_attempt_id == target_attempt_id {
        return Err(WorkCoordinationError::InvalidInput(
            "a work relation requires two distinct attempts".into(),
        ));
    }
    if run.relations.contains_key(relation_id) {
        return Err(WorkCoordinationError::AlreadyExists(
            relation_id.to_string(),
        ));
    }
    let source = run
        .attempts
        .get(source_attempt_id)
        .cloned()
        .ok_or_else(|| WorkCoordinationError::NotFound(source_attempt_id.to_string()))?;
    let target = run
        .attempts
        .get(target_attempt_id)
        .cloned()
        .ok_or_else(|| WorkCoordinationError::NotFound(target_attempt_id.to_string()))?;
    crate::dependency_graph::ensure_relation_acyclic(
        run,
        source_attempt_id,
        target_attempt_id,
        kind,
    )?;
    let (status, resume) = match kind {
        WorkRelationKind::Observation => (WorkRelationStatus::Active, None),
        WorkRelationKind::Alternate => {
            if source.roots != target.roots {
                return Err(WorkCoordinationError::InvalidInput(
                    "alternate attempts must begin from the same root checkpoints".into(),
                ));
            }
            (WorkRelationStatus::Active, None)
        }
        WorkRelationKind::Handoff { target_contract } => {
            if source.execution_status != WorkAttemptExecutionStatus::Sealed
                || target.execution_status != WorkAttemptExecutionStatus::Planned
                || &target.contract != target_contract
                || source.contract.contract_id != target.contract.contract_id
            {
                return Err(WorkCoordinationError::InvalidTransition(
                    "handoff requires a sealed source and planned target for the same contract"
                        .into(),
                ));
            }
            (WorkRelationStatus::Active, None)
        }
        WorkRelationKind::ResultDependency { result_digest } => {
            if target
                .result
                .as_ref()
                .is_none_or(|result| &result.result_digest != result_digest)
            {
                return Err(WorkCoordinationError::InvalidInput(
                    "result dependency does not match the target attempt result".into(),
                ));
            }
            (
                WorkRelationStatus::Satisfied {
                    evidence_digest: result_digest.clone(),
                },
                None,
            )
        }
        WorkRelationKind::Wait {
            target_execution_id,
            ..
        } => {
            if target.execution_id.as_ref() != Some(target_execution_id) || target.is_terminal() {
                return Err(WorkCoordinationError::InvalidInput(
                    "wait target must name the active target execution".into(),
                ));
            }
            if !matches!(
                source.execution_status,
                WorkAttemptExecutionStatus::Exploring | WorkAttemptExecutionStatus::Writing
            ) || source.coordination_status != WorkAttemptCoordinationStatus::Clear
                || source.waiting_relation_id.is_some()
            {
                return Err(WorkCoordinationError::InvalidTransition(
                    "only one clear active attempt can enter a wait".into(),
                ));
            }
            (WorkRelationStatus::Waiting, Some(source.execution_status))
        }
    };
    run.relations.insert(
        relation_id.clone(),
        WorkRelation {
            relation_id: relation_id.clone(),
            source_attempt_id: source_attempt_id.clone(),
            target_attempt_id: target_attempt_id.clone(),
            kind: kind.clone(),
            status,
            resume_execution_status: resume,
        },
    );
    if resume.is_some() {
        let source = attempt_reducer::attempt_mut(run, source_attempt_id)?;
        source.execution_status = WorkAttemptExecutionStatus::Waiting;
        source.waiting_relation_id = Some(relation_id.clone());
    }
    crate::verification_reducer::mark_coordination_changed(
        run,
        &[source_attempt_id.clone(), target_attempt_id.clone()],
        "a WorkAttempt relation changed the verification inputs",
    );
    Ok(())
}

pub(crate) fn resolve_wait(
    run: &mut WorkRun,
    relation_id: &WorkRelationId,
    target_attempt_id: &WorkAttemptId,
    target_execution_id: &WorkExecutionId,
    outcome: &ResolveWaitOutcome,
) -> Result<(), WorkCoordinationError> {
    let relation = run
        .relations
        .get(relation_id)
        .cloned()
        .ok_or_else(|| WorkCoordinationError::NotFound(relation_id.to_string()))?;
    let WorkRelationKind::Wait {
        target_execution_id: expected_execution_id,
        condition,
    } = &relation.kind
    else {
        return Err(WorkCoordinationError::InvalidTransition(
            "only a wait relation can be resolved as a wait".into(),
        ));
    };
    if relation.status != WorkRelationStatus::Waiting
        || &relation.target_attempt_id != target_attempt_id
        || expected_execution_id != target_execution_id
    {
        return Err(WorkCoordinationError::InvalidInput(
            "wait resolution does not match the frozen target attempt and execution".into(),
        ));
    }
    let target = run
        .attempts
        .get(target_attempt_id)
        .cloned()
        .ok_or_else(|| WorkCoordinationError::NotFound(target_attempt_id.to_string()))?;
    if !matches!(outcome, ResolveWaitOutcome::Cancelled) {
        let expected = crate::wait_reconciliation::expected_wait_outcome(
            condition,
            &target,
            target_execution_id,
        )?;
        if expected.as_ref() != Some(outcome) {
            return Err(WorkCoordinationError::InvalidTransition(
                "wait resolution does not match the outcome derived from the frozen target".into(),
            ));
        }
    }
    let status = wait_status(condition, &target, outcome)?;
    run.relations
        .get_mut(relation_id)
        .expect("relation existence checked above")
        .status = status.clone();
    let source = attempt_reducer::attempt_mut(run, &relation.source_attempt_id)?;
    source.waiting_relation_id = None;
    match status {
        WorkRelationStatus::Satisfied { .. } => {
            source.execution_status = relation.resume_execution_status.ok_or_else(|| {
                WorkCoordinationError::InvalidTransition(
                    "wait relation omitted its resume status".into(),
                )
            })?;
        }
        WorkRelationStatus::Failed { ref reason } => {
            source.execution_status = WorkAttemptExecutionStatus::Failed;
            source.failure = Some(reason.clone());
        }
        WorkRelationStatus::Cancelled => {
            source.execution_status = WorkAttemptExecutionStatus::Interrupted;
            source.failure = Some("wait was cancelled".into());
        }
        WorkRelationStatus::Stale => {
            source.execution_status = WorkAttemptExecutionStatus::Interrupted;
            source.coordination_status = WorkAttemptCoordinationStatus::Stale;
            source.verification_status = WorkAttemptVerificationStatus::Stale;
            source.failure = Some("wait source became stale".into());
        }
        WorkRelationStatus::Active | WorkRelationStatus::Waiting => {
            unreachable!("wait resolution always produces a terminal relation status")
        }
    }
    Ok(())
}

fn wait_status(
    condition: &WorkWaitCondition,
    target: &WorkAttempt,
    outcome: &ResolveWaitOutcome,
) -> Result<WorkRelationStatus, WorkCoordinationError> {
    match outcome {
        ResolveWaitOutcome::Satisfied { evidence_digest } => {
            let satisfied = match condition {
                WorkWaitCondition::ExecutionFinished => target.is_terminal(),
                WorkWaitCondition::AttemptSealed => {
                    target.execution_status == WorkAttemptExecutionStatus::Sealed
                }
                WorkWaitCondition::ExactResult {
                    result_digest: expected,
                } => {
                    expected == evidence_digest
                        && target
                            .result
                            .as_ref()
                            .is_some_and(|result| result.result_digest == *expected)
                }
            };
            if !satisfied {
                return Err(WorkCoordinationError::InvalidTransition(
                    "wait condition is not satisfied by the target attempt".into(),
                ));
            }
            Ok(WorkRelationStatus::Satisfied {
                evidence_digest: evidence_digest.clone(),
            })
        }
        ResolveWaitOutcome::Failed { reason } => {
            validation::text("wait failure", reason)?;
            Ok(WorkRelationStatus::Failed {
                reason: reason.clone(),
            })
        }
        ResolveWaitOutcome::Cancelled => Ok(WorkRelationStatus::Cancelled),
        ResolveWaitOutcome::SourceStale => Ok(WorkRelationStatus::Stale),
    }
}
