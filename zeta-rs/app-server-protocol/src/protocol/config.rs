use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;
use zeta_protocol::{CommandId, Patch};

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "ModelRef")]
pub struct ModelRefDto {
    #[schemars(length(min = 1))]
    pub provider: String,
    #[schemars(length(min = 1))]
    pub model: String,
}

/// User-facing selection for the model that reviews approval requests.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "type"
)]
#[ts(rename = "ApprovalReviewModelSelection")]
pub enum ApprovalReviewModelSelectionDto {
    Automatic,
    Explicit { model: ModelRefDto },
}

/// Non-secret declarative provider settings exposed through the App Server contract.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigDto {
    #[schemars(length(min = 1))]
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub max_output_tokens: Option<u32>,
}

/// Non-secret credential binding for a standalone MCP server declaration.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum McpCredentialBindingDto {
    Unauthenticated,
    Reference { credential_ref: String },
}

/// Desired enablement for a configured MCP server.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum McpServerEnablementDto {
    Disabled,
    Enabled,
}

/// Non-secret transport declaration for a standalone MCP server.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum McpTransportDto {
    Stdio { command: String, args: Vec<String> },
    StreamableHttp { url: String },
}

/// Desired, runtime-free standalone MCP server configuration.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfigDto {
    #[schemars(length(min = 1))]
    pub id: String,
    #[schemars(length(min = 1))]
    pub display_name: String,
    pub transport: McpTransportDto,
    pub credential: McpCredentialBindingDto,
    pub enablement: McpServerEnablementDto,
}

/// Desired enablement for one configured user Skill source.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SkillSourceEnablementDto {
    Disabled,
    Enabled,
}

/// Runtime-free declaration for one user-owned Skill source.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillSourceConfigDto {
    #[schemars(length(min = 1))]
    pub id: String,
    #[schemars(length(min = 1))]
    pub root_reference: String,
    pub enablement: SkillSourceEnablementDto,
}

/// Desired participation of one exact Plugin request in future activation resolution.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PluginRequestEnablementDto {
    Disabled,
    Enabled,
}

/// Declarative request for one exact Plugin package.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginRequestDto {
    #[schemars(length(min = 1))]
    pub plugin_id: String,
    #[schemars(length(min = 1))]
    pub version: String,
    pub enablement: PluginRequestEnablementDto,
}

/// Safe-point event that may request a Hook execution.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum HookEventDto {
    BeforeTool,
    AfterTool,
    TurnCompleted,
}

/// Desired enablement of one Hook declaration.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum HookEnablementDto {
    Disabled,
    Enabled,
}

/// Optional exact tool-name matcher for tool-related Hook events.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HookMatcherDto {
    pub tool_names: Vec<String>,
}

/// Runtime-free Hook action. Execution still requires policy and sandbox approval.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum HookActionDto {
    Process { program: String, args: Vec<String> },
}

/// Declarative Hook configuration exposed through the App Server contract.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HookConfigDto {
    #[schemars(length(min = 1))]
    pub id: String,
    pub event: HookEventDto,
    pub matcher: HookMatcherDto,
    pub action: HookActionDto,
    pub enablement: HookEnablementDto,
}

/// Durable user intent for one language server.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LanguageServerModeDto {
    Disabled,
    Automatic,
    Enabled,
}

/// Runtime-free language-server preference exposed through the App Server contract.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageServerConfigDto {
    pub mode: LanguageServerModeDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub executable: Option<String>,
}

/// Current user configuration snapshot returned by `config/read`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConfigReadResult {
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub generation: u64,
    pub preferred_model: Option<ModelRefDto>,
    pub approval_review_model: ApprovalReviewModelSelectionDto,
    pub providers: BTreeMap<String, ProviderConfigDto>,
    pub mcp_servers: BTreeMap<String, McpServerConfigDto>,
    pub skill_sources: BTreeMap<String, SkillSourceConfigDto>,
    pub plugin_requests: BTreeMap<String, PluginRequestDto>,
    pub hooks: BTreeMap<String, HookConfigDto>,
    pub language_servers: BTreeMap<String, LanguageServerConfigDto>,
}

/// Notification payload emitted after a durable Config authority commit.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChanged {
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub generation: u64,
}

/// Compact result of a retry-safe configuration mutation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConfigCommandResult {
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub generation: u64,
    pub disposition: ConfigCommandDispositionDto,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ConfigCommandDispositionDto {
    Updated,
    Replayed,
}

/// Patch for user preferences at one expected Config revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConfigUpdateParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    #[schemars(with = "Option<ModelRefDto>")]
    #[ts(as = "Option<ModelRefDto>", optional = nullable)]
    pub preferred_model: Patch<ModelRefDto>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    #[schemars(with = "Option<ApprovalReviewModelSelectionDto>")]
    #[ts(as = "Option<ApprovalReviewModelSelectionDto>", optional = nullable)]
    pub approval_review_model: Patch<ApprovalReviewModelSelectionDto>,
}

/// Creates or replaces one user-owned language-server preference.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageServerConfigureParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[schemars(length(min = 1))]
    pub server_id: String,
    pub config: LanguageServerConfigDto,
}

/// Removes an explicit language-server preference and restores the product default.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageServerRemoveParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[schemars(length(min = 1))]
    pub server_id: String,
}

/// Creates or replaces one provider entry in the user configuration authority.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigureParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub config: ProviderConfigDto,
}

/// Removes one provider entry from the user configuration authority.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRemoveParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[schemars(length(min = 1))]
    pub provider: String,
}

/// Creates or replaces one standalone MCP server declaration.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct McpServerUpsertParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub server: McpServerConfigDto,
}

/// Removes one standalone MCP server declaration.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct McpServerRemoveParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[schemars(length(min = 1))]
    pub server_id: String,
}

/// Changes desired enablement for one configured standalone MCP server.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct McpServerSetEnablementParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[schemars(length(min = 1))]
    pub server_id: String,
    pub enablement: McpServerEnablementDto,
}

/// Adds a user-owned, runtime-free Skill source declaration.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillSourceAddParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub source: SkillSourceConfigDto,
}

/// Removes one user-owned Skill source declaration.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillSourceRemoveParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[schemars(length(min = 1))]
    pub source_id: String,
}

/// Changes desired enablement for one configured user Skill source.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillSourceSetEnablementParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[schemars(length(min = 1))]
    pub source_id: String,
    pub enablement: SkillSourceEnablementDto,
}

/// Creates or replaces one exact user Plugin request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginRequestUpsertParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub request: PluginRequestDto,
}

/// Removes one user Plugin request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginRequestRemoveParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[schemars(length(min = 1))]
    pub plugin_id: String,
}

/// Changes desired enablement for one configured Plugin request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginRequestSetEnablementParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[schemars(length(min = 1))]
    pub plugin_id: String,
    pub enablement: PluginRequestEnablementDto,
}

/// Creates or replaces one declarative user Hook.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HookUpsertParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub hook: HookConfigDto,
}

/// Removes one declarative user Hook.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HookRemoveParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[schemars(length(min = 1))]
    pub hook_id: String,
}

/// Changes desired enablement for one configured user Hook.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HookSetEnablementParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[schemars(length(min = 1))]
    pub hook_id: String,
    pub enablement: HookEnablementDto,
}
