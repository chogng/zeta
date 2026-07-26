use crate::{
    ConfigDiagnostic, ConfigError, ConfigProvenance, McpConfig, SkillsConfig, WorkspaceConfigIntent,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::{ModelRef, ProviderId, Theme};

/// Monotonic revision of the user configuration authority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ConfigRevision(u64);

impl ConfigRevision {
    pub const INITIAL: Self = Self(0);

    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Consumer-visible generation of the resolved user configuration snapshot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ConfigGeneration(u64);

impl ConfigGeneration {
    pub const INITIAL: Self = Self(0);

    pub fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// User-interface preferences that are meaningful across Zeta clients.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
}

/// Agent defaults that may be resolved into future model invocations.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_model: Option<ModelRef>,
}

/// Durable, non-secret user intent for ordinary Zeta configuration.
///
/// Provider entries are keyed by `ProviderId`; each value repeats its provider identity so a
/// serialized document remains self-describing and mismatched entries can be rejected.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserConfigDocument {
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub providers: BTreeMap<ProviderId, ModelProviderConfig>,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub skills: SkillsConfig,
}

impl UserConfigDocument {
    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        for (provider_id, provider) in &self.providers {
            if provider.provider != *provider_id {
                return Err(ConfigError(format!(
                    "provider entry '{}' contains configuration for '{}'",
                    provider_id, provider.provider
                )));
            }
            provider
                .validate_static()
                .map_err(|error| ConfigError(error.to_string()))?;
        }
        if let Some(model) = &self.agent.preferred_model
            && !self.providers.contains_key(&model.provider)
        {
            return Err(ConfigError(format!(
                "preferred model provider '{}' is not configured",
                model.provider
            )));
        }
        self.mcp.validate_for_namespace("user")?;
        self.skills.validate_for_namespace("user")?;
        Ok(())
    }
}

/// Effective configuration derived from the currently supported user configuration sources.
///
/// Additional sources such as Workspace documents and session defaults will be resolved into this
/// type without exposing file or authority implementation details to runtime consumers.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResolvedConfig {
    pub theme: Option<Theme>,
    pub preferred_model: Option<ModelRef>,
    pub providers: BTreeMap<ProviderId, ModelProviderConfig>,
    pub mcp: McpConfig,
    pub skills: SkillsConfig,
    pub workspace: Option<WorkspaceConfigIntent>,
}

impl ResolvedConfig {
    pub fn selected_provider(&self) -> Option<&ModelProviderConfig> {
        self.preferred_model
            .as_ref()
            .and_then(|model| self.providers.get(&model.provider))
    }
}

impl From<&UserConfigDocument> for ResolvedConfig {
    fn from(document: &UserConfigDocument) -> Self {
        Self {
            theme: document.ui.theme,
            preferred_model: document.agent.preferred_model.clone(),
            providers: document.providers.clone(),
            mcp: document.mcp.clone(),
            skills: document.skills.clone(),
            workspace: None,
        }
    }
}

/// Immutable configuration input used by one runtime safe point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedConfigSnapshot {
    pub revision: ConfigRevision,
    pub generation: ConfigGeneration,
    pub values: ResolvedConfig,
    pub provenance: ConfigProvenance,
    pub diagnostics: Vec<ConfigDiagnostic>,
}

impl ResolvedConfigSnapshot {
    pub(crate) fn from_document(
        revision: ConfigRevision,
        generation: ConfigGeneration,
        document: &UserConfigDocument,
    ) -> Self {
        Self {
            revision,
            generation,
            values: ResolvedConfig::from(document),
            provenance: ConfigProvenance::from_user(document),
            diagnostics: Vec::new(),
        }
    }
}
