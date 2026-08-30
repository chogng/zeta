use std::sync::Arc;
use zeta_app_server_protocol::protocol::model::ModelCatalogEntry;
use zeta_core::CoreError;
use zeta_protocol::ModelRef;

/// Supplies the product model catalog and configured default.
///
/// Catalog membership is presentation metadata, not evidence that a remote invocation will succeed.
/// Runtime configuration, authentication, entitlement, rate limits, and transport are checked by the
/// selected Turn backend and become errors on that Turn.
pub(crate) trait ModelCatalog: Send + Sync {
    fn list(&self) -> Result<Vec<ModelCatalogEntry>, CoreError>;
    fn configured_default(&self) -> Result<Option<ModelRef>, CoreError>;
}

pub(crate) struct UnavailableModelCatalog;

impl ModelCatalog for UnavailableModelCatalog {
    fn list(&self) -> Result<Vec<ModelCatalogEntry>, CoreError> {
        Ok(Vec::new())
    }

    fn configured_default(&self) -> Result<Option<ModelRef>, CoreError> {
        Ok(None)
    }
}

pub(crate) fn unavailable_model_catalog() -> Arc<dyn ModelCatalog> {
    Arc::new(UnavailableModelCatalog)
}
