use crate::WorkAttempt;
use crate::WorkConflict;
use crate::WorkContractVersion;
use crate::WorkIntegration;
use crate::WorkRelation;
use crate::WorkVerification;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use zeta_protocol::ContentDigest;
use zeta_protocol::DelegationId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkConflictId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkDecisionId;
use zeta_protocol::WorkRelationId;
use zeta_protocol::WorkRunId;

pub const WORK_RUN_SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkRunStatus {
    Active,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkGoal {
    pub revision: u64,
    pub objective: String,
    pub acceptance_conditions: Vec<String>,
    pub exclusions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkParticipantRelation {
    Root,
    Delegated {
        parent_thread_id: ThreadId,
        delegation_id: DelegationId,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkParticipant {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub relation: WorkParticipantRelation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkDecision {
    pub decision_id: WorkDecisionId,
    pub authority: String,
    pub scope: String,
    pub statement: String,
    pub content_digest: ContentDigest,
}

/// Complete durable coordination record for one common development goal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRun {
    pub schema_version: u32,
    pub work_run_id: WorkRunId,
    pub revision: u64,
    pub topology_revision: u64,
    pub status: WorkRunStatus,
    pub terminal_reason: Option<String>,
    pub goals: Vec<WorkGoal>,
    pub participants: BTreeMap<ThreadId, WorkParticipant>,
    pub decisions: BTreeMap<WorkDecisionId, WorkDecision>,
    pub contracts: BTreeMap<WorkContractId, Vec<WorkContractVersion>>,
    pub attempts: BTreeMap<WorkAttemptId, WorkAttempt>,
    pub relations: BTreeMap<WorkRelationId, WorkRelation>,
    pub conflicts: BTreeMap<WorkConflictId, WorkConflict>,
    #[serde(default)]
    pub verifications: BTreeMap<ContentDigest, WorkVerification>,
    #[serde(default)]
    pub integrations: BTreeMap<ContentDigest, WorkIntegration>,
}

impl WorkRun {
    /// Checks every durable identity, reference and state relationship in this snapshot.
    ///
    /// Stores and service boundaries call this after deserialization so corrupted records cannot
    /// reach reducers or be returned as authoritative state.
    pub fn validate(&self) -> Result<(), crate::WorkCoordinationError> {
        crate::snapshot_validation::work_run(self)
    }

    pub fn current_goal(&self) -> Option<&WorkGoal> {
        self.goals.last()
    }

    pub fn contract(
        &self,
        contract_id: &WorkContractId,
        revision: u64,
    ) -> Option<&WorkContractVersion> {
        self.contracts
            .get(contract_id)?
            .iter()
            .find(|contract| contract.revision == revision)
    }

    pub fn latest_contract(&self, contract_id: &WorkContractId) -> Option<&WorkContractVersion> {
        self.contracts.get(contract_id)?.last()
    }

    pub fn session_count(&self) -> usize {
        self.participants
            .values()
            .map(|participant| &participant.session_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    }

    /// Returns attempts that currently own their Thread execution writer.
    pub fn active_writers(&self) -> impl Iterator<Item = &WorkAttempt> {
        self.attempts.values().filter(|attempt| {
            matches!(
                attempt.execution_status,
                crate::WorkAttemptExecutionStatus::Exploring
                    | crate::WorkAttemptExecutionStatus::Writing
            )
        })
    }
}
