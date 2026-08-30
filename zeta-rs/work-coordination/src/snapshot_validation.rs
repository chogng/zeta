use crate::IntegrationRootStatus;
use crate::VerificationCheckOutcome;
use crate::VerificationConclusion;
use crate::WorkAttempt;
use crate::WorkAttemptCoordinationStatus;
use crate::WorkAttemptExecutionStatus;
use crate::WorkAttemptIntegrationStatus;
use crate::WorkAttemptVerificationStatus;
use crate::WorkAttemptWorkspace;
use crate::WorkConflictStatus;
use crate::WorkContractVersion;
use crate::WorkCoordinationError;
use crate::WorkIntegrationStatus;
use crate::WorkParticipantRelation;
use crate::WorkRelationKind;
use crate::WorkRelationStatus;
use crate::WorkRun;
use crate::WorkRunStatus;
use crate::WorkVerificationStatus;
use crate::run::WORK_RUN_SCHEMA_VERSION;
use crate::validation;
use std::collections::BTreeSet;
use zeta_protocol::ContentDigest;
use zeta_protocol::ThreadId;

pub(crate) fn work_run(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    if run.schema_version != WORK_RUN_SCHEMA_VERSION {
        return invalid("unsupported WorkRun record schema version");
    }
    if run.revision == 0 || run.topology_revision == 0 || run.topology_revision > run.revision {
        return invalid("WorkRun revisions are inconsistent");
    }
    goals(run)?;
    participants(run)?;
    decisions(run)?;
    contracts(run)?;
    attempts(run)?;
    relations(run)?;
    conflicts(run)?;
    verifications(run)?;
    integrations(run)?;
    terminal_state(run)?;
    Ok(())
}

