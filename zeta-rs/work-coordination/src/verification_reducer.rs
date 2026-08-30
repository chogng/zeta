use crate::VerificationCheckEvidence;
use crate::VerificationCheckOutcome;
use crate::VerificationConclusion;
use crate::WorkAttemptIntegrationStatus;
use crate::WorkAttemptVerificationStatus;
use crate::WorkCoordinationError;
use crate::WorkRun;
use crate::WorkVerification;
use crate::WorkVerificationInput;
use crate::WorkVerificationStatus;
use crate::root_checkpoint_digest;
use crate::verification::verification_evidence_digest;
use crate::verification_key;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use zeta_protocol::ContentDigest;

pub(crate) fn begin(
    run: &mut WorkRun,
    input: &WorkVerificationInput,
) -> Result<(), WorkCoordinationError> {
    validate_input(run, input)?;
    let key = verification_key(&run.work_run_id, input)?;
    if run.verifications.contains_key(&key) {
        return Err(WorkCoordinationError::AlreadyExists(key.to_string()));
    }
    for result in &input.ordered_results {
        let attempt = run
            .attempts
            .get_mut(&result.attempt_id)
            .ok_or_else(|| WorkCoordinationError::NotFound(result.attempt_id.to_string()))?;
        attempt.verification_status = WorkAttemptVerificationStatus::Verifying;
    }
    run.verifications.insert(
        key.clone(),
        WorkVerification {
            verification_key: key,
            input: input.clone(),
            status: WorkVerificationStatus::Verifying,
            checks: Vec::new(),
            evidence_digest: None,
            reason: None,
            stale_reason: None,
        },
    );
    Ok(())
}

pub(crate) fn finish(
    run: &mut WorkRun,
    key: &ContentDigest,
    conclusion: VerificationConclusion,
    checks: &[VerificationCheckEvidence],
    reason: &str,
) -> Result<(), WorkCoordinationError> {
    crate::validation::text("verification conclusion reason", reason)?;
    validate_checks(conclusion, checks)?;
    if conclusion == VerificationConclusion::Verified {
        let verification = run
            .verifications
            .get(key)
            .ok_or_else(|| WorkCoordinationError::NotFound(key.to_string()))?;
        if verification.input.serializability.status != crate::WorkSerializabilityStatus::Proven {
            return Err(WorkCoordinationError::InvalidInput(
                "indeterminate serializability cannot produce a verified conclusion".into(),
            ));
        }
        if verification.input.ordered_results.iter().any(|result| {
            run.attempts
                .get(&result.attempt_id)
                .and_then(|attempt| attempt.result.as_ref())
                .is_none_or(|result| {
                    result.external_effects_status == crate::ExternalEffectsStatus::Unknown
                })
        }) {
            return Err(WorkCoordinationError::InvalidInput(
                "unknown external effects cannot produce a verified conclusion".into(),
            ));
        }
    }
    let verification = run
        .verifications
        .get_mut(key)
        .ok_or_else(|| WorkCoordinationError::NotFound(key.to_string()))?;
    if verification.status != WorkVerificationStatus::Verifying {
        return Err(WorkCoordinationError::InvalidTransition(
            "only an active verification can record a conclusion".into(),
        ));
    }
    let status = match conclusion {
        VerificationConclusion::Verified => WorkVerificationStatus::Verified,
        VerificationConclusion::Rejected => WorkVerificationStatus::Rejected,
        VerificationConclusion::Indeterminate => WorkVerificationStatus::Indeterminate,
    };
    verification.status = status;
    verification.checks = checks.into();
    verification.evidence_digest = Some(verification_evidence_digest(
        key, conclusion, checks, reason,
    )?);
    verification.reason = Some(reason.into());
    let attempt_status = match conclusion {
        VerificationConclusion::Verified => WorkAttemptVerificationStatus::Verified,
        VerificationConclusion::Rejected => WorkAttemptVerificationStatus::Rejected,
        VerificationConclusion::Indeterminate => WorkAttemptVerificationStatus::Indeterminate,
    };
    for result in &verification.input.ordered_results {
        run.attempts
            .get_mut(&result.attempt_id)
            .ok_or_else(|| WorkCoordinationError::NotFound(result.attempt_id.to_string()))?
            .verification_status = attempt_status;
    }
    Ok(())
}

