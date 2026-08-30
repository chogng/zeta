use crate::WorkAttemptExecutionStatus;
use crate::WorkContractRef;
use serde::Deserialize;
use serde::Serialize;
use zeta_protocol::ContentDigest;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkConflictId;
use zeta_protocol::WorkDecisionId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRelationId;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkWaitCondition {
    ExecutionFinished,
    AttemptSealed,
    ExactResult { result_digest: ContentDigest },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkRelationKind {
    Observation,
    Wait {
        target_execution_id: WorkExecutionId,
        condition: WorkWaitCondition,
    },
    Alternate,
    Handoff {
        target_contract: WorkContractRef,
    },
    ResultDependency {
        result_digest: ContentDigest,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkRelationStatus {
    Active,
    Waiting,
    Satisfied { evidence_digest: ContentDigest },
    Failed { reason: String },
    Cancelled,
    Stale,
}

/// Explicit, versioned dependency between two exact work attempts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRelation {
    pub relation_id: WorkRelationId,
    pub source_attempt_id: WorkAttemptId,
    pub target_attempt_id: WorkAttemptId,
    pub kind: WorkRelationKind,
    pub status: WorkRelationStatus,
    pub resume_execution_status: Option<WorkAttemptExecutionStatus>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkConflictStatus {
    Open,
    Resolved,
}

/// Durable reason why existing attempts cannot continue under their current contracts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkConflict {
    pub conflict_id: WorkConflictId,
    pub attempt_ids: Vec<WorkAttemptId>,
    pub resource: String,
    pub evidence: Vec<String>,
    pub status: WorkConflictStatus,
    pub resolution_decision_id: Option<WorkDecisionId>,
}
