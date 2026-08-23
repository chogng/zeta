use std::sync::Arc;
use zeta_app_server_protocol::protocol::model::ModelCatalogEntry;
use zeta_core::CoreError;
use zeta_model_provider_config::STATIC_MODEL_CATALOG;
use zeta_model_provider_config::find_static_model;
use zeta_protocol::ModelAccess;
use zeta_protocol::ModelOutputTransport;
use zeta_protocol::ModelRef;

/// Supplies the product model catalog and validates Session-owned identities.
///
/// Catalog membership is presentation metadata, not evidence that a remote invocation will succeed.
/// Runtime configuration, authentication, entitlement, rate limits, and transport are checked by the
/// selected Turn backend and become errors on that Turn.
pub(crate) trait ModelCatalog: Send + Sync {
    fn list(&self) -> Result<Vec<ModelCatalogEntry>, CoreError>;
    fn configured_default(&self) -> Result<Option<ModelRef>, CoreError>;
    fn validate(&self, model: &ModelRef) -> Result<(), CoreError>;
}

pub(crate) struct CombinedModelCatalog {
    direct: Arc<dyn ModelCatalog>,
}

impl CombinedModelCatalog {
    pub(crate) fn new(direct: Arc<dyn ModelCatalog>) -> Self {
        Self { direct }
    }

    fn subscription_entries(&self) -> Vec<ModelCatalogEntry> {
        STATIC_MODEL_CATALOG
            .iter()
            .filter(|model| model.access == ModelAccess::Subscription)
            .map(|model| {
                let info = model.model();
                ModelCatalogEntry::from_info(
                    model.model_ref(),
                    &info,
                    ModelOutputTransport::NativeStreaming,
                )
            })
            .collect()
    }
}

impl ModelCatalog for CombinedModelCatalog {
    fn list(&self) -> Result<Vec<ModelCatalogEntry>, CoreError> {
        let mut models = self.direct.list()?;
        models.extend(self.subscription_entries());
        models.sort_by(|left, right| {
            left.model
                .provider
                .cmp(&right.model.provider)
                .then_with(|| static_model_rank(&left.model).cmp(&static_model_rank(&right.model)))
                .then_with(|| left.model.model.cmp(&right.model.model))
        });
        Ok(models)
    }

    fn configured_default(&self) -> Result<Option<ModelRef>, CoreError> {
        self.direct.configured_default()
    }

    fn validate(&self, model: &ModelRef) -> Result<(), CoreError> {
        if find_static_model(model).is_some_and(|entry| entry.access == ModelAccess::Subscription) {
            return Ok(());
        }
        self.direct.validate(model)
    }
}

fn static_model_rank(model: &ModelRef) -> usize {
    STATIC_MODEL_CATALOG
        .iter()
        .position(|candidate| {
            candidate.provider_id == model.provider.as_str()
                && candidate.model_id == model.model.as_str()
        })
        .unwrap_or(usize::MAX)
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
