//! Stable leaf values that may cross the RPC boundary.

use serde::{Deserialize, Serialize};

pub use zeta_protocol::ThreadId;
pub use zeta_protocol::TurnId;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchemaHash(pub String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolVersions {
    pub min: u32,
    pub max: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClientCapabilities {
    #[serde(default)]
    pub notifications: bool,
    #[serde(default)]
    pub browser: Option<BrowserCapability>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BrowserCapability {
    pub version: u32,
    pub observe: bool,
    pub input: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}