pub(crate) fn mark_stale(
    run: &mut WorkRun,
    key: &ContentDigest,
    reason: &str,
) -> Result<(), WorkCoordinationError> {
    crate::validation::text("verification stale reason", reason)?;
    let verification = run
        .verifications
        .get_mut(key)
        .ok_or_else(|| WorkCoordinationError::NotFound(key.to_string()))?;
    if verification.status == WorkVerificationStatus::Stale {
        return Err(WorkCoordinationError::InvalidTransition(
            "verification is already stale".into(),
        ));
    }
    verification.status = WorkVerificationStatus::Stale;
    verification.stale_reason = Some(reason.into());
    for result in &verification.input.ordered_results {
        let attempt = run
            .attempts
            .get_mut(&result.attempt_id)
            .ok_or_else(|| WorkCoordinationError::NotFound(result.attempt_id.to_string()))?;
        if attempt.integration_status != WorkAttemptIntegrationStatus::Integrated {
            attempt.verification_status = WorkAttemptVerificationStatus::Stale;
        }
    }
    Ok(())
}

pub(crate) fn mark_attempt_stale(
    run: &mut WorkRun,
    attempt_id: &zeta_protocol::WorkAttemptId,
    reason: &str,
) {
    mark_coordination_changed(run, std::slice::from_ref(attempt_id), reason);
}

pub(crate) fn mark_coordination_changed(
    run: &mut WorkRun,
    attempt_ids: &[zeta_protocol::WorkAttemptId],
    reason: &str,
) {
    let attempt_ids = attempt_ids.iter().collect::<BTreeSet<_>>();
    let mut affected = BTreeSet::new();
    for verification in run.verifications.values_mut().filter(|verification| {
        verification.status != WorkVerificationStatus::Stale
            && verification
                .input
                .ordered_results
                .iter()
                .any(|result| attempt_ids.contains(&result.attempt_id))
    }) {
        verification.status = WorkVerificationStatus::Stale;
        verification.stale_reason = Some(reason.into());
        affected.extend(
            verification
                .input
                .ordered_results
                .iter()
                .map(|result| result.attempt_id.clone()),
        );
    }
    for attempt_id in affected {
        if let Some(attempt) = run.attempts.get_mut(&attempt_id)
            && attempt.integration_status != WorkAttemptIntegrationStatus::Integrated
        {
            attempt.verification_status = WorkAttemptVerificationStatus::Stale;
        }
    }
}

