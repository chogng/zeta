use crate::protocol::turn_changes::ChangeSetId;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use ts_rs::TS;
use zeta_environment::EnvId;
use zeta_file_access::DirId;
use zeta_protocol::ContentDigest;
use zeta_protocol::DelegationId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkConflictId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkDecisionId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRelationId;
use zeta_protocol::WorkRunId;

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkRunStatusDto {
    Active,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkGoalDto {
    #[ts(type = "number")]
    pub revision: u64,
    pub objective: String,
    pub acceptance_conditions: Vec<String>,
    pub exclusions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkParticipantRelationDto {
    Root,
    Delegated {
        parent_thread_id: ThreadId,
        delegation_id: DelegationId,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkParticipantDto {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub relation: WorkParticipantRelationDto,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkDecisionDto {
    pub decision_id: WorkDecisionId,
    pub authority: String,
    pub scope: String,
    pub statement: String,
    pub content_digest: ContentDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum GitRootTargetDto {
    Branch {
        name: String,
        expected_head: String,
    },
    UnbornBranch {
        name: String,
        anchor_object_id: String,
    },
    Detached {
        object_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryCheckpointDto {
    pub repository_id: String,
    pub relative_path: String,
    pub target: GitRootTargetDto,
    pub baseline_tree: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum RootStateDto {
    Git {
        repositories: Vec<GitRepositoryCheckpointDto>,
    },
    Directory {
        snapshot_id: String,
    },
}

#[derive(
    Clone, Copy, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize, TS,
)]
#[serde(rename_all = "camelCase")]
pub enum ControlResourceKindDto {
    ProjectInstructions,
    AgentDefinition,
    Skill,
    Hook,
    BuildEntry,
    ValidationProfile,
    PermissionPolicy,
    CoordinationPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ControlResourceBindingDto {
    pub kind: ControlResourceKindDto,
    pub source_dir_id: DirId,
    pub relative_path: String,
    pub scope: String,
    pub precedence: u32,
    pub content_digest: ContentDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RootCheckpointDto {
    pub environment_id: EnvId,
    pub dir_id: DirId,
    pub state: RootStateDto,
    pub control_resources: Vec<ControlResourceBindingDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationSnapshotRefDto {
    pub authority: String,
    pub policy_revision: String,
    pub grant_set_digest: ContentDigest,
    pub granted_effects_digest: ContentDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ValidationProfileRefDto {
    pub name: String,
    pub content_digest: ContentDigest,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkScopeClaimDto {
    pub components: BTreeSet<String>,
    pub paths: BTreeSet<String>,
    pub contracts: BTreeSet<String>,
    pub resources: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkContractRefDto {
    pub contract_id: WorkContractId,
    #[ts(type = "number")]
    pub revision: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkResultRefDto {
    pub attempt_id: WorkAttemptId,
    pub result_digest: ContentDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkContractVersionDto {
    pub contract_id: WorkContractId,
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub goal_revision: u64,
    #[ts(type = "number")]
    pub topology_revision: u64,
    pub owner_thread_id: ThreadId,
    pub objective: String,
    pub acceptance_conditions: Vec<String>,
    pub exclusions: Vec<String>,
    pub environment_id: EnvId,
    pub roots: Vec<RootCheckpointDto>,
    pub primary_root_dir_id: DirId,
    pub authorization: AuthorizationSnapshotRefDto,
    pub decision_ids: BTreeSet<WorkDecisionId>,
    pub upstream_results: Vec<WorkResultRefDto>,
    pub expected_scope: WorkScopeClaimDto,
    pub validation_profile: ValidationProfileRefDto,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkAttemptExecutionStatusDto {
    Planned,
    Exploring,
    Writing,
    Waiting,
    Sealed,
    Failed,
    Interrupted,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkAttemptCoordinationStatusDto {
    Clear,
    ExpansionRequested,
    Conflict,
    Stale,
    Blocked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkAttemptVerificationStatusDto {
    Pending,
    Verifying,
    Verified,
    Rejected,
    Indeterminate,
    Stale,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkAttemptIntegrationStatusDto {
    Idle,
    Queued,
    Integrating,
    Integrated,
    Partial,
    Conflict,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ExternalEffectsStatusDto {
    None,
    Verified,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkAttemptResultDto {
    pub result_digest: ContentDigest,
    pub change_set_ids: Vec<ChangeSetId>,
    pub private_output_digest: ContentDigest,
    pub external_effects_digest: ContentDigest,
    pub external_effects_status: ExternalEffectsStatusDto,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ManagedRootBindingDto {
    pub source_dir_id: DirId,
    pub managed_dir_id: DirId,
    pub root_checkpoint_digest: ContentDigest,
    pub binding_manifest_digest: ContentDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkAttemptWorkspaceDto {
    Provisioning,
    Ready {
        roots: Vec<ManagedRootBindingDto>,
        private_output_dir_id: DirId,
    },
    Failed {
        reason: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkAttemptDto {
    pub attempt_id: WorkAttemptId,
    pub contract: WorkContractRefDto,
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub environment_id: EnvId,
    pub roots: Vec<RootCheckpointDto>,
    pub primary_root_dir_id: DirId,
    pub workspace: WorkAttemptWorkspaceDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub execution_id: Option<WorkExecutionId>,
    pub execution_status: WorkAttemptExecutionStatusDto,
    pub coordination_status: WorkAttemptCoordinationStatusDto,
    pub verification_status: WorkAttemptVerificationStatusDto,
    pub integration_status: WorkAttemptIntegrationStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub waiting_relation_id: Option<WorkRelationId>,
    pub scope_expansion_evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub result: Option<WorkAttemptResultDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub failure: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkWaitConditionDto {
    ExecutionFinished,
    AttemptSealed,
    ExactResult { result_digest: ContentDigest },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkRelationKindDto {
    Observation,
    Wait {
        target_execution_id: WorkExecutionId,
        condition: WorkWaitConditionDto,
    },
    Alternate,
    Handoff {
        target_contract: WorkContractRefDto,
    },
    ResultDependency {
        result_digest: ContentDigest,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkRelationStatusDto {
    Active,
    Waiting,
    Satisfied { evidence_digest: ContentDigest },
    Failed { reason: String },
    Cancelled,
    Stale,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRelationDto {
    pub relation_id: WorkRelationId,
    pub source_attempt_id: WorkAttemptId,
    pub target_attempt_id: WorkAttemptId,
    pub kind: WorkRelationKindDto,
    pub status: WorkRelationStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub resume_execution_status: Option<WorkAttemptExecutionStatusDto>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkConflictStatusDto {
    Open,
    Resolved,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkConflictDto {
    pub conflict_id: WorkConflictId,
    pub attempt_ids: Vec<WorkAttemptId>,
    pub resource: String,
    pub evidence: Vec<String>,
    pub status: WorkConflictStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub resolution_decision_id: Option<WorkDecisionId>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkAttemptChangeEvidenceRefDto {
    pub change_set_id: ChangeSetId,
    pub evidence_digest: ContentDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VerificationChangeSetInputDto {
    pub attempt_id: WorkAttemptId,
    pub change_set: WorkAttemptChangeEvidenceRefDto,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitVerificationRepositoryDto {
    pub repository_id: String,
    pub relative_path: String,
    pub target: GitRootTargetDto,
    pub target_tree: String,
    pub final_tree: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum VerificationRootStateDto {
    Git {
        repositories: Vec<GitVerificationRepositoryDto>,
    },
    Directory {
        target_snapshot_id: String,
        final_snapshot_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRootDto {
    pub source_dir_id: DirId,
    pub checkpoint_digest: ContentDigest,
    pub state: VerificationRootStateDto,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkSerializabilityStatusDto {
    Proven,
    Indeterminate,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkSerializabilityEvidenceDto {
    pub status: WorkSerializabilityStatusDto,
    pub evidence_digest: ContentDigest,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkVerificationInputDto {
    #[ts(type = "number")]
    pub goal_revision: u64,
    #[ts(type = "number")]
    pub topology_revision: u64,
    pub coordination_digest: ContentDigest,
    pub ordered_results: Vec<WorkResultRefDto>,
    pub ordered_change_sets: Vec<VerificationChangeSetInputDto>,
    pub serializability: WorkSerializabilityEvidenceDto,
    pub roots: Vec<VerificationRootDto>,
    pub authorization_digests: BTreeSet<ContentDigest>,
    pub control_resource_digests: BTreeSet<ContentDigest>,
    pub validation_profile_digests: BTreeSet<ContentDigest>,
    pub validator_digest: ContentDigest,
    pub environment_digest: ContentDigest,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum VerificationCheckOutcomeDto {
    Passed,
    Failed,
    Indeterminate,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VerificationCheckEvidenceDto {
    pub check_id: String,
    pub command_digest: ContentDigest,
    pub output_digest: ContentDigest,
    pub outcome: VerificationCheckOutcomeDto,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkVerificationStatusDto {
    Verifying,
    Verified,
    Rejected,
    Indeterminate,
    Stale,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkVerificationDto {
    pub verification_key: ContentDigest,
    pub input: WorkVerificationInputDto,
    pub status: WorkVerificationStatusDto,
    pub checks: Vec<VerificationCheckEvidenceDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub evidence_digest: Option<ContentDigest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub stale_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum IntegrationRootTargetDto {
    Git {
        repository_id: String,
        relative_path: String,
        target: GitRootTargetDto,
        target_tree: String,
        final_tree: String,
    },
    Directory {
        target_snapshot_id: String,
        final_snapshot_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum IntegrationPreparedArtifactDto {
    GitCommit { object_id: String },
    DirectorySnapshot { snapshot_id: String },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum IntegrationRootStatusDto {
    Pending,
    Prepared,
    Published,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkIntegrationRootDto {
    pub root_id: ContentDigest,
    pub source_dir_id: DirId,
    pub target: IntegrationRootTargetDto,
    pub status: IntegrationRootStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub prepared_artifact: Option<IntegrationPreparedArtifactDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub publication_receipt_digest: Option<ContentDigest>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkIntegrationStatusDto {
    Queued,
    Integrating,
    Integrated,
    Partial,
    Conflict,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum IntegrationFailureKindDto {
    Conflict,
    Failure,
    TargetMoved,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationIncidentDto {
    #[ts(type = "number")]
    pub generation: u64,
    pub kind: IntegrationFailureKindDto,
    pub reason: String,
    #[ts(type = "number")]
    pub published_root_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkIntegrationDto {
    pub integration_key: ContentDigest,
    pub verification_key: ContentDigest,
    #[ts(type = "number")]
    pub generation: u64,
    pub status: WorkIntegrationStatusDto,
    pub roots: Vec<WorkIntegrationRootDto>,
    pub incidents: Vec<IntegrationIncidentDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub evidence_digest: Option<ContentDigest>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunDto {
    pub work_run_id: WorkRunId,
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub topology_revision: u64,
    pub status: WorkRunStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub terminal_reason: Option<String>,
    pub goals: Vec<WorkGoalDto>,
    pub participants: Vec<WorkParticipantDto>,
    pub decisions: Vec<WorkDecisionDto>,
    pub contracts: Vec<WorkContractVersionDto>,
    pub attempts: Vec<WorkAttemptDto>,
    pub relations: Vec<WorkRelationDto>,
    pub conflicts: Vec<WorkConflictDto>,
    pub verifications: Vec<WorkVerificationDto>,
    pub integrations: Vec<WorkIntegrationDto>,
}
