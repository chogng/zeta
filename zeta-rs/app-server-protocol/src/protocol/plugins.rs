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

/// Product-facing trust classification of one Marketplace source.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PluginMarketplaceTrustDto {
    ProductManaged,
    VerifiedExternal,
    LocalDevelopment,
}

/// Counts of each runtime-neutral capability kind carried by one Plugin package.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginContributionSummaryDto {
    pub skills: u32,
    pub mcp_servers: u32,
    pub connectors: u32,
    pub assets: u32,
    pub editor_extensions: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PluginWorkspaceAccessDto {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PluginCredentialKindDto {
    SecretText,
}

/// One declared capability ceiling shown before a Plugin package is installed or granted.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PluginPermissionDto {
    Process { executable: String },
    Workspace { access: PluginWorkspaceAccessDto },
    Network { hosts: Vec<String> },
}

/// One credential request shown without containing any secret value.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginCredentialSlotDto {
    pub name: String,
    pub kind: PluginCredentialKindDto,
    pub required_for: Vec<String>,
}

/// One exact package offered by a host-registered Marketplace.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketplacePackageDto {
    pub marketplace_id: String,
    pub marketplace_mode: PluginMarketplaceModeDto,
    pub marketplace_trust: PluginMarketplaceTrustDto,
    pub marketplace_revision: String,
    pub id: String,
    pub publisher: String,
    pub version: String,
    pub digest: String,
    pub display_name: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub compatibility_zeta: String,
    pub contributions: PluginContributionSummaryDto,
    pub permissions: Vec<PluginPermissionDto>,
    pub credential_slots: Vec<PluginCredentialSlotDto>,
    #[ts(type = "number")]
    pub package_file_count: u64,
    #[ts(type = "number")]
    pub package_size_bytes: u64,
    pub installed: bool,
    pub enabled: bool,
    pub granted: bool,
    pub effective: bool,
    pub revoked: bool,
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