fn integrations(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    let mut active_attempts = BTreeSet::new();
    let mut final_attempts = BTreeSet::new();
    for (key, integration) in &run.integrations {
        if key != &integration.integration_key
            || crate::integration_key(&run.work_run_id, &integration.verification_key)? != *key
            || integration.generation == 0
        {
            return invalid("integration identity or generation is inconsistent");
        }
        let verification = run
            .verifications
            .get(&integration.verification_key)
            .ok_or_else(|| {
                WorkCoordinationError::InvalidInput(
                    "integration references an unknown verification".into(),
                )
            })?;
        let expected = crate::integration::integration_roots(verification)?;
        if integration.roots.len() != expected.len()
            || integration
                .roots
                .iter()
                .zip(expected.iter())
                .any(|(actual, expected)| {
                    actual.root_id != expected.root_id
                        || actual.source_dir_id != expected.source_dir_id
                        || actual.target != expected.target
                })
        {
            return invalid("integration roots do not match the exact verified final states");
        }
        let mut root_ids = BTreeSet::new();
        for root in &integration.roots {
            if !root_ids.insert(root.root_id.clone()) {
                return invalid("integration repeats a root identity");
            }
            match root.status {
                IntegrationRootStatus::Pending => {
                    if root.prepared_artifact.is_some() || root.publication_receipt_digest.is_some()
                    {
                        return invalid("pending integration root contains publication evidence");
                    }
                }
                IntegrationRootStatus::Prepared => {
                    let artifact = root.prepared_artifact.as_ref().ok_or_else(|| {
                        WorkCoordinationError::InvalidInput(
                            "prepared integration root omitted its artifact".into(),
                        )
                    })?;
                    crate::integration::validate_prepared_artifact(root, artifact)?;
                    if root.publication_receipt_digest.is_some() {
                        return invalid("unpublished integration root contains a receipt");
                    }
                }
                IntegrationRootStatus::Published => {
                    let artifact = root.prepared_artifact.as_ref().ok_or_else(|| {
                        WorkCoordinationError::InvalidInput(
                            "published integration root omitted its artifact".into(),
                        )
                    })?;
                    crate::integration::validate_prepared_artifact(root, artifact)?;
                    if root.publication_receipt_digest.is_none() {
                        return invalid("published integration root omitted its receipt");
                    }
                }
            }
        }
        let published = integration
            .roots
            .iter()
            .filter(|root| root.status == IntegrationRootStatus::Published)
            .count();
        let pending = integration
            .roots
            .iter()
            .filter(|root| root.status == IntegrationRootStatus::Pending)
            .count();
        let terminal = match integration.status {
            WorkIntegrationStatus::Queued => {
                if published != 0 {
                    return invalid("queued integration already published a root");
                }
                false
            }
            WorkIntegrationStatus::Integrating => {
                if pending != 0 || published == integration.roots.len() {
                    return invalid("active publication root states are inconsistent");
                }
                false
            }
            WorkIntegrationStatus::Integrated => {
                if published != integration.roots.len() {
                    return invalid("integrated transaction omitted a published root");
                }
                true
            }
            WorkIntegrationStatus::Partial => {
                if published == 0 || published == integration.roots.len() {
                    return invalid("partial transaction does not contain a strict root subset");
                }
                true
            }
            WorkIntegrationStatus::Conflict | WorkIntegrationStatus::Failed => {
                if published != 0 {
                    return invalid("non-partial stopped integration published a root");
                }
                true
            }
        };
        if terminal {
            if integration.evidence_digest.as_ref()
                != Some(&crate::integration::integration_evidence_digest(
                    integration,
                )?)
            {
                return invalid("integration evidence digest is inconsistent");
            }
            if integration.status != WorkIntegrationStatus::Integrated
                && integration.incidents.is_empty()
            {
                return invalid("stopped integration omitted its incident evidence");
            }
        } else if integration.evidence_digest.is_some() {
            return invalid("active integration contains terminal evidence");
        }
        for incident in &integration.incidents {
            validation::text("integration incident reason", &incident.reason)?;
            if incident.generation == 0
                || incident.generation > integration.generation
                || incident.published_root_count
                    > u64::try_from(integration.roots.len()).map_err(|_| {
                        WorkCoordinationError::InvalidInput("too many integration roots".into())
                    })?
            {
                return invalid("integration incident generation or root count is invalid");
            }
        }
        let attempt_status = match integration.status {
            WorkIntegrationStatus::Queued => Some(WorkAttemptIntegrationStatus::Queued),
            WorkIntegrationStatus::Integrating => Some(WorkAttemptIntegrationStatus::Integrating),
            WorkIntegrationStatus::Integrated => Some(WorkAttemptIntegrationStatus::Integrated),
            WorkIntegrationStatus::Partial => Some(WorkAttemptIntegrationStatus::Partial),
            WorkIntegrationStatus::Conflict | WorkIntegrationStatus::Failed => None,
        };
        for result in &verification.input.ordered_results {
            let attempt = run.attempts.get(&result.attempt_id).ok_or_else(|| {
                WorkCoordinationError::InvalidInput(
                    "integration references an unknown WorkAttempt".into(),
                )
            })?;
            if matches!(
                integration.status,
                WorkIntegrationStatus::Queued | WorkIntegrationStatus::Integrating
            ) && !active_attempts.insert(result.attempt_id.clone())
            {
                return invalid("one WorkAttempt belongs to multiple active integrations");
            }
            if matches!(
                integration.status,
                WorkIntegrationStatus::Integrated | WorkIntegrationStatus::Partial
            ) && !final_attempts.insert(result.attempt_id.clone())
            {
                return invalid("one WorkAttempt has multiple final publication transactions");
            }
            if let Some(expected) = attempt_status
                && attempt.integration_status != expected
            {
                return invalid("WorkAttempt integration status disagrees with its transaction");
            }
        }
    }
    Ok(())
}

