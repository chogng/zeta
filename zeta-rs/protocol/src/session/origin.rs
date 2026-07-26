use crate::ThreadId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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
}
