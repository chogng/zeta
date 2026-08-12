use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::ModelRef;
use zeta_protocol::ProviderId;
use zeta_workspace::WorkspaceTrustId;

/// User-selected models for local semantic code-index orchestration.
///
/// Zeta sends bounded code chunks and retrieval queries to these model endpoints, while chunking,
/// vector persistence, recall, fusion, and final ordering remain local responsibilities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticCodeIndexModelSelection {
    pub embedding_model: ModelRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rerank_model: Option<ModelRef>,
}

/// Whether user-configured semantic model invocation participates in code retrieval.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum SemanticCodeIndexSelection {
    #[default]
    Disabled,
    Remote {
        models: SemanticCodeIndexModelSelection,
    },
}

/// Whether code retrieval is automatically added to the first model invocation of a Turn.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticCodeIndexAutomaticContext {
    #[default]
    Off,
    FirstInvocation,
}

impl SemanticCodeIndexSelection {
    pub fn remote_models(&self) -> Option<&SemanticCodeIndexModelSelection> {
        match self {
            Self::Disabled => None,
            Self::Remote { models } => Some(models),
        }
    }
}

/// Exact model selection approved to receive source-derived text for one canonical Workspace.
///
/// Copying the selection into the grant makes authorization fail closed after either configured
/// model changes. Raw code, endpoint credentials, and provider responses are never persisted here.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticCodeIndexEgressGrant {
    pub models: SemanticCodeIndexModelSelection,
    pub providers: BTreeMap<ProviderId, ModelProviderConfig>,
}

/// Durable semantic code-index preference and Workspace-scoped source-egress grants.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticCodeIndexConfig {
    #[serde(default)]
    pub selection: SemanticCodeIndexSelection,
    #[serde(default)]
    pub automatic_context: SemanticCodeIndexAutomaticContext,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub source_egress_grants: BTreeMap<WorkspaceTrustId, SemanticCodeIndexEgressGrant>,
}

impl SemanticCodeIndexConfig {
    /// Returns the current model selection only when this exact Workspace grant still matches it.
    pub fn authorized_remote_models(
        &self,
        workspace: &WorkspaceTrustId,
        providers: &BTreeMap<ProviderId, ModelProviderConfig>,
    ) -> Option<&SemanticCodeIndexModelSelection> {
        let models = self.selection.remote_models()?;
        self.source_egress_grants
            .get(workspace)
            .filter(|grant| {
                grant.models == *models
                    && grant.providers == selected_provider_configs(models, providers)
            })
            .map(|_| models)
    }

    pub(crate) fn authorize(
        &mut self,
        workspace: WorkspaceTrustId,
        providers: &BTreeMap<ProviderId, ModelProviderConfig>,
    ) -> Result<(), &'static str> {
        let models = self
            .selection
            .remote_models()
            .cloned()
            .ok_or("remote semantic code indexing is not configured")?;
        let selected = selected_provider_configs(&models, providers);
        if selected.len()
            != usize::from(
                models
                    .rerank_model
                    .as_ref()
                    .is_some_and(|rerank| rerank.provider != models.embedding_model.provider),
            ) + 1
        {
            return Err("semantic code-index model provider is not configured");
        }
        self.source_egress_grants.insert(
            workspace,
            SemanticCodeIndexEgressGrant {
                models,
                providers: selected,
            },
        );
        Ok(())
    }

    pub(crate) fn revoke(&mut self, workspace: &WorkspaceTrustId) {
        self.source_egress_grants.remove(workspace);
    }

    pub(crate) fn replace_selection(&mut self, selection: SemanticCodeIndexSelection) {
        if self.selection != selection {
            self.selection = selection;
            self.source_egress_grants.clear();
        }
    }

    pub(crate) fn replace_automatic_context(
        &mut self,
        automatic_context: SemanticCodeIndexAutomaticContext,
    ) {
        self.automatic_context = automatic_context;
    }
}

fn selected_provider_configs(
    models: &SemanticCodeIndexModelSelection,
    providers: &BTreeMap<ProviderId, ModelProviderConfig>,
) -> BTreeMap<ProviderId, ModelProviderConfig> {
    let mut selected = BTreeMap::new();
    if let Some(config) = providers.get(&models.embedding_model.provider) {
        selected.insert(models.embedding_model.provider.clone(), config.clone());
    }
    if let Some(rerank) = &models.rerank_model
        && let Some(config) = providers.get(&rerank.provider)
    {
        selected.insert(rerank.provider.clone(), config.clone());
    }
    selected
}
