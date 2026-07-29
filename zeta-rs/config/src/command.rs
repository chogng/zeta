use crate::{
    ApprovalReviewModelSelection, ConfigGeneration, ConfigRevision, McpServerConfig,
    McpServerEnablement, McpServerId, ModelRef, SkillEnablement, SkillId, SkillSourceConfig,
    SkillSourceEnablement, SkillSourceId, Theme,
};
use serde::{Deserialize, Serialize};
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::{CommandId, Patch, ProviderId};

/// A three-state update for user-facing preferences.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesUpdate {
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub preferred_model: Patch<ModelRef>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub approval_review_model: Patch<ApprovalReviewModelSelection>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub theme: Patch<Theme>,
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
