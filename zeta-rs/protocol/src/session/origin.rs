use crate::DelegationId;
use crate::ThreadId;
use crate::TurnId;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

/// How a Thread entered a product Session.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ThreadOrigin {
    Root,
    Fork {
        parent_thread_id: ThreadId,
        #[ts(type = "number")]
        parent_sequence: u64,
    },
    Rewind {
        parent_thread_id: ThreadId,
        #[ts(type = "number")]
        parent_sequence: u64,
        before_turn_id: TurnId,
    },
    AgentSpawn {
        parent_thread_id: ThreadId,
        #[ts(type = "number")]
        parent_sequence: u64,
        delegation_id: DelegationId,
    },
}