fn validate_input(
    run: &WorkRun,
    input: &WorkVerificationInput,
) -> Result<(), WorkCoordinationError> {
    if input.goal_revision != run.current_goal().map(|goal| goal.revision).unwrap_or(0)
        || input.topology_revision != run.topology_revision
    {
        return Err(WorkCoordinationError::InvalidInput(
            "verification input does not use the current goal and topology".into(),
        ));
    }
    if input.ordered_results.is_empty() || input.roots.is_empty() {
        return Err(WorkCoordinationError::InvalidInput(
            "verification requires results and final root states".into(),
        ));
    }
    crate::validation::text(
        "serializability evidence reason",
        &input.serializability.reason,
    )?;
    let mut attempt_ids = BTreeSet::new();
    let mut expected_changes = Vec::new();
    let mut expected_authorizations = BTreeSet::new();
    let mut expected_controls = BTreeSet::new();
    let mut expected_profiles = BTreeSet::new();
    let mut expected_roots = BTreeMap::new();
    for result in &input.ordered_results {
        if !attempt_ids.insert(result.attempt_id.clone()) {
            return Err(WorkCoordinationError::InvalidInput(
                "verification repeats a WorkAttempt result".into(),
            ));
        }
        let attempt = run
            .attempts
            .get(&result.attempt_id)
            .ok_or_else(|| WorkCoordinationError::NotFound(result.attempt_id.to_string()))?;
        let sealed = attempt.result.as_ref().ok_or_else(|| {
            WorkCoordinationError::InvalidInput("verification result is not sealed".into())
        })?;
        if sealed.result_digest != result.result_digest
            || attempt.coordination_status != crate::WorkAttemptCoordinationStatus::Clear
            || !matches!(
                attempt.verification_status,
                WorkAttemptVerificationStatus::Pending | WorkAttemptVerificationStatus::Stale
            )
            || !matches!(
                attempt.integration_status,
                WorkAttemptIntegrationStatus::Idle
                    | WorkAttemptIntegrationStatus::Conflict
                    | WorkAttemptIntegrationStatus::Failed
            )
        {
            return Err(WorkCoordinationError::InvalidTransition(
                "verification requires clear, pending, unintegrated sealed results".into(),
            ));
        }
        if run.verifications.values().any(|verification| {
            verification.status != WorkVerificationStatus::Stale
                && verification
                    .input
                    .ordered_results
                    .iter()
                    .any(|existing| existing.attempt_id == attempt.attempt_id)
        }) {
            return Err(WorkCoordinationError::InvalidTransition(
                "WorkAttempt already belongs to a current verification".into(),
            ));
        }
        expected_changes.extend(
            sealed
                .change_set_ids
                .iter()
                .map(|change_set_id| (attempt.attempt_id.clone(), change_set_id.clone())),
        );
        let contract = run
            .contract(&attempt.contract.contract_id, attempt.contract.revision)
            .ok_or_else(|| {
                WorkCoordinationError::NotFound(attempt.contract.contract_id.to_string())
            })?;
        expected_authorizations.insert(contract.authorization.grant_set_digest.clone());
        expected_authorizations.insert(contract.authorization.granted_effects_digest.clone());
        expected_profiles.insert(contract.validation_profile.content_digest.clone());
        for root in &attempt.roots {
            let digest = root_checkpoint_digest(root)?;
            if let Some(previous) = expected_roots.insert(root.dir_id.clone(), digest.clone())
                && previous != digest
            {
                return Err(WorkCoordinationError::InvalidInput(
                    "attempts use incompatible checkpoints for one root".into(),
                ));
            }
            expected_controls.extend(
                root.control_resources
                    .iter()
                    .map(|resource| resource.content_digest.clone()),
            );
        }
    }
    crate::dependency_graph::validate_order(run, &input.ordered_results)?;
    let actual_changes = input
        .ordered_change_sets
        .iter()
        .map(|change| {
            (
                change.attempt_id.clone(),
                change.change_set.change_set_id.clone(),
            )
        })
        .collect::<Vec<_>>();
    if actual_changes != expected_changes {
        return Err(WorkCoordinationError::InvalidInput(
            "verification ChangeSet order does not match the sealed result order".into(),
        ));
    }
    if input.coordination_digest
        != crate::verification_coordination_digest(run, &input.ordered_results)?
    {
        return Err(WorkCoordinationError::InvalidInput(
            "verification coordination evidence does not match the selected WorkAttempts".into(),
        ));
    }
    let actual_roots = input
        .roots
        .iter()
        .map(|root| (root.source_dir_id.clone(), root.checkpoint_digest.clone()))
        .collect::<BTreeMap<_, _>>();
    if actual_roots.len() != input.roots.len()
        || actual_roots != expected_roots
        || input.authorization_digests != expected_authorizations
        || input.control_resource_digests != expected_controls
        || input.validation_profile_digests != expected_profiles
    {
        return Err(WorkCoordinationError::InvalidInput(
            "verification roots, authorization, controls, or profile do not match the contracts"
                .into(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_checks(
    conclusion: VerificationConclusion,
    checks: &[VerificationCheckEvidence],
) -> Result<(), WorkCoordinationError> {
    if checks.is_empty() {
        return Err(WorkCoordinationError::InvalidInput(
            "verification conclusion requires check evidence".into(),
        ));
    }
    let mut identities = BTreeSet::new();
    if checks
        .iter()
        .any(|check| check.check_id.trim().is_empty() || !identities.insert(check.check_id.clone()))
    {
        return Err(WorkCoordinationError::InvalidInput(
            "verification check identities must be non-empty and unique".into(),
        ));
    }
    let valid = match conclusion {
        VerificationConclusion::Verified => checks
            .iter()
            .all(|check| check.outcome == VerificationCheckOutcome::Passed),
        VerificationConclusion::Rejected => checks
            .iter()
            .any(|check| check.outcome == VerificationCheckOutcome::Failed),
        VerificationConclusion::Indeterminate => checks
            .iter()
            .any(|check| check.outcome == VerificationCheckOutcome::Indeterminate),
    };
    if !valid {
        return Err(WorkCoordinationError::InvalidInput(
            "verification conclusion disagrees with its check evidence".into(),
        ));
    }
    Ok(())
}
