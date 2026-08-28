use crate::ApprovalMode;
use crate::ContextSeedDigest;
use crate::DelegationId;
use crate::ModelRef;
use crate::ThreadId;
use crate::TurnId;
use crate::WorkspaceBinding;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionCommand {
    Create {
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        model: Option<ModelRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        workspace: Option<WorkspaceBinding>,
    },
    SetModel {
        model: ModelRef,
    },
    SetNextApprovalMode {
        approval_mode: ApprovalMode,
    },
    CreateThread {
        title: String,
    },
    ForkThread {
        parent_thread_id: ThreadId,
        title: String,
    },
    RewindThread {
        parent_thread_id: ThreadId,
        before_turn_id: TurnId,
        title: String,
    },
    SpawnAgentThread {
        parent_thread_id: ThreadId,
        parent_turn_id: TurnId,
        delegation_id: DelegationId,
        context_seed_digest: ContextSeedDigest,
        title: String,
    },
    ArchiveThread {
        thread_id: ThreadId,
    },
    Complete,
    Archive,
}
