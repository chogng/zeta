use crate::RootCheckpoint;
use crate::WorkAttemptWorkspace;
use crate::WorkContractRef;
use serde::Deserialize;
use serde::Serialize;
use zeta_environment::EnvId;
use zeta_file_access::DirId;
use zeta_protocol::ContentDigest;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRelationId;
use zeta_turn_changes::ChangeSetId;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkStartMode {
    Explore,
    Write,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkAttemptExecutionStatus {
    Planned,
    Exploring,
    Writing,
    Waiting,
    Sealed,
    Failed,
    Interrupted,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkAttemptCoordinationStatus {
    Clear,
    ExpansionRequested,
    Conflict,
    Stale,
    Blocked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkAttemptVerificationStatus {
    Pending,
    Verifying,
    Verified,
    Rejected,
    Indeterminate,
    Stale,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkAttemptIntegrationStatus {
    Idle,
    Queued,
    Integrating,
    Integrated,
    Partial,
    Conflict,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalEffectsStatus {
    None,
    Verified,
    Unknown,
}

impl Default for ExternalEffectsStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkAttemptResult {
    pub result_digest: ContentDigest,
    pub change_set_ids: Vec<ChangeSetId>,
    pub private_output_digest: ContentDigest,
    pub external_effects_digest: ContentDigest,
    #[serde(default)]
    pub external_effects_status: ExternalEffectsStatus,
}

/// One Agent execution bound to an exact participant, contract and root set.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkAttempt {
    pub attempt_id: WorkAttemptId,
    pub contract: WorkContractRef,
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub environment_id: EnvId,
    pub roots: Vec<RootCheckpoint>,
    pub primary_root_dir_id: DirId,
    pub workspace: WorkAttemptWorkspace,
    pub execution_id: Option<WorkExecutionId>,
    pub execution_status: WorkAttemptExecutionStatus,
    pub coordination_status: WorkAttemptCoordinationStatus,
    pub verification_status: WorkAttemptVerificationStatus,
    pub integration_status: WorkAttemptIntegrationStatus,
    pub waiting_relation_id: Option<WorkRelationId>,
    pub scope_expansion_evidence: Vec<String>,
    pub result: Option<WorkAttemptResult>,
    pub failure: Option<String>,
}

impl WorkAttempt {
    pub(crate) fn is_terminal(&self) -> bool {
        matches!(
            self.execution_status,
            WorkAttemptExecutionStatus::Sealed
                | WorkAttemptExecutionStatus::Failed
                | WorkAttemptExecutionStatus::Interrupted
                | WorkAttemptExecutionStatus::Cancelled
        )
    }
}
