use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

/// Opens one shared structured-document room or joins an existing room.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentCollaborationOpenParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub room_id: Option<String>,
    #[schemars(length(min = 1))]
    pub client_id: String,
    #[schemars(length(min = 1))]
    pub schema_id: String,
    /// Serialized, schema-validated document supplied only when creating a room.
    #[schemars(length(min = 1))]
    pub document: String,
}

/// Versioned canonical document snapshot returned while opening or resynchronizing a room.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentCollaborationSnapshot {
    pub room_id: String,
    #[ts(type = "number")]
    pub version: u64,
    pub document: String,
}

/// Result of opening a shared structured-document room.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentCollaborationOpenResult {
    pub client_id: String,
    pub schema_id: String,
    pub snapshot: DocumentCollaborationSnapshot,
}

/// One server-ordered opaque Gama transaction.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentCollaborationUpdate {
    pub room_id: String,
    pub client_id: String,
    #[ts(type = "number")]
    pub sequence: u64,
    #[ts(type = "number")]
    pub base_version: u64,
    #[ts(type = "number")]
    pub version: u64,
    /// Serialized Gama transaction. The renderer validates it against its schema before use.
    pub transaction: String,
}

/// Submits one optimistic local transaction alongside its resulting document snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentCollaborationSubmitParams {
    #[schemars(length(min = 1))]
    pub room_id: String,
    #[schemars(length(min = 1))]
    pub client_id: String,
    #[ts(type = "number")]
    pub sequence: u64,
    #[ts(type = "number")]
    pub base_version: u64,
    #[schemars(length(min = 1))]
    pub transaction: String,
    /// Serialized, schema-validated document after applying `transaction` optimistically.
    #[schemars(length(min = 1))]
    pub document: String,
}

/// Server outcome for an optimistic document submission.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum DocumentCollaborationSubmitResult {
    Accepted {
        update: DocumentCollaborationUpdate,
    },
    Conflict {
        updates: Vec<DocumentCollaborationUpdate>,
    },
    Resync {
        snapshot: DocumentCollaborationSnapshot,
    },
}
