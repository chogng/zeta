use crate::WorkAttemptCoordinationStatus;
use crate::WorkAttemptExecutionStatus;
use crate::WorkAttemptVerificationStatus;
use crate::WorkCoordinationError;
use crate::WorkDecision;
use crate::WorkGoal;
use crate::WorkParticipantRelation;
use crate::WorkRelationStatus;
use crate::WorkRun;
use crate::WorkRunCommand;
use crate::WorkRunCommandRequest;
use crate::WorkRunStatus;
use crate::run::WORK_RUN_SCHEMA_VERSION;
use crate::validation;
use std::collections::BTreeMap;
use zeta_protocol::ContentDigest;

pub(crate) fn apply(
    current: Option<WorkRun>,
    request: &WorkRunCommandRequest,
) -> Result<WorkRun, WorkCoordinationError> {
    if matches!(request.command, WorkRunCommand::Create { .. }) {
        return create(current, request);
    }
    let mut run =
        current.ok_or_else(|| WorkCoordinationError::NotFound(request.work_run_id.to_string()))?;
    run.validate()?;
    if run.revision != request.expected_revision {
        return Err(WorkCoordinationError::RevisionConflict {
            expected: request.expected_revision,
            actual: run.revision,
        });
    }
    if run.status != WorkRunStatus::Active {
        return Err(WorkCoordinationError::WorkRunClosed);
    }
    match &request.command {
        WorkRunCommand::Create { .. } => unreachable!("creation handled above"),
        WorkRunCommand::ReviseGoal {
            objective,
            acceptance_conditions,
            exclusions,
        } => revise_goal(&mut run, objective, acceptance_conditions, exclusions)?,
        WorkRunCommand::AddParticipant { participant } => {
            validation::new_participant(&run, participant)?;
            run.participants
                .insert(participant.thread_id.clone(), participant.clone());
            run.topology_revision = next(run.topology_revision, "topology revision")?;
            let affected = run
                .attempts
                .values()
                .filter(|attempt| {
                    attempt.integration_status != crate::WorkAttemptIntegrationStatus::Integrated
                })
                .map(|attempt| attempt.attempt_id.clone())
                .collect::<Vec<_>>();
            for attempt_id in affected {
                crate::attempt_reducer::mark_stale(
                    &mut run,
                    &attempt_id,
                    "WorkRun collaboration topology changed",
                )?;
            }
        }
        WorkRunCommand::RecordDecision {
            decision_id,
            authority,
            scope,
            statement,
        } => record_decision(&mut run, decision_id, authority, scope, statement)?,
        WorkRunCommand::CreateContract { contract } => {
            crate::contract_reducer::create_contract(&mut run, contract)?
        }
        WorkRunCommand::ReviseContract {
            expected_contract_revision,
            contract,
        } => crate::contract_reducer::revise_contract(
            &mut run,
            *expected_contract_revision,
            contract,
        )?,
        WorkRunCommand::CreateAttempt {
            attempt_id,
            contract,
            participant_thread_id,
        } => crate::contract_reducer::create_attempt(
            &mut run,
            attempt_id,
            contract,
            participant_thread_id,
        )?,
        WorkRunCommand::RecordAttemptWorkspaceReady {
            attempt_id,
            roots,
            private_output_dir_id,
        } => crate::attempt_reducer::record_workspace_ready(
            &mut run,
            attempt_id,
            roots,
            private_output_dir_id,
        )?,
        WorkRunCommand::FailAttemptWorkspace { attempt_id, reason } => {
            crate::attempt_reducer::fail_workspace(&mut run, attempt_id, reason)?
        }
        WorkRunCommand::BeginAttempt {
            attempt_id,
            execution_id,
            mode,
        } => crate::attempt_reducer::begin(&mut run, attempt_id, execution_id, *mode)?,
        WorkRunCommand::RequestScopeExpansion {
            attempt_id,
            evidence,
        } => crate::attempt_reducer::request_scope_expansion(&mut run, attempt_id, evidence)?,
        WorkRunCommand::RecordConflict {
            conflict_id,
            attempt_ids,
            resource,
            evidence,
        } => crate::relation_reducer::record_conflict(
            &mut run,
            conflict_id,
            attempt_ids,
            resource,
            evidence,
        )?,
        WorkRunCommand::ResolveConflict {
            conflict_id,
            decision_id,
        } => crate::relation_reducer::resolve_conflict(&mut run, conflict_id, decision_id)?,
        WorkRunCommand::CreateRelation {
            relation_id,
            source_attempt_id,
            target_attempt_id,
            kind,
        } => crate::relation_reducer::create_relation(
            &mut run,
            relation_id,
            source_attempt_id,
            target_attempt_id,
            kind,
        )?,
        WorkRunCommand::ResolveWait {
            relation_id,
            target_attempt_id,
            target_execution_id,
            outcome,
        } => crate::relation_reducer::resolve_wait(
            &mut run,
            relation_id,
            target_attempt_id,
            target_execution_id,
            outcome,
        )?,
        WorkRunCommand::SealAttempt {
            attempt_id,
            result_digest,
            change_set_ids,
            private_output_digest,
            external_effects_digest,
            external_effects_status,
        } => crate::attempt_reducer::seal(
            &mut run,
            attempt_id,
            result_digest,
            change_set_ids,
            private_output_digest,
            external_effects_digest,
            *external_effects_status,
        )?,
        WorkRunCommand::BeginVerification { input } => {
            crate::verification_reducer::begin(&mut run, input)?
        }
        WorkRunCommand::FinishVerification {
            verification_key,
            conclusion,
            checks,
            reason,
        } => crate::verification_reducer::finish(
            &mut run,
            verification_key,
            *conclusion,
            checks,
            reason,
        )?,
        WorkRunCommand::MarkVerificationStale {
            verification_key,
            reason,
        } => crate::verification_reducer::mark_stale(&mut run, verification_key, reason)?,
        WorkRunCommand::QueueIntegration { verification_key } => {
            crate::integration_reducer::queue(&mut run, verification_key)?
        }
        WorkRunCommand::RecordIntegrationRootPrepared {
            integration_key,
            generation,
            root_id,
            artifact,
        } => crate::integration_reducer::record_prepared(
            &mut run,
            integration_key,
            *generation,
            root_id,
            artifact,
        )?,
        WorkRunCommand::BeginIntegration {
            integration_key,
            generation,
        } => crate::integration_reducer::begin(&mut run, integration_key, *generation)?,
        WorkRunCommand::RecordIntegrationRootPublished {
            integration_key,
            generation,
            root_id,
            receipt_digest,
        } => crate::integration_reducer::record_published(
            &mut run,
            integration_key,
            *generation,
            root_id,
            receipt_digest,
        )?,
        WorkRunCommand::FailIntegration {
            integration_key,
            generation,
            kind,
            reason,
        } => {
            crate::integration_reducer::fail(&mut run, integration_key, *generation, *kind, reason)?
        }
        WorkRunCommand::ResumeIntegration {
            integration_key,
            generation,
        } => crate::integration_reducer::resume(&mut run, integration_key, *generation)?,
        WorkRunCommand::FailAttempt {
            attempt_id,
            message,
        } => crate::attempt_reducer::finish(
            &mut run,
            attempt_id,
            WorkAttemptExecutionStatus::Failed,
            message,
        )?,
        WorkRunCommand::InterruptAttempt {
            attempt_id,
            message,
        } => crate::attempt_reducer::finish(
            &mut run,
            attempt_id,
            WorkAttemptExecutionStatus::Interrupted,
            message,
        )?,
        WorkRunCommand::CancelAttempt { attempt_id, reason } => crate::attempt_reducer::finish(
            &mut run,
            attempt_id,
            WorkAttemptExecutionStatus::Cancelled,
            reason,
        )?,
        WorkRunCommand::Complete => complete(&mut run)?,
        WorkRunCommand::Cancel { reason } => cancel(&mut run, reason)?,
    }
    run.revision = next(run.revision, "work-run revision")?;
    run.validate()?;
    Ok(run)
}