fn verifications(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    let mut current_attempts = BTreeSet::new();
    for (key, verification) in &run.verifications {
        if key != &verification.verification_key
            || crate::verification_key(&run.work_run_id, &verification.input)? != *key
        {
            return invalid("verification map key or canonical input identity is inconsistent");
        }
        validation::text(
            "serializability evidence reason",
            &verification.input.serializability.reason,
        )?;
        if verification.status == WorkVerificationStatus::Verified
            && verification.input.serializability.status != crate::WorkSerializabilityStatus::Proven
        {
            return invalid("verified input has no serializability proof");
        }
        let mut result_ids = BTreeSet::new();
        let mut change_ids = BTreeSet::new();
        let mut root_ids = BTreeSet::new();
        if verification.input.ordered_results.is_empty()
            || verification.input.roots.is_empty()
            || !verification
                .input
                .ordered_results
                .iter()
                .all(|result| result_ids.insert(result.attempt_id.clone()))
            || !verification
                .input
                .ordered_change_sets
                .iter()
                .all(|change| change_ids.insert(change.change_set.change_set_id.clone()))
            || !verification
                .input
                .roots
                .iter()
                .all(|root| root_ids.insert(root.source_dir_id.clone()))
        {
            return invalid("verification input repeats or omits required identities");
        }
        for result in &verification.input.ordered_results {
            let attempt = run.attempts.get(&result.attempt_id).ok_or_else(|| {
                WorkCoordinationError::InvalidInput(
                    "verification references an unknown WorkAttempt".into(),
                )
            })?;
            if attempt
                .result
                .as_ref()
                .is_none_or(|sealed| sealed.result_digest != result.result_digest)
            {
                return invalid("verification references a mismatched sealed result");
            }
            if verification.status != WorkVerificationStatus::Stale
                && !current_attempts.insert(result.attempt_id.clone())
            {
                return invalid("one WorkAttempt belongs to multiple current verifications");
            }
        }
        if verification.status != WorkVerificationStatus::Stale
            && (verification.input.goal_revision
                != run.current_goal().map(|goal| goal.revision).unwrap_or(0)
                || verification.input.topology_revision != run.topology_revision)
        {
            return invalid("current verification uses stale goal or topology inputs");
        }
        if verification.status != WorkVerificationStatus::Stale
            && verification.input.coordination_digest
                != crate::verification_coordination_digest(
                    run,
                    &verification.input.ordered_results,
                )?
        {
            return invalid("current verification uses stale coordination evidence");
        }
        let conclusion = match verification.status {
            WorkVerificationStatus::Verifying => {
                if !verification.checks.is_empty()
                    || verification.evidence_digest.is_some()
                    || verification.reason.is_some()
                    || verification.stale_reason.is_some()
                {
                    return invalid("active verification contains terminal evidence");
                }
                None
            }
            WorkVerificationStatus::Verified => Some(VerificationConclusion::Verified),
            WorkVerificationStatus::Rejected => Some(VerificationConclusion::Rejected),
            WorkVerificationStatus::Indeterminate => Some(VerificationConclusion::Indeterminate),
            WorkVerificationStatus::Stale => {
                if verification.stale_reason.is_none() {
                    return invalid("stale verification omitted its stale reason");
                }
                if verification.evidence_digest.is_none() {
                    if !verification.checks.is_empty() || verification.reason.is_some() {
                        return invalid("stale active verification has partial terminal evidence");
                    }
                    None
                } else if verification
                    .checks
                    .iter()
                    .any(|check| check.outcome == VerificationCheckOutcome::Failed)
                {
                    Some(VerificationConclusion::Rejected)
                } else if verification
                    .checks
                    .iter()
                    .any(|check| check.outcome == VerificationCheckOutcome::Indeterminate)
                {
                    Some(VerificationConclusion::Indeterminate)
                } else {
                    Some(VerificationConclusion::Verified)
                }
            }
        };
        if let Some(conclusion) = conclusion {
            crate::verification_reducer::validate_checks(conclusion, &verification.checks)?;
            let reason = verification.reason.as_deref().ok_or_else(|| {
                WorkCoordinationError::InvalidInput(
                    "verification omitted its conclusion reason".into(),
                )
            })?;
            let expected = crate::verification::verification_evidence_digest(
                key,
                conclusion,
                &verification.checks,
                reason,
            )?;
            if verification.evidence_digest.as_ref() != Some(&expected) {
                return invalid("verification evidence digest is inconsistent");
            }
        }
    }
    for attempt in run.attempts.values() {
        let current = run.verifications.values().find(|verification| {
            verification.status != WorkVerificationStatus::Stale
                && verification
                    .input
                    .ordered_results
                    .iter()
                    .any(|result| result.attempt_id == attempt.attempt_id)
        });
        let expected = current.map(|verification| match verification.status {
            WorkVerificationStatus::Verifying => WorkAttemptVerificationStatus::Verifying,
            WorkVerificationStatus::Verified => WorkAttemptVerificationStatus::Verified,
            WorkVerificationStatus::Rejected => WorkAttemptVerificationStatus::Rejected,
            WorkVerificationStatus::Indeterminate => WorkAttemptVerificationStatus::Indeterminate,
            WorkVerificationStatus::Stale => unreachable!("stale records were filtered"),
        });
        if let Some(expected) = expected
            && attempt.verification_status != expected
        {
            return invalid("WorkAttempt verification status disagrees with its current record");
        }
    }
    Ok(())
}

