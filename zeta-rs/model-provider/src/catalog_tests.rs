use super::*;
use std::sync::Mutex;
use zeta_client::ClientError;
use zeta_client::ClientRequest;
use zeta_client::ClientResponse;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_model_provider_config::ProviderConfigRegistry;
use zeta_models_manager::CatalogQuery;
use zeta_models_manager::CatalogReadPolicy;
use zeta_models_manager::CatalogReadSource;

struct CatalogClient {
    request: Mutex<Option<ClientRequest>>,
}

impl zeta_client::OperationClient for CatalogClient {
    fn execute(&self, request: &ClientRequest) -> Result<ClientResponse, ClientError> {
        *self.request.lock().unwrap() = Some(request.clone());
        let body = if request.url().ends_with("/api/tags") {
            br#"{"models":[{"name":"qwen3:8b"},{"name":"nomic-embed-text"}]}"#.to_vec()
        } else if request
            .body()
            .windows("nomic-embed-text".len())
            .any(|value| value == b"nomic-embed-text")
        {
            br#"{"capabilities":["embedding"]}"#.to_vec()
        } else {
            br#"{"capabilities":["completion","tools"]}"#.to_vec()
        };
        Ok(ClientResponse::new(200, Vec::new(), body))
    }
}

#[test]
fn ollama_catalog_uses_shared_client_and_adds_installed_models() {
    let client = Arc::new(CatalogClient {
        request: Mutex::new(None),
    });
    let runtime =
        crate::ModelProviderRuntime::with_client(ProviderConfigRegistry::builtin(), client.clone());
    let config = ModelProviderConfig::new(ProviderId::new("ollama").unwrap());
    let binding = runtime.catalog_binding(&config).unwrap().unwrap();

    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(runtime.models_manager().read(
            binding.scope().clone(),
            CatalogReadPolicy::RequireFresh,
            CatalogReadSource::dynamic(binding.source()),
        ))
        .unwrap();

    let models = runtime
        .models_manager()
        .list(&[binding.scope().clone()], &CatalogQuery::all())
        .unwrap();
    assert_eq!(
        models
            .iter()
            .map(|entry| entry.model().model.as_str())
            .collect::<Vec<_>>(),
        vec!["qwen3:8b"]
    );
    assert_eq!(
        client.request.lock().unwrap().as_ref().unwrap().url(),
        "http://localhost:11434/api/show"
    );
    assert_eq!(
        models[0].info().capabilities.tools,
        CapabilitySupport::Supported
    );
}

#[test]
fn providers_without_dynamic_discovery_return_no_binding() {
    let runtime = crate::ModelProviderRuntime::builtin_with_client(Arc::new(CatalogClient {
        request: Mutex::new(None),
    }));
    let config = ModelProviderConfig::new(ProviderId::new("openai").unwrap());

    assert!(runtime.catalog_binding(&config).unwrap().is_none());
}
