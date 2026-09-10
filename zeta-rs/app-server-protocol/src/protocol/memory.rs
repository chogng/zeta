use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;
use zeta_protocol::CommandId;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryAddParams {
    pub command_id: CommandId,
    pub memory_id: memories::MemoryId,
    pub scope: memories::MemoryScope,
    #[schemars(length(min = 1, max = 256))]
    pub title: String,
    #[schemars(length(min = 1, max = 16384))]
    pub body: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryListParams {
    pub scope: memories::MemoryScope,
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryReadParams {
    pub memory_id: memories::MemoryId,
    pub scope: memories::MemoryScope,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemorySearchParams {
    pub scope: memories::MemoryScope,
    #[schemars(length(min = 1, max = 512))]
    pub query: String,
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryDeleteParams {
    pub command_id: CommandId,
    pub memory_id: memories::MemoryId,
    pub scope: memories::MemoryScope,
    #[ts(type = "number")]
    pub expected_revision: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MemoryChanged {
    pub scope: memories::MemoryScope,
    #[ts(type = "number")]
    pub catalog_revision: u64,
}
