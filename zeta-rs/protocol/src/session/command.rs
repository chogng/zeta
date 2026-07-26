use crate::ThreadId;
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
    },
    CreateThread {
        title: String,
    },
    ForkThread {
        parent_thread_id: ThreadId,
        title: String,
    },
    ArchiveThread {
        thread_id: ThreadId,
    },
    Complete,
    Archive,
}
