use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::ModelRef;
use zeta_workspace::WorkspaceTrustId;

/// Exact model and endpoint configuration authorized to receive one Workspace's source text.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitMessageEgressGrant {
    pub model: ModelRef,
    pub provider: ModelProviderConfig,
}

/// Workspace-scoped authorization for automatic commit-message generation.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitMessageConfig {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub source_egress_grants: BTreeMap<WorkspaceTrustId, CommitMessageEgressGrant>,
}

impl CommitMessageConfig {
    /// Returns the selected model only when the Workspace grant still matches model and endpoint.
    pub fn authorized_model<'a>(
        &self,
        workspace: &WorkspaceTrustId,
        model: Option<&'a ModelRef>,
        providers: &BTreeMap<zeta_protocol::ProviderId, ModelProviderConfig>,
    ) -> Option<&'a ModelRef> {
        let model = model?;
        let provider = providers.get(&model.provider)?;
        self.source_egress_grants
            .get(workspace)
            .filter(|grant| grant.model == *model && grant.provider == *provider)
            .map(|_| model)
    }

    pub(crate) fn authorize(
        &mut self,
        workspace: WorkspaceTrustId,
        model: Option<&ModelRef>,
        providers: &BTreeMap<zeta_protocol::ProviderId, ModelProviderConfig>,
    ) -> Result<(), &'static str> {
        let model = model
            .cloned()
            .ok_or("commit-message model is not configured")?;
        let provider = providers
            .get(&model.provider)
            .cloned()
            .ok_or("commit-message model provider is not configured")?;
        self.source_egress_grants
            .insert(workspace, CommitMessageEgressGrant { model, provider });
        Ok(())
    }

    pub(crate) fn revoke(&mut self, workspace: &WorkspaceTrustId) {
        self.source_egress_grants.remove(workspace);
    }

    pub(crate) fn revoke_all(&mut self) {
        self.source_egress_grants.clear();
    }
}
