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
    let config = ModelProviderConfig::new(ProviderId::new("anthropic").unwrap());

    assert!(runtime.catalog_binding(&config).unwrap().is_none());
}

#[test]
fn openai_catalog_handles_empty_lists_invalid_payloads_and_http_errors() {
    struct Client { responses: Mutex<std::collections::VecDeque<ClientResponse>> }
    impl OperationClient for Client {
        fn execute(&self, _: &ClientRequest) -> Result<ClientResponse, ClientError> {
            Ok(self.responses.lock().unwrap().pop_front().unwrap())
        }
    }
    let client = Arc::new(Client { responses: Mutex::new(std::collections::VecDeque::from([
        ClientResponse::new(200, vec![], br#"{"data":[{"id":"example-model"}]}"#.to_vec()),
        ClientResponse::new(200, vec![], br#"{"data":[]}"#.to_vec()),
        ClientResponse::new(200, vec![], br#"{"wrong":[]}"#.to_vec()),
        ClientResponse::new(401, vec![], b"sensitive-response-body".to_vec()),
    ])) });
    let runtime = crate::ModelProviderRuntime::builtin_with_client(client);
    let binding = runtime.catalog_binding(&ModelProviderConfig::new(ProviderId::new("openai").unwrap())).unwrap().unwrap();
    let executor = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let manager = runtime.models_manager();
    let first = executor.block_on(manager.refresh(binding.scope().clone(), binding.source())).unwrap();
    assert_eq!(first.entries().iter().filter(|entry| entry.availability() == zeta_protocol::ModelAvailability::Available).count(), 1);
    let empty = executor.block_on(manager.refresh(binding.scope().clone(), binding.source())).unwrap();
    assert!(empty.entries().iter().all(|entry| entry.availability() != zeta_protocol::ModelAvailability::Available));
    assert!(executor.block_on(manager.refresh(binding.scope().clone(), binding.source())).is_err());
    let error = executor.block_on(manager.refresh(binding.scope().clone(), binding.source())).unwrap_err().to_string();
    assert!(error.contains("401"));
    assert!(!error.contains("sensitive-response-body"));
}

#[test]
fn custom_catalog_fetches_models_with_its_own_key_and_invalidates_scope() {
    use crate::ProviderCredentialService;
    use zeta_model_provider_config::CustomProviderConfig;
    use zeta_model_provider_config::CustomProviderProtocol;
    struct Client {
        requests: Mutex<Vec<ClientRequest>>,
        body: Vec<u8>,
    }
    impl OperationClient for Client {
        fn execute(&self, request: &ClientRequest) -> Result<ClientResponse, ClientError> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(ClientResponse::new(200, vec![], self.body.clone()))
        }
    }
    let secrets = Arc::new(zeta_secrets::MemorySecretStore::default());
    let client = Arc::new(Client {
        requests: Mutex::new(vec![]),
        body: br#"{"data":[{"id":"one"},{"id":"two"},{"id":"one"}]}"#.to_vec(),
    });
    let mut config = ModelProviderConfig::new(ProviderId::new("custom-test").unwrap());
    config.base_url = Some("https://example.test/v1/".into());
    config.custom = Some(CustomProviderConfig {
        name: "Example".into(),
        protocol: CustomProviderProtocol::Responses,
    });
    let registry = ProviderConfigRegistry::builtin()
        .with_configs([&config])
        .unwrap();
    let credentials = ProviderCredentialService::new(registry.clone(), secrets.clone());
    credentials
        .set_api_key(&config.provider, b"custom-key".to_vec())
        .unwrap();
    let runtime = crate::ModelProviderRuntime::with_client_and_secrets(
        ProviderConfigRegistry::builtin(),
        client.clone(),
        secrets,
    );
    let binding = runtime.catalog_binding(&config).unwrap().unwrap();
    assert!(client.requests.lock().unwrap().is_empty());
    let manager = runtime.models_manager().with_registry(registry);
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(manager.refresh(binding.scope().clone(), binding.source()))
        .unwrap();
    let entries = manager
        .list(&[binding.scope().clone()], &CatalogQuery::all())
        .unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.model().model.as_str())
            .collect::<Vec<_>>(),
        vec!["one", "two"]
    );
    let requests = client.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url(), "https://example.test/v1/models");
    assert_eq!(requests[0].method(), zeta_http_client::HttpMethod::Get);
    assert_eq!(
        requests[0].headers(),
        &[zeta_http_client::HttpHeader::new(
            "Authorization",
            "Bearer custom-key"
        )]
    );
    drop(requests);
    credentials
        .set_api_key(&config.provider, b"rotated-key".to_vec())
        .unwrap();
    let changed_key = runtime.catalog_binding(&config).unwrap().unwrap();
    assert_ne!(binding.scope(), changed_key.scope());
    config.custom.as_mut().unwrap().protocol = CustomProviderProtocol::ChatCompletions;
    let changed_protocol = runtime.catalog_binding(&config).unwrap().unwrap();
    assert_ne!(changed_key.scope(), changed_protocol.scope());
    config.base_url = Some("https://other.test/v1".into());
    assert_ne!(
        changed_protocol.scope(),
        runtime.catalog_binding(&config).unwrap().unwrap().scope()
    );
}
