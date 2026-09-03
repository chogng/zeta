use sha2::Digest;
use sha2::Sha256;
use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use zeta_async_utils::CancellationSource;
use zeta_client::OperationClient;
use zeta_model_provider_config::ModelId;
use zeta_models_manager::CatalogCacheHint;
use zeta_models_manager::CatalogDiscoveryOutcome;
use zeta_models_manager::CatalogScopeKey;
use zeta_models_manager::CatalogSourceError;
use zeta_models_manager::CatalogSourceErrorKind;
use zeta_models_manager::CatalogSourceFuture;
use zeta_models_manager::CatalogSourceScopeId;
use zeta_models_manager::DiscoveredCatalog;
use zeta_models_manager::DiscoveredModel;
use zeta_models_manager::DiscoveryCoverage;
use zeta_models_manager::ModelCapabilitiesPatch;
use zeta_models_manager::ModelCatalogSource;
use zeta_models_manager::ModelMetadataPatch;
use zeta_ollama::OllamaClient;
use zeta_ollama::OllamaError;
use zeta_protocol::CapabilitySupport;
use zeta_protocol::ProviderId;

const OLLAMA_FRESH_FOR: Duration = Duration::from_secs(2);
const OLLAMA_STALE_USABLE_FOR: Duration = Duration::from_secs(30);

/// Binds one exact provider configuration to its dynamic catalog identity and source.
///
/// App Server composition uses this value to refresh the shared models manager without exposing
/// endpoint details or credentials through its client protocol.
pub struct ModelCatalogBinding {
    scope: CatalogScopeKey,
    source: Arc<dyn ModelCatalogSource>,
}

impl ModelCatalogBinding {
    pub fn scope(&self) -> &CatalogScopeKey {
        &self.scope
    }

    pub fn source(&self) -> Arc<dyn ModelCatalogSource> {
        Arc::clone(&self.source)
    }
}

pub(crate) fn ollama_catalog_binding(
    provider: ProviderId,
    base_url: &str,
    client: Arc<dyn OperationClient>,
) -> Result<ModelCatalogBinding, OllamaError> {
    let client = OllamaClient::from_openai_compatible_base_url(base_url, client)?;
    let scope = CatalogScopeKey::new(provider, catalog_scope(client.host_root())?);
    let source = Arc::new(OllamaCatalogSource {
        scope: scope.clone(),
        client,
    });
    Ok(ModelCatalogBinding { scope, source })
}

fn catalog_scope(host_root: &str) -> Result<CatalogSourceScopeId, OllamaError> {
    let digest = Sha256::digest(host_root.as_bytes());
    let mut value = String::with_capacity("ollama:".len() + digest.len() * 2);
    value.push_str("ollama:");
    for byte in digest {
        write!(&mut value, "{byte:02x}").expect("writing to a string cannot fail");
    }
    CatalogSourceScopeId::new(value)
        .map_err(|error| OllamaError::InvalidEndpoint(error.to_string()))
}

struct OllamaCatalogSource {
    scope: CatalogScopeKey,
    client: OllamaClient,
}

impl ModelCatalogSource for OllamaCatalogSource {
    fn discover<'a>(
        &'a self,
        request: zeta_models_manager::CatalogDiscoveryRequest,
    ) -> CatalogSourceFuture<'a> {
        if request.scope() != &self.scope {
            return Box::pin(async {
                Err(CatalogSourceError::new(
                    CatalogSourceErrorKind::InvalidPayload,
                    "Ollama catalog request scope does not match its source",
                ))
            });
        }
        let scope = self.scope.clone();
        let client = self.client.clone();
        let cancellation = CancellationSource::new();
        let token = cancellation.token();
        let task = tokio::task::spawn_blocking(move || {
            client
                .list_models(&token)?
                .into_iter()
                .try_fold(Vec::new(), |mut models, model| {
                    let info = client.show_model(&model.name, &token)?;
                    if info.supports("completion") != Some(false) {
                        models.push((model, info));
                    }
                    Ok::<_, OllamaError>(models)
                })
        });
        Box::pin(async move {
            let cancel_on_drop = cancellation.cancel_on_drop();
            let models = task
                .await
                .map_err(|_| {
                    CatalogSourceError::new(
                        CatalogSourceErrorKind::Transient,
                        "Ollama catalog worker stopped",
                    )
                })?
                .map_err(catalog_error)?;
            cancel_on_drop.disarm();
            let models = models
                .into_iter()
                .map(|(model, info)| {
                    let id = ModelId::new(model.name.clone()).map_err(|_| {
                        CatalogSourceError::new(
                            CatalogSourceErrorKind::InvalidPayload,
                            "Ollama returned an invalid model name",
                        )
                    })?;
                    Ok(DiscoveredModel::new(id).with_metadata(ModelMetadataPatch {
                        display_name: Some(model.name),
                        capabilities: ModelCapabilitiesPatch {
                            tools: info.supports("tools").map(capability_support),
                            reasoning: info.supports("thinking").map(capability_support),
                            ..ModelCapabilitiesPatch::default()
                        },
                        ..ModelMetadataPatch::default()
                    }))
                })
                .collect::<Result<Vec<_>, CatalogSourceError>>()?;
            Ok(CatalogDiscoveryOutcome::Modified(
                DiscoveredCatalog::new(scope, DiscoveryCoverage::Partial, SystemTime::now())
                    .with_models(models)
                    .with_cache_hint(
                        CatalogCacheHint::unspecified()
                            .with_fresh_for(OLLAMA_FRESH_FOR)
                            .with_stale_usable_for(OLLAMA_STALE_USABLE_FOR),
                    ),
            ))
        })
    }
}

fn capability_support(supported: bool) -> CapabilitySupport {
    if supported {
        CapabilitySupport::Supported
    } else {
        CapabilitySupport::Unsupported
    }
}

fn catalog_error(error: OllamaError) -> CatalogSourceError {
    let kind = if error.is_cancelled() {
        CatalogSourceErrorKind::Cancelled
    } else {
        match error.status() {
            Some(429) => CatalogSourceErrorKind::RateLimited,
            Some(404 | 405 | 501) => CatalogSourceErrorKind::Unsupported,
            _ => match error {
                OllamaError::InvalidEndpoint(_)
                | OllamaError::InvalidRequest(_)
                | OllamaError::InvalidResponse(_) => CatalogSourceErrorKind::InvalidPayload,
                _ => CatalogSourceErrorKind::Transient,
            },
        }
    };
    CatalogSourceError::new(kind, "Ollama model discovery failed")
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
