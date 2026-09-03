use crate::ToolCallId;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

/// One stable, secret-free hop in the source chain of a durable Tool Call binding.
///
/// Hosts append hops in distribution-to-execution order. Mutable process identities, credentials,
/// session handles, and filesystem paths must never be stored here.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ToolSourceProvenance {
    Product {
        component: String,
    },
    Plugin {
        plugin_id: String,
        version: String,
        package_digest: String,
        contribution_id: String,
    },
    Mcp {
        server_id: String,
        remote_name: String,
        #[ts(type = "number")]
        catalog_generation: u64,
        #[ts(type = "number")]
        connection_generation: u64,
    },
    Dynamic {
        name: String,
    },
    Extension {
        id: String,
    },
    System {
        id: String,
    },
}

/// Identifies how the model-visible Tool Call reached the ordinary tool scheduler.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ToolCallCaller {
    Direct,
    CodeMode {
        parent_tool_call_id: ToolCallId,
        cell_id: String,
        runtime_call_id: String,
    },
}

/// Durable binding between one Tool Call and the exact definition/source generation it selected.
///
/// Runtime keys are intentionally absent. Recovery must compare this value with an available
/// immutable source snapshot and fail closed when the exact binding can no longer be restored.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallBinding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_incarnation: Option<String>,
    #[ts(type = "number")]
    pub registry_generation: u64,
    pub definition_digest: String,
    pub source_chain: Vec<ToolSourceProvenance>,
    pub caller: ToolCallCaller,
}
