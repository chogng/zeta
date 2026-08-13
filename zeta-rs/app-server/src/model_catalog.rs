use std::sync::Arc;
use zeta_app_server_protocol::protocol::model::ModelCatalogEntry;
use zeta_core::CoreError;
use zeta_protocol::ModelRef;

/// Supplies the configured model catalog and validates Session-owned selections.
///
/// Implementations must return only models whose provider configuration is usable by the same
/// App Server model runtime. Remote entitlement remains an invocation-time boundary.
pub(crate) trait ModelCatalog: Send + Sync {
    fn list(&self) -> Result<Vec<ModelCatalogEntry>, CoreError>;
    fn configured_default(&self) -> Result<Option<ModelRef>, CoreError>;
    fn validate(&self, model: &ModelRef) -> Result<(), CoreError>;
}

pub(crate) struct CombinedModelCatalog {
    direct: Arc<dyn ModelCatalog>,
    codex: zeta_codex_app_server::CodexModelCatalog,
    codex_provider: zeta_protocol::ProviderId,
}

impl CombinedModelCatalog {
    pub(crate) fn new(
        direct: Arc<dyn ModelCatalog>,
        codex: zeta_codex_app_server::CodexModelCatalog,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            direct,
            codex,
            codex_provider: zeta_protocol::ProviderId::new(
                zeta_codex_app_server::CODEX_SUBSCRIPTION_PROVIDER_ID,
            )
            .map_err(|error| CoreError::Model(error.to_string()))?,
        })
    }

    fn codex_entries(&self) -> Result<Vec<ModelCatalogEntry>, CoreError> {
        self.codex
            .list()
            .map_err(|error| CoreError::Model(error.to_string()))?
            .into_iter()
            .map(|model| {
                Ok(ModelCatalogEntry {
                    model: ModelRef::new(
                        self.codex_provider.clone(),
                        zeta_protocol::ModelId::new(model.id)
                            .map_err(|error| CoreError::Model(error.to_string()))?,
                    ),
                    display_name: model.display_name,
                })
            })
            .collect()
    }
}

impl ModelCatalog for CombinedModelCatalog {
    fn list(&self) -> Result<Vec<ModelCatalogEntry>, CoreError> {
        let mut models = self.direct.list()?;
        models.extend(self.codex_entries().unwrap_or_default());
        models.sort_by(|left, right| {
            left.model
                .provider
                .cmp(&right.model.provider)
                .then_with(|| left.model.model.cmp(&right.model.model))
        });
        Ok(models)
    }

    fn configured_default(&self) -> Result<Option<ModelRef>, CoreError> {
        self.direct.configured_default()
    }

    fn validate(&self, model: &ModelRef) -> Result<(), CoreError> {
        if model.provider != self.codex_provider {
            return self.direct.validate(model);
        }
        self.codex_entries()?
            .iter()
            .any(|entry| &entry.model == model)
            .then_some(())
            .ok_or_else(|| CoreError::Model("Codex subscription model is not available".into()))
    }
}

pub(crate) struct UnavailableModelCatalog;

impl ModelCatalog for UnavailableModelCatalog {
    fn list(&self) -> Result<Vec<ModelCatalogEntry>, CoreError> {
        Ok(Vec::new())
    }

    fn configured_default(&self) -> Result<Option<ModelRef>, CoreError> {
        Ok(None)
    }

    fn validate(&self, _: &ModelRef) -> Result<(), CoreError> {
        Err(CoreError::Model("model catalog is unavailable".into()))
    }
}

pub(crate) fn unavailable_model_catalog() -> Arc<dyn ModelCatalog> {
    Arc::new(UnavailableModelCatalog)
}
