use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use zeta_environment::EnvId;
use zeta_file_access::DirId;
use zeta_protocol::ContentDigest;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkDecisionId;

/// Exact branch or detached target whose old value is checked before publication.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum GitRootTarget {
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

/// One repository captured inside a selected directory root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryCheckpoint {
    pub repository_id: String,
    pub relative_path: String,
    pub target: GitRootTarget,
    pub baseline_tree: String,
}

/// Immutable content identity selected for one work root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum RootState {
    Git {
        repositories: Vec<GitRepositoryCheckpoint>,
    },
    Directory {
        snapshot_id: String,
    },
}

/// A behavior-changing resource frozen into one root checkpoint.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ControlResourceKind {
    ProjectInstructions,
    AgentDefinition,
    Skill,
    Hook,
    BuildEntry,
    ValidationProfile,
    PermissionPolicy,
    CoordinationPolicy,
}

/// Provenance and precedence of one exact control resource.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlResourceBinding {
    pub kind: ControlResourceKind,
    pub source_dir_id: DirId,
    pub relative_path: String,
    pub scope: String,
    pub precedence: u32,
    pub content_digest: ContentDigest,
}

/// Immutable code and control input for one directory selected by a contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootCheckpoint {
    pub environment_id: EnvId,
    pub dir_id: DirId,
    pub state: RootState,
    pub control_resources: Vec<ControlResourceBinding>,
}

/// Reference to an authorization already issued by the permission authority.
///
/// This record binds an exact permission result; it never grants capability itself.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationSnapshotRef {
    pub authority: String,
    pub policy_revision: String,
    pub grant_set_digest: ContentDigest,
    pub granted_effects_digest: ContentDigest,
}

/// Exact validation profile selected before an attempt begins.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationProfileRef {
    pub name: String,
    pub content_digest: ContentDigest,
}

/// Planning-only claim used to decide whether work may run concurrently.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkScopeClaim {
    pub components: BTreeSet<String>,
    pub paths: BTreeSet<String>,
    pub contracts: BTreeSet<String>,
    pub resources: BTreeSet<String>,
}

/// Stable reference to one immutable work contract version.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkContractRef {
    pub contract_id: WorkContractId,
    pub revision: u64,
}

/// Sealed result accepted as an input by a later contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkResultRef {
    pub attempt_id: WorkAttemptId,
    pub result_digest: ContentDigest,
}

/// One immutable version of an authorized development contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkContractVersion {
    pub contract_id: WorkContractId,
    pub revision: u64,
    pub goal_revision: u64,
    pub topology_revision: u64,
    pub owner_thread_id: ThreadId,
    pub objective: String,
    pub acceptance_conditions: Vec<String>,
    pub exclusions: Vec<String>,
    pub environment_id: EnvId,
    pub roots: Vec<RootCheckpoint>,
    pub primary_root_dir_id: DirId,
    pub authorization: AuthorizationSnapshotRef,
    pub decision_ids: BTreeSet<WorkDecisionId>,
    pub upstream_results: Vec<WorkResultRef>,
    pub expected_scope: WorkScopeClaim,
    pub validation_profile: ValidationProfileRef,
}

impl WorkContractVersion {
    pub fn reference(&self) -> WorkContractRef {
        WorkContractRef {
            contract_id: self.contract_id.clone(),
            revision: self.revision,
        }
    }
}
