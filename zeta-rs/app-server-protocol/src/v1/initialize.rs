use crate::common::{ClientCapabilities, ClientInfo, ProtocolVersions, SchemaHash, ServerInfo};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub client_info: ClientInfo,
    pub protocol_versions: ProtocolVersions,
    #[serde(default)]
    pub capabilities: ClientCapabilities,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub server_info: ServerInfo,
    pub protocol_version: u32,
    pub schema_hash: SchemaHash,
    pub capabilities: ServerCapabilities,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServerCapabilities {
    pub threads: bool,
    pub turns: bool,
    pub resources: bool,
}