fn create(
    current: Option<WorkRun>,
    request: &WorkRunCommandRequest,
) -> Result<WorkRun, WorkCoordinationError> {
    if current.is_some() {
        return Err(WorkCoordinationError::AlreadyExists(
            request.work_run_id.to_string(),
        ));
    }
    if request.expected_revision != 0 {
        return Err(WorkCoordinationError::RevisionConflict {
            expected: request.expected_revision,
            actual: 0,
        });
    }
    let WorkRunCommand::Create {
        objective,
        acceptance_conditions,
        exclusions,
        root_participant,
    } = &request.command
    else {
        unreachable!("creation command checked by caller")
    };
    validation::goal(objective, acceptance_conditions, exclusions)?;
    if root_participant.relation != WorkParticipantRelation::Root {
        return Err(WorkCoordinationError::InvalidInput(
            "a WorkRun must begin with a root participant".into(),
        ));
    }
    let run = WorkRun {
        schema_version: WORK_RUN_SCHEMA_VERSION,
        work_run_id: request.work_run_id.clone(),
        revision: 1,
        topology_revision: 1,
        status: WorkRunStatus::Active,
        terminal_reason: None,
        goals: vec![WorkGoal {
            revision: 1,
            objective: objective.clone(),
            acceptance_conditions: acceptance_conditions.clone(),
            exclusions: exclusions.clone(),
        }],
        participants: BTreeMap::from([(
            root_participant.thread_id.clone(),
            root_participant.clone(),
        )]),
        decisions: BTreeMap::new(),
        contracts: BTreeMap::new(),
        attempts: BTreeMap::new(),
        relations: BTreeMap::new(),
        conflicts: BTreeMap::new(),
        verifications: BTreeMap::new(),
        integrations: BTreeMap::new(),
    };
    run.validate()?;
    Ok(run)
}