fn goals(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    if run.goals.is_empty() {
        return invalid("WorkRun has no goal revision");
    }
    for (index, goal) in run.goals.iter().enumerate() {
        let expected = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| WorkCoordinationError::InvalidInput("too many goal revisions".into()))?;
        if goal.revision != expected {
            return invalid("WorkRun goal revisions are not contiguous");
        }
        validation::goal(
            &goal.objective,
            &goal.acceptance_conditions,
            &goal.exclusions,
        )?;
    }
    Ok(())
}

fn participants(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    if run.participants.is_empty() {
        return invalid("WorkRun has no participant");
    }
    let participant_count = u64::try_from(run.participants.len())
        .map_err(|_| WorkCoordinationError::InvalidInput("too many participants".into()))?;
    if run.topology_revision != participant_count {
        return invalid("WorkRun topology revision does not match its participant set");
    }
    let mut roots = BTreeSet::new();
    let mut delegations = BTreeSet::new();
    for (thread_id, participant) in &run.participants {
        if thread_id != &participant.thread_id {
            return invalid("participant map key disagrees with its Thread identity");
        }
        match &participant.relation {
            WorkParticipantRelation::Root => {
                if !roots.insert(participant.session_id.clone()) {
                    return invalid("one Session has more than one WorkRun root participant");
                }
            }
            WorkParticipantRelation::Delegated {
                parent_thread_id,
                delegation_id,
            } => {
                if parent_thread_id == thread_id {
                    return invalid("a delegated participant is its own parent");
                }
                let parent = run.participants.get(parent_thread_id).ok_or_else(|| {
                    WorkCoordinationError::InvalidInput(
                        "a delegated participant has no parent participant".into(),
                    )
                })?;
                if parent.session_id != participant.session_id {
                    return invalid("a delegated participant crosses Session boundaries");
                }
                if !delegations.insert(delegation_id) {
                    return invalid("one delegation identity is bound to multiple participants");
                }
            }
        }
        participant_chain(run, thread_id)?;
    }
    for participant in run.participants.values() {
        if !roots.contains(&participant.session_id) {
            return invalid("a participant Session has no root participant");
        }
    }
    Ok(())
}

fn participant_chain(run: &WorkRun, start: &ThreadId) -> Result<(), WorkCoordinationError> {
    let mut visited = BTreeSet::new();
    let mut current = start;
    loop {
        if !visited.insert(current) {
            return invalid("participant delegation graph contains a cycle");
        }
        let participant = &run.participants[current];
        match &participant.relation {
            WorkParticipantRelation::Root => return Ok(()),
            WorkParticipantRelation::Delegated {
                parent_thread_id, ..
            } => current = parent_thread_id,
        }
    }
}

fn decisions(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    for (decision_id, decision) in &run.decisions {
        if decision_id != &decision.decision_id {
            return invalid("decision map key disagrees with its identity");
        }
        validation::text("decision authority", &decision.authority)?;
        validation::text("decision scope", &decision.scope)?;
        validation::text("decision statement", &decision.statement)?;
        let encoded = serde_json::to_vec(&(
            decision.authority.as_str(),
            decision.scope.as_str(),
            decision.statement.as_str(),
        ))
        .map_err(|error| WorkCoordinationError::InvalidInput(error.to_string()))?;
        if decision.content_digest != ContentDigest::sha256(&encoded) {
            return invalid("decision content digest does not match its content");
        }
    }
    Ok(())
}

fn contracts(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    for (contract_id, versions) in &run.contracts {
        if versions.is_empty() {
            return invalid("a contract identity has no versions");
        }
        for (index, contract) in versions.iter().enumerate() {
            let expected = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| {
                    WorkCoordinationError::InvalidInput("too many contract revisions".into())
                })?;
            if &contract.contract_id != contract_id || contract.revision != expected {
                return invalid("contract identity or revision sequence is inconsistent");
            }
            contract_version(run, contract)?;
        }
    }
    Ok(())
}

