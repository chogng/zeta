use crate::IntegrationFailureKind;
use crate::IntegrationIncident;
use crate::IntegrationPreparedArtifact;
use crate::IntegrationRootStatus;
use crate::WorkAttemptIntegrationStatus;
use crate::WorkCoordinationError;
use crate::WorkIntegration;
use crate::WorkIntegrationStatus;
use crate::WorkRun;
use crate::WorkVerificationStatus;
use crate::integration::integration_evidence_digest;
use crate::integration::integration_roots;
use crate::integration::validate_prepared_artifact;
use crate::integration_key;
use zeta_protocol::ContentDigest;

pub(crate) fn queue(
    run: &mut WorkRun,
    verification_key: &ContentDigest,
) -> Result<(), WorkCoordinationError> {
    let verification = run
        .verifications
        .get(verification_key)
        .ok_or_else(|| WorkCoordinationError::NotFound(verification_key.to_string()))?;
    if verification.status != WorkVerificationStatus::Verified {
        return Err(WorkCoordinationError::InvalidTransition(
            "integration accepts only a current verified input".into(),
        ));
    }
    let key = integration_key(&run.work_run_id, verification_key)?;
    if run.integrations.contains_key(&key) {
        return Err(WorkCoordinationError::AlreadyExists(key.to_string()));
    }
    for result in &verification.input.ordered_results {
        let attempt = run
            .attempts
            .get(&result.attempt_id)
            .ok_or_else(|| WorkCoordinationError::NotFound(result.attempt_id.to_string()))?;
        if attempt.verification_status != crate::WorkAttemptVerificationStatus::Verified
            || !matches!(
                attempt.integration_status,
                WorkAttemptIntegrationStatus::Idle
                    | WorkAttemptIntegrationStatus::Conflict
                    | WorkAttemptIntegrationStatus::Failed
            )
        {
            return Err(WorkCoordinationError::InvalidTransition(
                "integration requires verified results that are not published or partial".into(),
            ));
        }
    }
    let roots = integration_roots(verification)?;
    for result in &verification.input.ordered_results {
        run.attempts
            .get_mut(&result.attempt_id)
            .ok_or_else(|| WorkCoordinationError::NotFound(result.attempt_id.to_string()))?
            .integration_status = WorkAttemptIntegrationStatus::Queued;
    }
    run.integrations.insert(
        key.clone(),
        WorkIntegration {
            integration_key: key,
            verification_key: verification_key.clone(),
            generation: 1,
            status: WorkIntegrationStatus::Queued,
            roots,
            incidents: Vec::new(),
            evidence_digest: None,
        },
    );
    Ok(())
}

pub(crate) fn record_prepared(
    run: &mut WorkRun,
    integration_key: &ContentDigest,
    generation: u64,
    root_id: &ContentDigest,
    artifact: &IntegrationPreparedArtifact,
) -> Result<(), WorkCoordinationError> {
    let integration = active(
        run,
        integration_key,
        generation,
        WorkIntegrationStatus::Queued,
    )?;
    let root = integration
        .roots
        .iter_mut()
        .find(|root| &root.root_id == root_id)
        .ok_or_else(|| WorkCoordinationError::NotFound(root_id.to_string()))?;
    if root.status != IntegrationRootStatus::Pending
        || root.prepared_artifact.is_some()
        || root.publication_receipt_digest.is_some()
    {
        return Err(WorkCoordinationError::InvalidTransition(
            "integration root is not awaiting preparation".into(),
        ));
    }
    validate_prepared_artifact(root, artifact)?;
    root.prepared_artifact = Some(artifact.clone());
    root.status = IntegrationRootStatus::Prepared;
    Ok(())
}

pub(crate) fn begin(
    run: &mut WorkRun,
    integration_key: &ContentDigest,
    generation: u64,
) -> Result<(), WorkCoordinationError> {
    let verification_key = run
        .integrations
        .get(integration_key)
        .ok_or_else(|| WorkCoordinationError::NotFound(integration_key.to_string()))?
        .verification_key
        .clone();
    let result_ids = verification_result_ids(run, &verification_key)?;
    {
        let integration = active(
            run,
            integration_key,
            generation,
            WorkIntegrationStatus::Queued,
        )?;
        if integration
            .roots
            .iter()
            .any(|root| root.status != IntegrationRootStatus::Prepared)
        {
            return Err(WorkCoordinationError::InvalidTransition(
                "integration cannot publish before every root is prepared".into(),
            ));
        }
        integration.status = WorkIntegrationStatus::Integrating;
    }
    set_attempt_status(run, &result_ids, WorkAttemptIntegrationStatus::Integrating)
}

