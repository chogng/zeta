use crate::AuthorizationSnapshotRef;
use crate::ExternalEffectsStatus;
use crate::IntegrationFailureKind;
use crate::IntegrationPreparedArtifact;
use crate::ManagedRootBinding;
use crate::RootCheckpoint;
use crate::ValidationProfileRef;
use crate::VerificationCheckEvidence;
use crate::VerificationConclusion;
use crate::WorkContractRef;
use crate::WorkParticipant;
use crate::WorkRelationKind;
use crate::WorkResultRef;
use crate::WorkScopeClaim;
use crate::WorkStartMode;
use crate::WorkVerificationInput;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use zeta_environment::EnvId;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkConflictId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkDecisionId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRelationId;
use zeta_protocol::WorkRunId;
use zeta_turn_changes::ChangeSetId;

/// Full candidate content for one immutable work-contract version.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkContractDraft {
    pub contract_id: WorkContractId,
    pub goal_revision: u64,
    pub topology_revision: u64,
    pub owner_thread_id: ThreadId,
    pub objective: String,
    pub acceptance_conditions: Vec<String>,
    pub exclusions: Vec<String>,
    pub environment_id: EnvId,
    pub roots: Vec<RootCheckpoint>,
    pub primary_root_dir_id: zeta_file_access::DirId,
    pub authorization: AuthorizationSnapshotRef,
    pub decision_ids: BTreeSet<WorkDecisionId>,
    pub upstream_results: Vec<WorkResultRef>,
    pub expected_scope: WorkScopeClaim,
    pub validation_profile: ValidationProfileRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ResolveWaitOutcome {
    Satisfied { evidence_digest: ContentDigest },
    Failed { reason: String },
    Cancelled,
    SourceStale,
}

/// Typed mutation accepted by the deterministic WorkRun reducer.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkRunCommand {
    Create {
        objective: String,
        acceptance_conditions: Vec<String>,
        exclusions: Vec<String>,
        root_participant: WorkParticipant,
    },
    ReviseGoal {
        objective: String,
        acceptance_conditions: Vec<String>,
        exclusions: Vec<String>,
    },
    AddParticipant {
        participant: WorkParticipant,
    },
    RecordDecision {
        decision_id: WorkDecisionId,
        authority: String,
        scope: String,
        statement: String,
    },
    CreateContract {
        contract: WorkContractDraft,
    },
    ReviseContract {
        expected_contract_revision: u64,
        contract: WorkContractDraft,
    },
    CreateAttempt {
        attempt_id: WorkAttemptId,
        contract: WorkContractRef,
        participant_thread_id: ThreadId,
    },
    RecordAttemptWorkspaceReady {
        attempt_id: WorkAttemptId,
        roots: Vec<ManagedRootBinding>,
        private_output_dir_id: zeta_file_access::DirId,
    },
    FailAttemptWorkspace {
        attempt_id: WorkAttemptId,
        reason: String,
    },
    BeginAttempt {
        attempt_id: WorkAttemptId,
        execution_id: WorkExecutionId,
        mode: WorkStartMode,
    },
    RequestScopeExpansion {
        attempt_id: WorkAttemptId,
        evidence: Vec<String>,
    },
    RecordConflict {
        conflict_id: WorkConflictId,
        attempt_ids: Vec<WorkAttemptId>,
        resource: String,
        evidence: Vec<String>,
    },
    ResolveConflict {
        conflict_id: WorkConflictId,
        decision_id: WorkDecisionId,
    },
    CreateRelation {
        relation_id: WorkRelationId,
        source_attempt_id: WorkAttemptId,
        target_attempt_id: WorkAttemptId,
        kind: WorkRelationKind,
    },
    ResolveWait {
        relation_id: WorkRelationId,
        target_attempt_id: WorkAttemptId,
        target_execution_id: WorkExecutionId,
        outcome: ResolveWaitOutcome,
    },
    SealAttempt {
        attempt_id: WorkAttemptId,
        result_digest: ContentDigest,
        change_set_ids: Vec<ChangeSetId>,
        private_output_digest: ContentDigest,
        external_effects_digest: ContentDigest,
        #[serde(default)]
        external_effects_status: ExternalEffectsStatus,
    },
    BeginVerification {
        input: WorkVerificationInput,
    },
    FinishVerification {
        verification_key: ContentDigest,
        conclusion: VerificationConclusion,
        checks: Vec<VerificationCheckEvidence>,
        reason: String,
    },
    MarkVerificationStale {
        verification_key: ContentDigest,
        reason: String,
    },
    QueueIntegration {
        verification_key: ContentDigest,
    },
    RecordIntegrationRootPrepared {
        integration_key: ContentDigest,
        generation: u64,
        root_id: ContentDigest,
        artifact: IntegrationPreparedArtifact,
    },
    BeginIntegration {
        integration_key: ContentDigest,
        generation: u64,
    },
    RecordIntegrationRootPublished {
        integration_key: ContentDigest,
        generation: u64,
        root_id: ContentDigest,
        receipt_digest: ContentDigest,
    },
    FailIntegration {
        integration_key: ContentDigest,
        generation: u64,
        kind: IntegrationFailureKind,
        reason: String,
    },
    ResumeIntegration {
        integration_key: ContentDigest,
        generation: u64,
    },
    FailAttempt {
        attempt_id: WorkAttemptId,
        message: String,
    },
    InterruptAttempt {
        attempt_id: WorkAttemptId,
        message: String,
    },
    CancelAttempt {
        attempt_id: WorkAttemptId,
        reason: String,
    },
    Complete,
    Cancel {
        reason: String,
    },
}

/// Retry-safe mutation of one WorkRun at an exact aggregate revision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRunCommandRequest {
    pub command_id: CommandId,
    pub work_run_id: WorkRunId,
    pub expected_revision: u64,
    pub command: WorkRunCommand,
}