fn contract_version(
    run: &WorkRun,
    contract: &WorkContractVersion,
) -> Result<(), WorkCoordinationError> {
    if !run
        .goals
        .iter()
        .any(|goal| goal.revision == contract.goal_revision)
    {
        return invalid("contract references an unknown goal revision");
    }
    if contract.topology_revision == 0 || contract.topology_revision > run.topology_revision {
        return invalid("contract topology revision is invalid");
    }
    if !run.participants.contains_key(&contract.owner_thread_id) {
        return invalid("contract owner is not a WorkRun participant");
    }
    validation::goal(
        &contract.objective,
        &contract.acceptance_conditions,
        &contract.exclusions,
    )?;
    validation::root_checkpoints(&contract.roots, &contract.environment_id)?;
    if !contract
        .roots
        .iter()
        .any(|root| root.dir_id == contract.primary_root_dir_id)
    {
        return invalid("contract primary root is not one of its checkpoints");
    }
    validation::text("authorization authority", &contract.authorization.authority)?;
    validation::text(
        "authorization policy revision",
        &contract.authorization.policy_revision,
    )?;
    validation::text("validation profile name", &contract.validation_profile.name)?;
    validation::scope_claim(&contract.expected_scope)?;
    for decision_id in &contract.decision_ids {
        if !run.decisions.contains_key(decision_id) {
            return invalid("contract references an unknown decision");
        }
    }
    for result in &contract.upstream_results {
        let attempt = run.attempts.get(&result.attempt_id).ok_or_else(|| {
            WorkCoordinationError::InvalidInput(
                "contract references an unknown upstream attempt".into(),
            )
        })?;
        if attempt
            .result
            .as_ref()
            .is_none_or(|sealed| sealed.result_digest != result.result_digest)
        {
            return invalid("contract references an unsealed or mismatched upstream result");
        }
    }
    Ok(())
}

fn attempts(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    let mut execution_ids = BTreeSet::new();
    let mut active_threads = BTreeSet::new();
    let mut source_dirs = BTreeSet::new();
    let mut managed_dirs = BTreeSet::new();
    let mut output_dirs = BTreeSet::new();
    for (attempt_id, attempt) in &run.attempts {
        if attempt_id != &attempt.attempt_id {
            return invalid("attempt map key disagrees with its identity");
        }
        let participant = run.participants.get(&attempt.thread_id).ok_or_else(|| {
            WorkCoordinationError::InvalidInput("attempt owner is not a WorkRun participant".into())
        })?;
        if participant.session_id != attempt.session_id {
            return invalid("attempt Session disagrees with its participant");
        }
        let contract = run
            .contract(&attempt.contract.contract_id, attempt.contract.revision)
            .ok_or_else(|| {
                WorkCoordinationError::InvalidInput("attempt references an unknown contract".into())
            })?;
        if contract.owner_thread_id != attempt.thread_id
            || contract.environment_id != attempt.environment_id
            || contract.roots != attempt.roots
            || contract.primary_root_dir_id != attempt.primary_root_dir_id
        {
            return invalid("attempt does not preserve its immutable contract binding");
        }
        source_dirs.extend(attempt.roots.iter().map(|root| &root.dir_id));
        if let Some(execution_id) = &attempt.execution_id
            && !execution_ids.insert(execution_id)
        {
            return invalid("one execution identity is bound to multiple attempts");
        }
        if matches!(
            attempt.execution_status,
            WorkAttemptExecutionStatus::Exploring | WorkAttemptExecutionStatus::Writing
        ) && !active_threads.insert(&attempt.thread_id)
        {
            return invalid("one Thread has multiple active WorkAttempts");
        }
        match &attempt.workspace {
            WorkAttemptWorkspace::Provisioning => {
                if !matches!(
                    attempt.execution_status,
                    WorkAttemptExecutionStatus::Planned
                        | WorkAttemptExecutionStatus::Failed
                        | WorkAttemptExecutionStatus::Interrupted
                        | WorkAttemptExecutionStatus::Cancelled
                ) {
                    return invalid("an executing attempt has no ready workspace");
                }
            }
            WorkAttemptWorkspace::Ready {
                roots,
                private_output_dir_id,
            } => {
                validation::workspace_bindings(&attempt.roots, roots, private_output_dir_id)?;
                for root in roots {
                    if !managed_dirs.insert(&root.managed_dir_id) {
                        return invalid("one managed root is shared by multiple WorkAttempts");
                    }
                }
                if !output_dirs.insert(private_output_dir_id) {
                    return invalid("one private output is shared by multiple WorkAttempts");
                }
            }
            WorkAttemptWorkspace::Failed { reason } => {
                validation::text("workspace provisioning failure", reason)?;
                if attempt.execution_status != WorkAttemptExecutionStatus::Failed
                    || attempt.failure.as_deref() != Some(reason)
                {
                    return invalid("failed workspace disagrees with attempt execution state");
                }
            }
        }
        attempt_state(attempt)?;
    }
    if managed_dirs
        .iter()
        .any(|dir| output_dirs.contains(dir) || source_dirs.contains(dir))
        || output_dirs.iter().any(|dir| source_dirs.contains(dir))
    {
        return invalid("managed roots and private outputs must be isolated from source roots");
    }
    Ok(())
}

