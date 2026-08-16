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
    #[serde(default)]
    #[ts(optional = nullable)]
    pub workspace_trust_host: Option<WorkspaceTrustHostCapability>,
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
    /// Exact client-hosted dynamic tool names this connection can execute.
    ///
    /// This is ephemeral routing authority, not an executable registration. App Server still
    /// accepts and validates definitions through its composition root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dynamic_tools: Option<Vec<zeta_protocol::ToolName>>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BrowserCapability {
    pub version: u32,
    pub observe: bool,
    pub input: bool,
}

/// Declares that this connection is owned by a product host that can collect Workspace trust.
///
/// This is an authority boundary for host-only protocol operations. Renderer or extension
/// connections must not declare it merely because they can display a trust prompt.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTrustHostCapability {
    pub version: u32,
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
