use crate::WorkAttempt;
use crate::WorkAttemptCoordinationStatus;
use crate::WorkAttemptExecutionStatus;
use crate::WorkAttemptIntegrationStatus;
use crate::WorkAttemptVerificationStatus;
use crate::WorkAttemptWorkspace;
use crate::WorkContractDraft;
use crate::WorkContractRef;
use crate::WorkContractVersion;
use crate::WorkCoordinationError;
use crate::WorkRun;
use crate::validation;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;

pub(crate) fn create_contract(
    run: &mut WorkRun,
    draft: &WorkContractDraft,
) -> Result<(), WorkCoordinationError> {
    if run.contracts.contains_key(&draft.contract_id) {
        return Err(WorkCoordinationError::AlreadyExists(
            draft.contract_id.to_string(),
        ));
    }
    validation::contract(run, draft)?;
    run.contracts
        .insert(draft.contract_id.clone(), vec![contract_version(draft, 1)]);
    Ok(())
}

pub(crate) fn revise_contract(
    run: &mut WorkRun,
    expected_revision: u64,
    draft: &WorkContractDraft,
) -> Result<(), WorkCoordinationError> {
    let latest = run
        .latest_contract(&draft.contract_id)
        .ok_or_else(|| WorkCoordinationError::NotFound(draft.contract_id.to_string()))?;
    if latest.revision != expected_revision {
        return Err(WorkCoordinationError::InvalidTransition(format!(
            "contract revision mismatch: expected {expected_revision}, actual {}",
            latest.revision
        )));
    }
    validation::contract(run, draft)?;
    let revision = crate::reducer::next(expected_revision, "contract revision")?;
    run.contracts
        .get_mut(&draft.contract_id)
        .expect("contract existence checked above")
        .push(contract_version(draft, revision));
    let affected = run
        .attempts
        .values()
        .filter(|attempt| attempt.contract.contract_id == draft.contract_id)
        .map(|attempt| attempt.attempt_id.clone())
        .collect::<Vec<_>>();
    for attempt_id in affected {
        crate::attempt_reducer::mark_stale(run, &attempt_id, "work contract revision changed")?;
    }
    Ok(())
}

pub(crate) fn create_attempt(
    run: &mut WorkRun,
    attempt_id: &WorkAttemptId,
    contract_ref: &WorkContractRef,
    participant_thread_id: &ThreadId,
) -> Result<(), WorkCoordinationError> {
    if run.attempts.contains_key(attempt_id) {
        return Err(WorkCoordinationError::AlreadyExists(attempt_id.to_string()));
    }
    let contract = run
        .contract(&contract_ref.contract_id, contract_ref.revision)
        .cloned()
        .ok_or_else(|| WorkCoordinationError::NotFound(contract_ref.contract_id.to_string()))?;
    if run
        .latest_contract(&contract_ref.contract_id)
        .is_none_or(|latest| latest.revision != contract_ref.revision)
        || contract.goal_revision
            != run
                .current_goal()
                .ok_or_else(|| {
                    WorkCoordinationError::InvalidInput("WorkRun has no goal revision".into())
                })?
                .revision
        || contract.topology_revision != run.topology_revision
    {
        return Err(WorkCoordinationError::InvalidInput(
            "a new attempt must bind the latest contract, goal and topology revisions".into(),
        ));
    }
    if &contract.owner_thread_id != participant_thread_id {
        return Err(WorkCoordinationError::InvalidInput(
            "attempt participant does not own the selected contract".into(),
        ));
    }
    let participant = validation::participant_for_attempt(run, participant_thread_id)?.clone();
    run.attempts.insert(
        attempt_id.clone(),
        WorkAttempt {
            attempt_id: attempt_id.clone(),
            contract: contract_ref.clone(),
            session_id: participant.session_id,
            thread_id: participant.thread_id,
            environment_id: contract.environment_id,
            roots: contract.roots,
            primary_root_dir_id: contract.primary_root_dir_id,
            workspace: WorkAttemptWorkspace::Provisioning,
            execution_id: None,
            execution_status: WorkAttemptExecutionStatus::Planned,
            coordination_status: WorkAttemptCoordinationStatus::Clear,
            verification_status: WorkAttemptVerificationStatus::Pending,
            integration_status: WorkAttemptIntegrationStatus::Idle,
            waiting_relation_id: None,
            scope_expansion_evidence: Vec::new(),
            result: None,
            failure: None,
        },
    );
    Ok(())
}

fn contract_version(draft: &WorkContractDraft, revision: u64) -> WorkContractVersion {
    WorkContractVersion {
        contract_id: draft.contract_id.clone(),
        revision,
        goal_revision: draft.goal_revision,
        topology_revision: draft.topology_revision,
        owner_thread_id: draft.owner_thread_id.clone(),
        objective: draft.objective.clone(),
        acceptance_conditions: draft.acceptance_conditions.clone(),
        exclusions: draft.exclusions.clone(),
        environment_id: draft.environment_id.clone(),
        roots: draft.roots.clone(),
        primary_root_dir_id: draft.primary_root_dir_id.clone(),
        authorization: draft.authorization.clone(),
        decision_ids: draft.decision_ids.clone(),
        upstream_results: draft.upstream_results.clone(),
        expected_scope: draft.expected_scope.clone(),
        validation_profile: draft.validation_profile.clone(),
    }
}
