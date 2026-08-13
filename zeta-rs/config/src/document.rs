use crate::ConfigDiagnostic;
use crate::ConfigError;
use crate::ConfigProvenance;
use crate::HooksConfig;
use crate::LanguageServersConfig;
use crate::McpConfig;
use crate::PluginsConfig;
use crate::SemanticCodeIndexConfig;
use crate::SkillsConfig;
use crate::ToolSearchConfig;
use crate::UserExecPolicyConfig;
use crate::WorkspaceConfigIntent;
use crate::WorkspaceTrustConfig;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_model_provider_config::ProviderConfigError;
use zeta_model_provider_config::ProviderConfigRegistry;
use zeta_protocol::ModelRef;
use zeta_protocol::ProviderId;

/// User-selected model source for automatic approval review.
///
/// `Automatic` follows the active Agent model's provider and delegates the exact model choice to
/// that provider's definition. `Explicit` binds review to one configured provider/model without
/// changing the main Agent model.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum ApprovalReviewModelSelection {
    #[default]
    Automatic,
    Explicit {
        model: ModelRef,
    },
}

impl ApprovalReviewModelSelection {
    pub fn explicit_model(&self) -> Option<&ModelRef> {
        match self {
            Self::Automatic => None,
            Self::Explicit { model } => Some(model),
        }
    }
}

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

/// Agent defaults that may be resolved into future model invocations.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_model: Option<ModelRef>,
    #[serde(default)]
    pub approval_review_model: ApprovalReviewModelSelection,
}

/// Durable, non-secret user intent for ordinary Zeta configuration.
///
/// Provider entries are keyed by `ProviderId`; each value repeats its provider identity so a
/// serialized document remains self-describing and mismatched entries can be rejected.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserConfigDocument {
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub providers: BTreeMap<ProviderId, ModelProviderConfig>,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub skills: SkillsConfig,
    #[serde(default)]
    pub plugins: PluginsConfig,
    #[serde(default)]
    pub hooks: HooksConfig,
    #[serde(default)]
    pub language_servers: LanguageServersConfig,
    #[serde(default)]
    pub tool_search: ToolSearchConfig,
    #[serde(default)]
    pub semantic_code_index: SemanticCodeIndexConfig,
    #[serde(default)]
    pub exec_policy: UserExecPolicyConfig,
    #[serde(default)]
    pub workspace_trust: WorkspaceTrustConfig,
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
        if let Some(model) = self.agent.approval_review_model.explicit_model()
            && !self.providers.contains_key(&model.provider)
        {
            return Err(ConfigError(format!(
                "approval review model provider '{}' is not configured",
                model.provider
            )));
        }
        if let Some(models) = self.semantic_code_index.selection.remote_models() {
            for (role, model) in [
                ("embedding", &models.embedding_model),
                (
                    "rerank",
                    models
                        .rerank_model
                        .as_ref()
                        .unwrap_or(&models.embedding_model),
                ),
            ] {
                if role == "rerank" && models.rerank_model.is_none() {
                    continue;
                }
                if !self.providers.contains_key(&model.provider) {
                    return Err(ConfigError(format!(
                        "semantic code-index {role} provider '{}' is not configured",
                        model.provider
                    )));
                }
            }
        }
        if self.tool_search.mode == crate::ToolSearchModeConfig::HybridEmbedding
            && self.tool_search.embedding_model.is_none()
        {
            return Err(ConfigError(
                "hybrid embedding Tool Search requires an embedding model".into(),
            ));
        }
        if let Some(model) = &self.tool_search.embedding_model
            && !self.providers.contains_key(&model.provider)
        {
            return Err(ConfigError(format!(
                "Tool Search embedding provider '{}' is not configured",
                model.provider
            )));
        }
        self.mcp.validate_for_namespace("user")?;
        self.skills.validate_for_namespace("user")?;
        self.plugins.validate()?;
        self.hooks.validate_for_namespace("user")?;
        self.language_servers.validate()?;
        self.exec_policy.validate()?;
        Ok(())
    }
}

/// Effective configuration derived from the currently supported user configuration sources.
///
/// Additional sources such as Workspace documents and session defaults will be resolved into this
/// type without exposing file or authority implementation details to runtime consumers.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResolvedConfig {
    pub preferred_model: Option<ModelRef>,
    pub approval_review_model: ApprovalReviewModelSelection,
    pub providers: BTreeMap<ProviderId, ModelProviderConfig>,
    pub mcp: McpConfig,
    pub skills: SkillsConfig,
    pub plugins: PluginsConfig,
    pub hooks: HooksConfig,
    pub language_servers: LanguageServersConfig,
    pub tool_search: ToolSearchConfig,
    pub semantic_code_index: SemanticCodeIndexConfig,
    pub exec_policy: UserExecPolicyConfig,
    pub workspace_trust: WorkspaceTrustConfig,
    pub workspace: Option<WorkspaceConfigIntent>,
}

impl ResolvedConfig {
    pub fn selected_provider(&self) -> Option<&ModelProviderConfig> {
        self.preferred_model
            .as_ref()
            .and_then(|model| self.providers.get(&model.provider))
    }

    pub fn selected_approval_review_provider(&self) -> Option<&ModelProviderConfig> {
        match &self.approval_review_model {
            ApprovalReviewModelSelection::Automatic => self.selected_provider(),
            ApprovalReviewModelSelection::Explicit { model } => self.providers.get(&model.provider),
        }
    }

    /// Resolves and preflights the review model selected for the next approval assessment.
    ///
    /// Automatic selection follows the active Agent provider. Explicit selection remains fixed.
    /// This validates local provider configuration and static catalog availability; credentials,
    /// subscription entitlement, and remote availability are validated by the runtime invocation.
    pub fn resolve_approval_review_model(
        &self,
        registry: &ProviderConfigRegistry,
    ) -> Result<ModelRef, ConfigError> {
        let model = match &self.approval_review_model {
            ApprovalReviewModelSelection::Automatic => {
                let active_model = self.preferred_model.as_ref().ok_or_else(|| {
                    ConfigError(
                        "automatic approval review requires a configured preferred model".into(),
                    )
                })?;
                registry
                    .automatic_approval_review_model(active_model)
                    .map_err(provider_config_error)?
            }
            ApprovalReviewModelSelection::Explicit { model } => model.clone(),
        };
        let provider = self.providers.get(&model.provider).ok_or_else(|| {
            ConfigError(format!(
                "approval review model provider '{}' is not configured",
                model.provider
            ))
        })?;
        registry
            .normalize_for(provider, &model.provider)
            .map_err(provider_config_error)?;
        registry
            .validate_model_selection(&model)
            .map_err(provider_config_error)?;
        Ok(model)
    }
}

fn provider_config_error(error: ProviderConfigError) -> ConfigError {
    ConfigError(error.to_string())
}

impl From<&UserConfigDocument> for ResolvedConfig {
    fn from(document: &UserConfigDocument) -> Self {
        Self {
            preferred_model: document.agent.preferred_model.clone(),
            approval_review_model: document.agent.approval_review_model.clone(),
            providers: document.providers.clone(),
            mcp: document.mcp.clone(),
            skills: document.skills.clone(),
            plugins: document.plugins.clone(),
            hooks: document.hooks.clone(),
            language_servers: document.language_servers.clone(),
            tool_search: document.tool_search.clone(),
            semantic_code_index: document.semantic_code_index.clone(),
            exec_policy: document.exec_policy.clone(),
            workspace_trust: document.workspace_trust.clone(),
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