pub(crate) fn record_published(
    run: &mut WorkRun,
    integration_key: &ContentDigest,
    generation: u64,
    root_id: &ContentDigest,
    receipt_digest: &ContentDigest,
) -> Result<(), WorkCoordinationError> {
    let verification_key = run
        .integrations
        .get(integration_key)
        .ok_or_else(|| WorkCoordinationError::NotFound(integration_key.to_string()))?
        .verification_key
        .clone();
    let result_ids = verification_result_ids(run, &verification_key)?;
    let completed = {
        let integration = active(
            run,
            integration_key,
            generation,
            WorkIntegrationStatus::Integrating,
        )?;
        let next = integration
            .roots
            .iter_mut()
            .find(|root| root.status != IntegrationRootStatus::Published)
            .ok_or_else(|| {
                WorkCoordinationError::InvalidTransition(
                    "integration has no unpublished root".into(),
                )
            })?;
        if &next.root_id != root_id || next.status != IntegrationRootStatus::Prepared {
            return Err(WorkCoordinationError::InvalidTransition(
                "integration roots must publish in their recorded order".into(),
            ));
        }
        next.status = IntegrationRootStatus::Published;
        next.publication_receipt_digest = Some(receipt_digest.clone());
        let completed = integration
            .roots
            .iter()
            .all(|root| root.status == IntegrationRootStatus::Published);
        if completed {
            integration.status = WorkIntegrationStatus::Integrated;
            integration.evidence_digest = Some(integration_evidence_digest(integration)?);
        }
        completed
    };
    if completed {
        set_attempt_status(run, &result_ids, WorkAttemptIntegrationStatus::Integrated)?;
    }
    Ok(())
}

pub(crate) fn fail(
    run: &mut WorkRun,
    integration_key: &ContentDigest,
    generation: u64,
    kind: IntegrationFailureKind,
    reason: &str,
) -> Result<(), WorkCoordinationError> {
    crate::validation::text("integration incident reason", reason)?;
    let verification_key = run
        .integrations
        .get(integration_key)
        .ok_or_else(|| WorkCoordinationError::NotFound(integration_key.to_string()))?
        .verification_key
        .clone();
    let result_ids = verification_result_ids(run, &verification_key)?;
    let status = {
        let integration = run
            .integrations
            .get_mut(integration_key)
            .ok_or_else(|| WorkCoordinationError::NotFound(integration_key.to_string()))?;
        if integration.generation != generation
            || !matches!(
                integration.status,
                WorkIntegrationStatus::Queued | WorkIntegrationStatus::Integrating
            )
        {
            return Err(WorkCoordinationError::InvalidTransition(
                "integration incident does not match the active generation".into(),
            ));
        }
        let published = integration
            .roots
            .iter()
            .filter(|root| root.status == IntegrationRootStatus::Published)
            .count();
        let published_root_count = u64::try_from(published).map_err(|_| {
            WorkCoordinationError::InvalidInput("too many integration roots".into())
        })?;
        integration.incidents.push(IntegrationIncident {
            generation,
            kind,
            reason: reason.into(),
            published_root_count,
        });
        let status = if published > 0 {
            WorkIntegrationStatus::Partial
        } else {
            match kind {
                IntegrationFailureKind::Conflict | IntegrationFailureKind::TargetMoved => {
                    WorkIntegrationStatus::Conflict
                }
                IntegrationFailureKind::Failure => WorkIntegrationStatus::Failed,
            }
        };
        integration.status = status;
        integration.evidence_digest = Some(integration_evidence_digest(integration)?);
        status
    };
    let attempt_status = match status {
        WorkIntegrationStatus::Partial => WorkAttemptIntegrationStatus::Partial,
        WorkIntegrationStatus::Conflict => WorkAttemptIntegrationStatus::Conflict,
        WorkIntegrationStatus::Failed => WorkAttemptIntegrationStatus::Failed,
        WorkIntegrationStatus::Queued
        | WorkIntegrationStatus::Integrating
        | WorkIntegrationStatus::Integrated => {
            return Err(WorkCoordinationError::InvalidTransition(
                "integration incident produced a non-terminal status".into(),
            ));
        }
    };
    set_attempt_status(run, &result_ids, attempt_status)?;
    if kind == IntegrationFailureKind::TargetMoved {
        crate::verification_reducer::mark_stale(
            run,
            &verification_key,
            "integration target moved after verification",
        )?;
    }
    Ok(())
}