fn attempt_state(attempt: &WorkAttempt) -> Result<(), WorkCoordinationError> {
    match attempt.execution_status {
        WorkAttemptExecutionStatus::Planned => {
            if attempt.execution_id.is_some()
                || attempt.result.is_some()
                || attempt.failure.is_some()
                || attempt.waiting_relation_id.is_some()
            {
                return invalid("planned attempt contains execution or terminal state");
            }
        }
        WorkAttemptExecutionStatus::Exploring | WorkAttemptExecutionStatus::Writing => {
            if attempt.execution_id.is_none()
                || attempt.result.is_some()
                || attempt.failure.is_some()
                || attempt.waiting_relation_id.is_some()
            {
                return invalid("active attempt state is inconsistent");
            }
            if !attempt.workspace.is_ready() {
                return invalid("active attempt has no ready workspace");
            }
        }
        WorkAttemptExecutionStatus::Waiting => {
            if attempt.execution_id.is_none()
                || attempt.result.is_some()
                || attempt.failure.is_some()
                || attempt.waiting_relation_id.is_none()
            {
                return invalid("waiting attempt state is inconsistent");
            }
        }
        WorkAttemptExecutionStatus::Sealed => {
            if attempt.execution_id.is_none()
                || attempt.result.is_none()
                || attempt.failure.is_some()
                || attempt.waiting_relation_id.is_some()
            {
                return invalid("sealed attempt state is inconsistent");
            }
            if !attempt.workspace.is_ready() {
                return invalid("sealed attempt has no immutable workspace binding");
            }
        }
        WorkAttemptExecutionStatus::Failed
        | WorkAttemptExecutionStatus::Interrupted
        | WorkAttemptExecutionStatus::Cancelled => {
            if attempt.result.is_some()
                || attempt.failure.is_none()
                || attempt.waiting_relation_id.is_some()
            {
                return invalid("terminal attempt state is inconsistent");
            }
        }
    }
    if attempt.coordination_status == WorkAttemptCoordinationStatus::ExpansionRequested
        && attempt.scope_expansion_evidence.is_empty()
    {
        return invalid("scope expansion has no evidence");
    }
    if let Some(result) = &attempt.result {
        let mut changes = BTreeSet::new();
        if !result
            .change_set_ids
            .iter()
            .all(|identity| changes.insert(identity))
        {
            return invalid("attempt result repeats a ChangeSet identity");
        }
    }
    if attempt.integration_status == WorkAttemptIntegrationStatus::Integrated
        && attempt.verification_status != WorkAttemptVerificationStatus::Verified
    {
        return invalid("an unverified attempt is marked integrated");
    }
    Ok(())
}

