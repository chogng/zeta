use crate::ExternalEffectsStatus;
use crate::WorkAttempt;
use crate::WorkCoordinationError;
use crate::root_checkpoint_digest;
use serde::Deserialize;
use serde::Serialize;
use zeta_protocol::ContentDigest;
use zeta_protocol::WorkRunId;
use zeta_turn_changes::ChangeSetId;

/// Content identity of one sealed ChangeSet in its required result order.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkAttemptChangeEvidenceRef {
    pub change_set_id: ChangeSetId,
    pub evidence_digest: ContentDigest,
}

/// Canonical result identity for one exact WorkAttempt execution.
pub fn work_attempt_result_digest(
    work_run_id: &WorkRunId,
    topology_revision: u64,
    attempt: &WorkAttempt,
    change_sets: &[WorkAttemptChangeEvidenceRef],
    private_output_digest: &ContentDigest,
    external_effects_digest: &ContentDigest,
    external_effects_status: ExternalEffectsStatus,
) -> Result<ContentDigest, WorkCoordinationError> {
    let execution_id = attempt.execution_id.as_ref().ok_or_else(|| {
        WorkCoordinationError::InvalidInput(
            "a WorkAttempt result requires an execution identity".into(),
        )
    })?;
    let roots = attempt
        .roots
        .iter()
        .map(root_checkpoint_digest)
        .collect::<Result<Vec<_>, _>>()?;
    let identity = (
        work_run_id,
        topology_revision,
        &attempt.attempt_id,
        execution_id,
        &attempt.session_id,
        &attempt.thread_id,
        &attempt.environment_id,
        &attempt.contract,
        roots,
    );
    let encoded = serde_json::to_vec(&(
        1_u32,
        identity,
        change_sets,
        private_output_digest,
        external_effects_digest,
        external_effects_status,
    ))
    .map_err(|error| WorkCoordinationError::InvalidInput(error.to_string()))?;
    Ok(ContentDigest::sha256(&encoded))
}
