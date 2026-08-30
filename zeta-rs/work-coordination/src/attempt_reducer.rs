use crate::ManagedRootBinding;
use crate::WorkAttempt;
use crate::WorkAttemptCoordinationStatus;
use crate::WorkAttemptExecutionStatus;
use crate::WorkAttemptResult;
use crate::WorkAttemptWorkspace;
use crate::WorkCoordinationError;
use crate::WorkRelationStatus;
use crate::WorkRun;
use crate::WorkStartMode;
use crate::validation;
use zeta_file_access::DirId;
use zeta_protocol::ContentDigest;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkExecutionId;
use zeta_turn_changes::ChangeSetId;

pub(crate) fn begin(
    run: &mut WorkRun,
    attempt_id: &WorkAttemptId,
    execution_id: &WorkExecutionId,
    mode: WorkStartMode,
) -> Result<(), WorkCoordinationError> {
    let thread_id = run
        .attempts
        .get(attempt_id)
        .ok_or_else(|| WorkCoordinationError::NotFound(attempt_id.to_string()))?
        .thread_id
        .clone();
    if run.attempts.values().any(|attempt| {
        &attempt.attempt_id != attempt_id
            && attempt.thread_id == thread_id
            && matches!(
                attempt.execution_status,
                WorkAttemptExecutionStatus::Exploring | WorkAttemptExecutionStatus::Writing
            )
    }) {
        return Err(WorkCoordinationError::InvalidTransition(
            "one Thread cannot execute more than one WorkAttempt at a time".into(),
        ));
    }
    if run
        .attempts
        .values()
        .any(|attempt| attempt.execution_id.as_ref() == Some(execution_id))
    {
        return Err(WorkCoordinationError::AlreadyExists(
            execution_id.to_string(),
        ));
    }
    let attempt = attempt_mut(run, attempt_id)?;
    if attempt.execution_status != WorkAttemptExecutionStatus::Planned
        || attempt.coordination_status != WorkAttemptCoordinationStatus::Clear
        || attempt.execution_id.is_some()
    {
        return Err(WorkCoordinationError::InvalidTransition(
            "only one clear planned attempt can begin".into(),
        ));
    }
    if !attempt.workspace.is_ready() {
        return Err(WorkCoordinationError::InvalidTransition(
            "a WorkAttempt cannot begin before its managed root set is ready".into(),
        ));
    }
    attempt.execution_id = Some(execution_id.clone());
    attempt.execution_status = match mode {
        WorkStartMode::Explore => WorkAttemptExecutionStatus::Exploring,
        WorkStartMode::Write => WorkAttemptExecutionStatus::Writing,
    };
    Ok(())
}

pub(crate) fn record_workspace_ready(
    run: &mut WorkRun,
    attempt_id: &WorkAttemptId,
    roots: &[ManagedRootBinding],
    private_output_dir_id: &DirId,
) -> Result<(), WorkCoordinationError> {
    let attempt = attempt_mut(run, attempt_id)?;
    if attempt.execution_status != WorkAttemptExecutionStatus::Planned
        || attempt.coordination_status != WorkAttemptCoordinationStatus::Clear
        || !matches!(attempt.workspace, WorkAttemptWorkspace::Provisioning)
    {
        return Err(WorkCoordinationError::InvalidTransition(
            "only a clear planned attempt can finish workspace provisioning".into(),
        ));
    }
    validation::workspace_bindings(&attempt.roots, roots, private_output_dir_id)?;
    attempt.workspace = WorkAttemptWorkspace::Ready {
        roots: roots.into(),
        private_output_dir_id: private_output_dir_id.clone(),
    };
    Ok(())
}

pub(crate) fn fail_workspace(
    run: &mut WorkRun,
    attempt_id: &WorkAttemptId,
    reason: &str,
) -> Result<(), WorkCoordinationError> {
    validation::text("workspace provisioning failure", reason)?;
    let attempt = attempt_mut(run, attempt_id)?;
    if attempt.execution_status != WorkAttemptExecutionStatus::Planned
        || !matches!(attempt.workspace, WorkAttemptWorkspace::Provisioning)
    {
        return Err(WorkCoordinationError::InvalidTransition(
            "only a provisioning attempt can record workspace failure".into(),
        ));
    }
    attempt.workspace = WorkAttemptWorkspace::Failed {
        reason: reason.into(),
    };
    attempt.execution_status = WorkAttemptExecutionStatus::Failed;
    attempt.failure = Some(reason.into());
    Ok(())
}

pub(crate) fn request_scope_expansion(
    run: &mut WorkRun,
    attempt_id: &WorkAttemptId,
    evidence: &[String],
) -> Result<(), WorkCoordinationError> {
    validation::non_empty_texts("scope expansion evidence", evidence)?;
    {
        let attempt = active_attempt_mut(run, attempt_id)?;
        if attempt.coordination_status != WorkAttemptCoordinationStatus::Clear {
            return Err(WorkCoordinationError::InvalidTransition(
                "scope expansion requires a clear attempt".into(),
            ));
        }
        attempt.scope_expansion_evidence = evidence.into();
    }
    stop_for_coordination(
        run,
        attempt_id,
        WorkAttemptCoordinationStatus::ExpansionRequested,
        "scope expansion requires a new contract and WorkAttempt",
    )?;
    Ok(())
}