pub(crate) fn resume(
    run: &mut WorkRun,
    integration_key: &ContentDigest,
    generation: u64,
) -> Result<(), WorkCoordinationError> {
    let verification_key = run
        .integrations
        .get(integration_key)
        .ok_or_else(|| WorkCoordinationError::NotFound(integration_key.to_string()))?
        .verification_key
        .clone();
    let verification = run
        .verifications
        .get(&verification_key)
        .ok_or_else(|| WorkCoordinationError::NotFound(verification_key.to_string()))?;
    if verification.status != WorkVerificationStatus::Verified {
        return Err(WorkCoordinationError::InvalidTransition(
            "stale or rejected verification cannot resume publication".into(),
        ));
    }
    let result_ids = verification_result_ids(run, &verification_key)?;
    let next_status = {
        let integration = run
            .integrations
            .get_mut(integration_key)
            .ok_or_else(|| WorkCoordinationError::NotFound(integration_key.to_string()))?;
        if integration.generation != generation
            || !matches!(
                integration.status,
                WorkIntegrationStatus::Conflict
                    | WorkIntegrationStatus::Failed
                    | WorkIntegrationStatus::Partial
            )
        {
            return Err(WorkCoordinationError::InvalidTransition(
                "only a stopped integration generation can resume".into(),
            ));
        }
        integration.generation = integration.generation.checked_add(1).ok_or_else(|| {
            WorkCoordinationError::InvalidTransition("generation overflow".into())
        })?;
        integration.evidence_digest = None;
        let next_status = if integration
            .roots
            .iter()
            .all(|root| root.status != IntegrationRootStatus::Pending)
        {
            WorkIntegrationStatus::Integrating
        } else {
            WorkIntegrationStatus::Queued
        };
        integration.status = next_status;
        next_status
    };
    set_attempt_status(
        run,
        &result_ids,
        match next_status {
            WorkIntegrationStatus::Queued => WorkAttemptIntegrationStatus::Queued,
            WorkIntegrationStatus::Integrating => WorkAttemptIntegrationStatus::Integrating,
            _ => {
                return Err(WorkCoordinationError::InvalidTransition(
                    "resumed integration has an invalid state".into(),
                ));
            }
        },
    )
}

fn active<'a>(
    run: &'a mut WorkRun,
    integration_key: &ContentDigest,
    generation: u64,
    status: WorkIntegrationStatus,
) -> Result<&'a mut WorkIntegration, WorkCoordinationError> {
    let integration = run
        .integrations
        .get_mut(integration_key)
        .ok_or_else(|| WorkCoordinationError::NotFound(integration_key.to_string()))?;
    if integration.generation != generation || integration.status != status {
        return Err(WorkCoordinationError::InvalidTransition(
            "integration command does not match its active generation and state".into(),
        ));
    }
    Ok(integration)
}

fn verification_result_ids(
    run: &WorkRun,
    verification_key: &ContentDigest,
) -> Result<Vec<zeta_protocol::WorkAttemptId>, WorkCoordinationError> {
    Ok(run
        .verifications
        .get(verification_key)
        .ok_or_else(|| WorkCoordinationError::NotFound(verification_key.to_string()))?
        .input
        .ordered_results
        .iter()
        .map(|result| result.attempt_id.clone())
        .collect())
}

fn set_attempt_status(
    run: &mut WorkRun,
    attempt_ids: &[zeta_protocol::WorkAttemptId],
    status: WorkAttemptIntegrationStatus,
) -> Result<(), WorkCoordinationError> {
    for attempt_id in attempt_ids {
        run.attempts
            .get_mut(attempt_id)
            .ok_or_else(|| WorkCoordinationError::NotFound(attempt_id.to_string()))?
            .integration_status = status;
    }
    Ok(())
}
