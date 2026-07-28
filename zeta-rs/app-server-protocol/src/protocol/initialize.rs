use crate::protocol::common::{ClientCapabilities, ClientInfo, SchemaHash, ServerInfo};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub client_info: ClientInfo,
    #[serde(default)]
    pub capabilities: ClientCapabilities,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub server_info: ServerInfo,
    pub schema_hash: SchemaHash,
    pub capabilities: ServerCapabilities,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    pub sessions: bool,
    pub threads: bool,
    pub turns: bool,
    pub resources: bool,
    pub typst: bool,
    pub update_replay: bool,
}
