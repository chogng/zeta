use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

/// Exact installed package plus each independent Plugin authority layer.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginPackageDto {
    pub id: String,
    pub version: String,
    pub digest: String,
    pub enabled: bool,
    pub granted: bool,
    pub effective: bool,
    pub revoked: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PluginMarketplaceModeDto {
    Managed,
    RemoteManaged,
    LocalDevelopment,
}

/// One exact package offered by a host-registered Marketplace.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketplacePackageDto {
    pub marketplace_id: String,
    pub marketplace_mode: PluginMarketplaceModeDto,
    pub marketplace_revision: String,
    pub id: String,
    pub version: String,
    pub digest: String,
    pub installed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketplaceListResult {
    pub packages: Vec<PluginMarketplacePackageDto>,
}

/// Current durable Plugin authority projection.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginListResult {
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub activation_generation: u64,
    pub packages: Vec<PluginPackageDto>,
}

/// Exact package target for retry-safe Plugin lifecycle commands.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginPackageCommandParams {
    pub command_id: String,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub id: String,
    pub version: String,
    pub digest: String,
}

/// Exact Marketplace entry selected without exposing a host filesystem path.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketplaceCommandParams {
    pub command_id: String,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub marketplace_id: String,
    pub id: String,
    pub version: String,
    pub digest: String,
}

/// Result of one committed or exactly replayed Plugin authority command.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandResultDto {
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub activation_generation: u64,
    pub disposition: PluginCommandDispositionDto,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PluginCommandDispositionDto {
    Updated,
    Replayed,
}

/// Notification emitted for every committed Plugin authority revision.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginsChanged {
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub activation_generation: u64,
}
