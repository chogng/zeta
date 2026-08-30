use crate::protocol::common::CommandId;
use crate::protocol::common::SessionId;
use crate::protocol::common::ThreadId;
use crate::protocol::work_run_model::WorkParticipantRelationDto;
use crate::protocol::work_run_model::WorkRelationKindDto;
use crate::protocol::work_run_model::WorkRunDto;
use crate::protocol::work_run_model::WorkRunStatusDto;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;
use zeta_protocol::AgentTreeProjection;
use zeta_protocol::ContentDigest;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkRelationId;
use zeta_protocol::WorkRunId;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunSummaryDto {
    pub work_run_id: WorkRunId,
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub topology_revision: u64,
    pub status: WorkRunStatusDto,
    pub objective: String,
    #[ts(type = "number")]
    pub session_count: u64,
    #[ts(type = "number")]
    pub participant_count: u64,
    #[ts(type = "number")]
    pub attempt_count: u64,
    #[ts(type = "number")]
    pub open_conflict_count: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(deny_unknown_fields)]
pub struct WorkRunListParams {}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunListResult {
    pub work_runs: Vec<WorkRunSummaryDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunReadParams {
    pub work_run_id: WorkRunId,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunReadResult {
    pub work_run: WorkRunDto,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkRunCollaborationModeDto {
    SingleAgent,
    Team,
    MultiSession,
}

/// One canonical Session Agent tree composed into a WorkRun view without copying its facts.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunSessionTreeDto {
    pub session_id: SessionId,
    pub agent_tree: AgentTreeProjection,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunViewReadResult {
    pub work_run: WorkRunDto,
    pub collaboration_mode: WorkRunCollaborationModeDto,
    pub session_trees: Vec<WorkRunSessionTreeDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunCreateParams {
    pub command_id: CommandId,
    pub work_run_id: WorkRunId,
    pub root_session_id: SessionId,
    pub root_thread_id: ThreadId,
    pub objective: String,
    pub acceptance_conditions: Vec<String>,
    pub exclusions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunParticipantAddParams {
    pub command_id: CommandId,
    pub work_run_id: WorkRunId,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub relation: WorkParticipantRelationDto,
}

/// Creates an explicit same-Session or cross-Session relationship between exact Attempts.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkRunRelationCreateParams {
    pub command_id: CommandId,
    pub work_run_id: WorkRunId,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub relation_id: WorkRelationId,
    pub source_attempt_id: WorkAttemptId,
    pub target_attempt_id: WorkAttemptId,
    pub kind: WorkRelationKindDto,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunGoalReviseParams {
    pub command_id: CommandId,
    pub work_run_id: WorkRunId,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub objective: String,
    pub acceptance_conditions: Vec<String>,
    pub exclusions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunCancelParams {
    pub command_id: CommandId,
    pub work_run_id: WorkRunId,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub reason: String,
}

/// Requests host-derived replay and independent verification for exact sealed Attempts.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkRunVerificationRequestParams {
    pub command_id: CommandId,
    pub work_run_id: WorkRunId,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub attempt_ids: Vec<WorkAttemptId>,
}

/// Requests host-owned publication of one exact current verified result set.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkRunIntegrationRequestParams {
    pub command_id: CommandId,
    pub work_run_id: WorkRunId,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub verification_key: ContentDigest,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkRunCommandDispositionDto {
    Committed,
    Replayed,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunMutationResult {
    pub disposition: WorkRunCommandDispositionDto,
    pub work_run: WorkRunDto,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunChanged {
    pub work_run: WorkRunDto,
}
