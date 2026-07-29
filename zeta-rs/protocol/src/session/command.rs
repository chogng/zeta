use crate::{ModelRef, ThreadId};
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
    },
    SetModel {
        model: ModelRef,
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