fn relations(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    crate::dependency_graph::validate_acyclic(run)?;
    for (relation_id, relation) in &run.relations {
        if relation_id != &relation.relation_id {
            return invalid("relation map key disagrees with its identity");
        }
        if relation.source_attempt_id == relation.target_attempt_id
            || !run.attempts.contains_key(&relation.source_attempt_id)
            || !run.attempts.contains_key(&relation.target_attempt_id)
        {
            return invalid("relation has invalid attempt identities");
        }
        match &relation.kind {
            WorkRelationKind::Wait {
                target_execution_id,
                ..
            } => {
                let target = &run.attempts[&relation.target_attempt_id];
                if target.execution_id.as_ref() != Some(target_execution_id)
                    || !matches!(
                        relation.resume_execution_status,
                        Some(WorkAttemptExecutionStatus::Exploring)
                            | Some(WorkAttemptExecutionStatus::Writing)
                    )
                {
                    return invalid("wait relation does not preserve its execution generation");
                }
                let source = &run.attempts[&relation.source_attempt_id];
                if relation.status == WorkRelationStatus::Waiting
                    && (source.execution_status != WorkAttemptExecutionStatus::Waiting
                        || source.waiting_relation_id.as_ref() != Some(relation_id))
                {
                    return invalid("waiting relation disagrees with its source attempt");
                }
            }
            WorkRelationKind::ResultDependency { result_digest } => {
                if relation.resume_execution_status.is_some()
                    || relation.status
                        != (WorkRelationStatus::Satisfied {
                            evidence_digest: result_digest.clone(),
                        })
                {
                    return invalid("result dependency state is inconsistent");
                }
            }
            WorkRelationKind::Observation
            | WorkRelationKind::Alternate
            | WorkRelationKind::Handoff { .. } => {
                if relation.resume_execution_status.is_some()
                    || relation.status != WorkRelationStatus::Active
                {
                    return invalid("non-wait relation state is inconsistent");
                }
            }
        }
    }
    for attempt in run.attempts.values() {
        if let Some(relation_id) = &attempt.waiting_relation_id {
            let relation = run.relations.get(relation_id).ok_or_else(|| {
                WorkCoordinationError::InvalidInput(
                    "waiting attempt references an unknown relation".into(),
                )
            })?;
            if relation.source_attempt_id != attempt.attempt_id
                || relation.status != WorkRelationStatus::Waiting
            {
                return invalid("waiting attempt references a non-waiting relation");
            }
        }
    }
    Ok(())
}

fn conflicts(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    for (conflict_id, conflict) in &run.conflicts {
        if conflict_id != &conflict.conflict_id || conflict.attempt_ids.is_empty() {
            return invalid("conflict identity or participants are invalid");
        }
        validation::text("conflict resource", &conflict.resource)?;
        validation::non_empty_texts("conflict evidence", &conflict.evidence)?;
        let mut attempts = BTreeSet::new();
        for attempt_id in &conflict.attempt_ids {
            if !attempts.insert(attempt_id) || !run.attempts.contains_key(attempt_id) {
                return invalid("conflict contains duplicate or unknown attempts");
            }
        }
        match conflict.status {
            WorkConflictStatus::Open if conflict.resolution_decision_id.is_some() => {
                return invalid("open conflict already has a resolution decision");
            }
            WorkConflictStatus::Resolved => {
                if conflict
                    .resolution_decision_id
                    .as_ref()
                    .is_none_or(|decision_id| !run.decisions.contains_key(decision_id))
                {
                    return invalid("resolved conflict has no accepted decision");
                }
            }
            WorkConflictStatus::Open => {}
        }
    }
    Ok(())
}

fn terminal_state(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    match run.status {
        WorkRunStatus::Active | WorkRunStatus::Completed if run.terminal_reason.is_some() => {
            invalid("active or completed WorkRun contains a cancellation reason")
        }
        WorkRunStatus::Cancelled => {
            let reason = run.terminal_reason.as_deref().ok_or_else(|| {
                WorkCoordinationError::InvalidInput(
                    "cancelled WorkRun has no terminal reason".into(),
                )
            })?;
            validation::text("work-run cancellation reason", reason)?;
            if run.attempts.values().any(|attempt| !attempt.is_terminal()) {
                return invalid("cancelled WorkRun still has a non-terminal attempt");
            }
            Ok(())
        }
        WorkRunStatus::Completed => {
            if run.attempts.is_empty()
                || run.attempts.values().any(|attempt| {
                    attempt.execution_status != WorkAttemptExecutionStatus::Sealed
                        || attempt.coordination_status != WorkAttemptCoordinationStatus::Clear
                        || attempt.verification_status != WorkAttemptVerificationStatus::Verified
                        || attempt.integration_status != WorkAttemptIntegrationStatus::Integrated
                })
            {
                return invalid("completed WorkRun contains unfinished work");
            }
            Ok(())
        }
        WorkRunStatus::Active => Ok(()),
    }
}

fn invalid<T>(message: &str) -> Result<T, WorkCoordinationError> {
    Err(WorkCoordinationError::InvalidInput(message.into()))
}