fn revise_goal(
    run: &mut WorkRun,
    objective: &str,
    acceptance_conditions: &[String],
    exclusions: &[String],
) -> Result<(), WorkCoordinationError> {
    validation::goal(objective, acceptance_conditions, exclusions)?;
    let revision = next(
        run.current_goal()
            .ok_or_else(|| {
                WorkCoordinationError::InvalidInput("WorkRun has no goal revision".into())
            })?
            .revision,
        "goal revision",
    )?;
    run.goals.push(WorkGoal {
        revision,
        objective: objective.into(),
        acceptance_conditions: acceptance_conditions.into(),
        exclusions: exclusions.into(),
    });
    let attempt_ids = run.attempts.keys().cloned().collect::<Vec<_>>();
    for attempt_id in attempt_ids {
        crate::attempt_reducer::mark_stale(run, &attempt_id, "WorkRun goal revision changed")?;
    }
    Ok(())
}

fn record_decision(
    run: &mut WorkRun,
    decision_id: &zeta_protocol::WorkDecisionId,
    authority: &str,
    scope: &str,
    statement: &str,
) -> Result<(), WorkCoordinationError> {
    if run.decisions.contains_key(decision_id) {
        return Err(WorkCoordinationError::AlreadyExists(
            decision_id.to_string(),
        ));
    }
    validation::text("decision authority", authority)?;
    validation::text("decision scope", scope)?;
    validation::text("decision statement", statement)?;
    let encoded = serde_json::to_vec(&(authority, scope, statement))
        .map_err(|error| WorkCoordinationError::InvalidInput(error.to_string()))?;
    run.decisions.insert(
        decision_id.clone(),
        WorkDecision {
            decision_id: decision_id.clone(),
            authority: authority.into(),
            scope: scope.into(),
            statement: statement.into(),
            content_digest: ContentDigest::sha256(&encoded),
        },
    );
    Ok(())
}

fn complete(run: &mut WorkRun) -> Result<(), WorkCoordinationError> {
    if run.attempts.is_empty()
        || run
            .attempts
            .values()
            .any(|attempt| attempt.execution_status != WorkAttemptExecutionStatus::Sealed)
        || run
            .conflicts
            .values()
            .any(|conflict| conflict.status == crate::WorkConflictStatus::Open)
        || run
            .relations
            .values()
            .any(|relation| relation.status == WorkRelationStatus::Waiting)
        || run.attempts.values().any(|attempt| {
            attempt.coordination_status != WorkAttemptCoordinationStatus::Clear
                || attempt.verification_status != WorkAttemptVerificationStatus::Verified
                || attempt.integration_status != crate::WorkAttemptIntegrationStatus::Integrated
        })
    {
        return Err(WorkCoordinationError::InvalidTransition(
            "a WorkRun completes only after every attempt is sealed, clear, independently verified and integrated, with every dependency settled"
                .into(),
        ));
    }
    run.status = WorkRunStatus::Completed;
    Ok(())
}

fn cancel(run: &mut WorkRun, reason: &str) -> Result<(), WorkCoordinationError> {
    validation::text("work-run cancellation reason", reason)?;
    for attempt in run
        .attempts
        .values_mut()
        .filter(|attempt| !attempt.is_terminal())
    {
        attempt.execution_status = WorkAttemptExecutionStatus::Cancelled;
        attempt.failure = Some(reason.into());
        attempt.waiting_relation_id = None;
    }
    for relation in run.relations.values_mut() {
        if relation.status == WorkRelationStatus::Waiting {
            relation.status = WorkRelationStatus::Cancelled;
        }
    }
    run.status = WorkRunStatus::Cancelled;
    run.terminal_reason = Some(reason.into());
    Ok(())
}

pub(crate) fn next(value: u64, label: &str) -> Result<u64, WorkCoordinationError> {
    value
        .checked_add(1)
        .ok_or_else(|| WorkCoordinationError::InvalidTransition(format!("{label} overflow")))
}