pub(crate) fn stop_for_coordination(
    run: &mut WorkRun,
    attempt_id: &WorkAttemptId,
    coordination_status: WorkAttemptCoordinationStatus,
    reason: &str,
) -> Result<(), WorkCoordinationError> {
    validation::text("coordination stop reason", reason)?;
    let waiting_relation = {
        let attempt = attempt_mut(run, attempt_id)?;
        attempt.coordination_status = coordination_status;
        if matches!(
            attempt.execution_status,
            WorkAttemptExecutionStatus::Exploring
                | WorkAttemptExecutionStatus::Writing
                | WorkAttemptExecutionStatus::Waiting
        ) {
            attempt.execution_status = WorkAttemptExecutionStatus::Interrupted;
            attempt.failure = Some(reason.into());
        }
        attempt.waiting_relation_id.take()
    };
    if let Some(relation_id) = waiting_relation
        && let Some(relation) = run.relations.get_mut(&relation_id)
        && relation.status == WorkRelationStatus::Waiting
    {
        relation.status = WorkRelationStatus::Stale;
    }
    Ok(())
}

pub(crate) fn mark_stale(
    run: &mut WorkRun,
    attempt_id: &WorkAttemptId,
    reason: &str,
) -> Result<(), WorkCoordinationError> {
    stop_for_coordination(
        run,
        attempt_id,
        WorkAttemptCoordinationStatus::Stale,
        reason,
    )?;
    let attempt = attempt_mut(run, attempt_id)?;
    attempt.verification_status = crate::WorkAttemptVerificationStatus::Stale;
    if attempt.execution_status == WorkAttemptExecutionStatus::Planned {
        attempt.execution_status = WorkAttemptExecutionStatus::Cancelled;
        attempt.failure = Some(reason.into());
    }
    crate::verification_reducer::mark_attempt_stale(run, attempt_id, reason);
    Ok(())
}

pub(crate) fn seal(
    run: &mut WorkRun,
    attempt_id: &WorkAttemptId,
    result_digest: &ContentDigest,
    change_set_ids: &[ChangeSetId],
    private_output_digest: &ContentDigest,
    external_effects_digest: &ContentDigest,
    external_effects_status: crate::ExternalEffectsStatus,
) -> Result<(), WorkCoordinationError> {
    let attempt = active_attempt_mut(run, attempt_id)?;
    if attempt.coordination_status != WorkAttemptCoordinationStatus::Clear {
        return Err(WorkCoordinationError::InvalidTransition(
            "an attempt with unresolved coordination state cannot be sealed".into(),
        ));
    }
    if change_set_ids.iter().enumerate().any(|(index, id)| {
        change_set_ids[..index]
            .iter()
            .any(|existing| existing == id)
    }) {
        return Err(WorkCoordinationError::InvalidInput(
            "sealed ChangeSet identities must be unique".into(),
        ));
    }
    attempt.execution_status = WorkAttemptExecutionStatus::Sealed;
    attempt.result = Some(WorkAttemptResult {
        result_digest: result_digest.clone(),
        change_set_ids: change_set_ids.into(),
        private_output_digest: private_output_digest.clone(),
        external_effects_digest: external_effects_digest.clone(),
        external_effects_status,
    });
    Ok(())
}

pub(crate) fn finish(
    run: &mut WorkRun,
    attempt_id: &WorkAttemptId,
    status: WorkAttemptExecutionStatus,
    message: &str,
) -> Result<(), WorkCoordinationError> {
    validation::text("attempt terminal reason", message)?;
    let waiting_relation = {
        let attempt = attempt_mut(run, attempt_id)?;
        if attempt.is_terminal() {
            return Err(WorkCoordinationError::InvalidTransition(
                "a terminal attempt cannot transition again".into(),
            ));
        }
        attempt.execution_status = status;
        attempt.failure = Some(message.into());
        attempt.waiting_relation_id.take()
    };
    if let Some(relation_id) = waiting_relation
        && let Some(relation) = run.relations.get_mut(&relation_id)
        && relation.status == WorkRelationStatus::Waiting
    {
        relation.status = WorkRelationStatus::Cancelled;
    }
    Ok(())
}

pub(crate) fn attempt_mut<'a>(
    run: &'a mut WorkRun,
    attempt_id: &WorkAttemptId,
) -> Result<&'a mut WorkAttempt, WorkCoordinationError> {
    run.attempts
        .get_mut(attempt_id)
        .ok_or_else(|| WorkCoordinationError::NotFound(attempt_id.to_string()))
}

pub(crate) fn active_attempt_mut<'a>(
    run: &'a mut WorkRun,
    attempt_id: &WorkAttemptId,
) -> Result<&'a mut WorkAttempt, WorkCoordinationError> {
    let attempt = attempt_mut(run, attempt_id)?;
    if !matches!(
        attempt.execution_status,
        WorkAttemptExecutionStatus::Exploring | WorkAttemptExecutionStatus::Writing
    ) {
        return Err(WorkCoordinationError::InvalidTransition(
            "attempt must be actively exploring or writing".into(),
        ));
    }
    Ok(attempt)
}
