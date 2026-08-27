use crate::SemanticCodeIndexAutomaticContext;
use crate::SemanticCodeIndexSelection;
use crate::ToolSearchConfig;
use crate::{
    ApprovalReviewModelSelection, ConfigGeneration, ConfigRevision, HookConfig, HookEnablement,
    HookId, LanguageServerConfig, LanguageServerId, McpServerConfig, McpServerEnablement,
    McpServerId, ModelRef, PluginId, PluginRequest, PluginRequestEnablement, SkillEnablement,
    SkillId, SkillSourceConfig, SkillSourceEnablement, SkillSourceId, WorkspaceTrustSetting,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use zeta_execpolicy::ExecPolicyRule;
use zeta_execpolicy::ExecPolicyRuleId;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::CommandId;
use zeta_protocol::Patch;
use zeta_protocol::ProviderId;
use zeta_protocol::ToolMode;
use zeta_workspace::WorkspaceTrustId;

/// A three-state update for user-facing preferences.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesUpdate {
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub preferred_model: Patch<ModelRef>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub approval_review_model: Patch<ApprovalReviewModelSelection>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub tool_mode: Patch<ToolMode>,
}

/// Typed mutations accepted by the user configuration authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum UserConfigCommand {
    UpdatePreferences(PreferencesUpdate),
    ConfigureProvider {
        provider: ProviderId,
        config: ModelProviderConfig,
    },
    RemoveProvider {
        provider: ProviderId,
    },
    UpsertMcpServer {
        server: McpServerConfig,
    },
    RemoveMcpServer {
        server_id: McpServerId,
    },
    SetMcpServerEnablement {
        server_id: McpServerId,
        enablement: McpServerEnablement,
    },
    AddSkillSource {
        source: SkillSourceConfig,
    },
    RemoveSkillSource {
        source_id: SkillSourceId,
    },
    SetSkillSourceEnablement {
        source_id: SkillSourceId,
        enablement: SkillSourceEnablement,
    },
    SetSkillEnablement {
        skill_id: SkillId,
        enablement: SkillEnablement,
    },
    UpsertPluginRequest {
        request: PluginRequest,
    },
    RemovePluginRequest {
        plugin_id: PluginId,
    },
    SetPluginRequestEnablement {
        plugin_id: PluginId,
        enablement: PluginRequestEnablement,
    },
    UpsertHook {
        hook: HookConfig,
    },
    RemoveHook {
        hook_id: HookId,
    },
    SetHookEnablement {
        hook_id: HookId,
        enablement: HookEnablement,
    },
    ConfigureLanguageServer {
        server_id: LanguageServerId,
        config: LanguageServerConfig,
    },
    RemoveLanguageServerConfiguration {
        server_id: LanguageServerId,
    },
    ConfigureSemanticCodeIndex {
        selection: SemanticCodeIndexSelection,
        automatic_context: SemanticCodeIndexAutomaticContext,
    },
    ConfigureToolSearch {
        config: ToolSearchConfig,
    },
    AuthorizeSemanticCodeIndexEgress {
        workspace: WorkspaceTrustId,
    },
    RevokeSemanticCodeIndexEgress {
        workspace: WorkspaceTrustId,
    },
    SetWorkspaceTrust {
        workspace: WorkspaceTrustId,
        setting: WorkspaceTrustSetting,
        /// Optional canonical root retained as non-authoritative display metadata.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_root: Option<PathBuf>,
    },
    ForgetWorkspaceTrust {
        workspace: WorkspaceTrustId,
    },
    UpsertExecPolicyRule {
        rule: ExecPolicyRule,
    },
    RemoveExecPolicyRule {
        rule_id: ExecPolicyRuleId,
    },
}

/// Retry-safe request to mutate user configuration at one expected revision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigCommandRequest {
    pub command_id: CommandId,
    pub expected_revision: ConfigRevision,
    pub command: UserConfigCommand,
}

/// Whether a configuration command committed a new revision or replayed a prior receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigCommandDisposition {
    Updated,
    Replayed,
}

/// Compact result of a durable configuration command.
///
/// Callers read the corresponding `ResolvedConfigSnapshot` separately. This keeps command
/// receipts independent of the full configuration document while preserving exact replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfigCommandResult {
    pub revision: ConfigRevision,
    pub generation: ConfigGeneration,
    pub disposition: ConfigCommandDisposition,
}

/// Errors specific to retry-safe configuration command processing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigCommandError {
    Config(crate::ConfigError),
    CommandConflict,
    RevisionConflict {
        expected: ConfigRevision,
        actual: ConfigRevision,
    },
}

impl From<crate::ConfigError> for ConfigCommandError {
    fn from(error: crate::ConfigError) -> Self {
        Self::Config(error)
    }
}
