use crate::GitRootTarget;
use crate::WorkAttemptChangeEvidenceRef;
use crate::WorkCoordinationError;
use crate::WorkResultRef;
use crate::WorkRun;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use zeta_file_access::DirId;
use zeta_protocol::ContentDigest;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkExecutionId;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationChangeSetInput {
    pub attempt_id: zeta_protocol::WorkAttemptId,
    pub change_set: WorkAttemptChangeEvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitVerificationRepository {
    pub repository_id: String,
    pub relative_path: String,
    pub target: GitRootTarget,
    pub target_tree: String,
    pub final_tree: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum VerificationRootState {
    Git {
        repositories: Vec<GitVerificationRepository>,
    },
    Directory {
        target_snapshot_id: String,
        final_snapshot_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRoot {
    pub source_dir_id: DirId,
    pub checkpoint_digest: ContentDigest,
    pub state: VerificationRootState,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkSerializabilityStatus {
    Proven,
    Indeterminate,
}

/// Host-derived proof summary for the declared dependency graph plus actual read/write effects.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkSerializabilityEvidence {
    pub status: WorkSerializabilityStatus,
    pub evidence_digest: ContentDigest,
    pub reason: String,
}

/// Immutable inputs selected before independent verification begins.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkVerificationInput {
    pub goal_revision: u64,
    pub topology_revision: u64,
    pub coordination_digest: ContentDigest,
    pub ordered_results: Vec<WorkResultRef>,
    pub ordered_change_sets: Vec<VerificationChangeSetInput>,
    pub serializability: WorkSerializabilityEvidence,
    pub roots: Vec<VerificationRoot>,
    pub authorization_digests: BTreeSet<ContentDigest>,
    pub control_resource_digests: BTreeSet<ContentDigest>,
    pub validation_profile_digests: BTreeSet<ContentDigest>,
    pub validator_digest: ContentDigest,
    pub environment_digest: ContentDigest,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AttemptCoordinationIdentity<'a> {
    attempt_id: &'a WorkAttemptId,
    contract: &'a crate::WorkContractRef,
    session_id: &'a SessionId,
    thread_id: &'a ThreadId,
    environment_id: &'a zeta_environment::EnvId,
    roots: &'a [crate::RootCheckpoint],
    primary_root_dir_id: &'a DirId,
    execution_id: &'a Option<WorkExecutionId>,
    result: &'a Option<crate::WorkAttemptResult>,
}

/// Binds the immutable coordination facts that explain one verification selection.
pub fn verification_coordination_digest(
    run: &WorkRun,
    ordered_results: &[WorkResultRef],
) -> Result<ContentDigest, WorkCoordinationError> {
    let selected = ordered_results
        .iter()
        .map(|result| result.attempt_id.clone())
        .collect::<BTreeSet<_>>();
    if selected.len() != ordered_results.len() {
        return Err(WorkCoordinationError::InvalidInput(
            "verification coordination identity repeats a WorkAttempt".into(),
        ));
    }
    let mut attempts = Vec::with_capacity(ordered_results.len());
    let mut contract_refs = BTreeSet::new();
    let mut decision_ids = BTreeSet::new();
    for result in ordered_results {
        let attempt = run
            .attempts
            .get(&result.attempt_id)
            .ok_or_else(|| WorkCoordinationError::NotFound(result.attempt_id.to_string()))?;
        if attempt
            .result
            .as_ref()
            .is_none_or(|sealed| sealed.result_digest != result.result_digest)
        {
            return Err(WorkCoordinationError::InvalidInput(
                "verification coordination identity names a mismatched result".into(),
            ));
        }
        let contract = run
            .contract(&attempt.contract.contract_id, attempt.contract.revision)
            .ok_or_else(|| {
                WorkCoordinationError::NotFound(attempt.contract.contract_id.to_string())
            })?;
        contract_refs.insert(attempt.contract.clone());
        decision_ids.extend(contract.decision_ids.iter().cloned());
        attempts.push(AttemptCoordinationIdentity {
            attempt_id: &attempt.attempt_id,
            contract: &attempt.contract,
            session_id: &attempt.session_id,
            thread_id: &attempt.thread_id,
            environment_id: &attempt.environment_id,
            roots: &attempt.roots,
            primary_root_dir_id: &attempt.primary_root_dir_id,
            execution_id: &attempt.execution_id,
            result: &attempt.result,
        });
    }
    let contracts = contract_refs
        .iter()
        .map(|contract| {
            run.contract(&contract.contract_id, contract.revision)
                .ok_or_else(|| WorkCoordinationError::NotFound(contract.contract_id.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let decisions = decision_ids
        .iter()
        .map(|decision_id| {
            run.decisions
                .get(decision_id)
                .ok_or_else(|| WorkCoordinationError::NotFound(decision_id.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let relations = run
        .relations
        .values()
        .filter(|relation| {
            selected.contains(&relation.source_attempt_id)
                || selected.contains(&relation.target_attempt_id)
        })
        .collect::<Vec<_>>();
    let conflicts = run
        .conflicts
        .values()
        .filter(|conflict| {
            conflict
                .attempt_ids
                .iter()
                .any(|attempt_id| selected.contains(attempt_id))
        })
        .collect::<Vec<_>>();
    let encoded = serde_json::to_vec(&(
        1_u32,
        run.current_goal(),
        run.topology_revision,
        &run.participants,
        attempts,
        contracts,
        decisions,
        relations,
        conflicts,
    ))
    .map_err(|error| WorkCoordinationError::InvalidInput(error.to_string()))?;
    Ok(ContentDigest::sha256(&encoded))
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VerificationCheckOutcome {
    Passed,
    Failed,
    Indeterminate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationCheckEvidence {
    pub check_id: String,
    pub command_digest: ContentDigest,
    pub output_digest: ContentDigest,
    pub outcome: VerificationCheckOutcome,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VerificationConclusion {
    Verified,
    Rejected,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkVerificationStatus {
    Verifying,
    Verified,
    Rejected,
    Indeterminate,
    Stale,
}

/// Durable independent-verification record for one exact input identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkVerification {
    pub verification_key: ContentDigest,
    pub input: WorkVerificationInput,
    pub status: WorkVerificationStatus,
    pub checks: Vec<VerificationCheckEvidence>,
    pub evidence_digest: Option<ContentDigest>,
    pub reason: Option<String>,
    pub stale_reason: Option<String>,
}

pub fn verification_key(
    work_run_id: &zeta_protocol::WorkRunId,
    input: &WorkVerificationInput,
) -> Result<ContentDigest, WorkCoordinationError> {
    let encoded = serde_json::to_vec(&(1_u32, work_run_id, input))
        .map_err(|error| WorkCoordinationError::InvalidInput(error.to_string()))?;
    Ok(ContentDigest::sha256(&encoded))
}

pub(crate) fn verification_evidence_digest(
    verification_key: &ContentDigest,
    conclusion: VerificationConclusion,
    checks: &[VerificationCheckEvidence],
    reason: &str,
) -> Result<ContentDigest, WorkCoordinationError> {
    let encoded = serde_json::to_vec(&(1_u32, verification_key, conclusion, checks, reason))
        .map_err(|error| WorkCoordinationError::InvalidInput(error.to_string()))?;
    Ok(ContentDigest::sha256(&encoded))
}
