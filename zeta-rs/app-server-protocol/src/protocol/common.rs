//! Stable leaf values that may cross the RPC boundary.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use zeta_protocol::CommandId;
pub use zeta_protocol::ItemId;
pub use zeta_protocol::RequestId;
pub use zeta_protocol::SessionId;
pub use zeta_protocol::StreamInstanceId;
pub use zeta_protocol::ThreadId;
pub use zeta_protocol::ToolCallId;
pub use zeta_protocol::ToolName;
pub use zeta_protocol::TurnId;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
pub struct SchemaHash(pub String);

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    #[schemars(length(min = 1))]
    pub name: String,
    #[schemars(length(min = 1))]
    pub version: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    #[serde(default)]
    #[ts(optional)]
    pub notifications: Option<bool>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub agent_interactions: Option<AgentInteractionCapability>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub browser: Option<BrowserCapability>,
}

/// Agent interaction kinds that one client connection can present and resolve.
///
/// App Server uses this declaration only for ephemeral owner selection. It never persists the
/// connection capability in Session or Thread state, and clients must still subscribe to the
/// target Thread before they can become an interaction owner.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentInteractionCapability {
    pub version: u32,
    pub kinds: Vec<zeta_protocol::AgentInteractionKind>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BrowserCapability {
    pub version: u32,
    pub observe: bool,
    pub input: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyParams {}

impl TS for EmptyParams {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_: &ts_rs::Config) -> String {
        "Record<string, never>".into()
    }

    fn inline(_: &ts_rs::Config) -> String {
        "Record<string, never>".into()
    }
}
