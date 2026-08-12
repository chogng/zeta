use std::sync::Arc;

use zeta_api::SemanticApiEndpoint;
use zeta_client::OperationClient;
use zeta_client::ResolvedApiTarget;
use zeta_http_client::HttpHeader;
use zeta_model_provider_config::ProviderAdapter;
use zeta_model_provider_config::ProviderConfigRegistry;
use zeta_secrets::SecretKey;
use zeta_secrets::SecretStore;

use crate::EmbeddingInvoker;
use crate::EmbeddingRequest;
use crate::EmbeddingResponse;
use crate::EmbeddingRuntimeRequest;
use crate::EmbeddingVector;
use crate::ModelProviderError;
use crate::RerankInvoker;
use crate::RerankRequest;
use crate::RerankResponse;
use crate::RerankRuntimeRequest;
use crate::SemanticModelProvider;

const OPENAI_API_KEY_SECRET: &str = "provider/openai/default/api-key";

pub(crate) struct SemanticRuntimeResolver {
    pub(crate) configs: ProviderConfigRegistry,
    pub(crate) client: Arc<dyn OperationClient>,
    pub(crate) secrets: Arc<dyn SecretStore>,
}

impl SemanticModelProvider for SemanticRuntimeResolver {
    fn embedding_runtime(
        &self,
        request: EmbeddingRuntimeRequest,
    ) -> Result<Arc<dyn EmbeddingInvoker>, ModelProviderError> {
        let runtime = self.resolve(
            &request.model.provider,
            &request.config,
            SemanticOperation::Embedding,
        )?;
        Ok(Arc::new(ProviderEmbeddingInvoker {
            runtime,
            model: request.model.model.to_string(),
        }))
    }

    fn rerank_runtime(
        &self,
        request: RerankRuntimeRequest,
    ) -> Result<Arc<dyn RerankInvoker>, ModelProviderError> {
        let runtime = self.resolve(
            &request.model.provider,
            &request.config,
            SemanticOperation::Rerank,
        )?;
        Ok(Arc::new(ProviderRerankInvoker {
            runtime,
            model: request.model.model.to_string(),
        }))
    }
}

impl SemanticRuntimeResolver {
    fn resolve(
        &self,
        provider: &zeta_protocol::ProviderId,
        config: &zeta_model_provider_config::ModelProviderConfig,
        operation: SemanticOperation,
    ) -> Result<SemanticHttpRuntime, ModelProviderError> {
        let normalized = self.configs.normalize_for(config, provider)?;
        let definition = self
            .configs
            .get(provider)
            .expect("normalization only succeeds for registered providers");
        let supported = match operation {
            SemanticOperation::Embedding => matches!(
                definition.adapter,
                ProviderAdapter::OpenAi
                    | ProviderAdapter::OpenAiCompatible
                    | ProviderAdapter::Ollama
            ),
            SemanticOperation::Rerank => definition.adapter == ProviderAdapter::OpenAiCompatible,
        };
        if !supported {
            return Err(ModelProviderError::Unavailable(format!(
                "provider '{provider}' does not expose the configured {} endpoint",
                operation.label()
            )));
        }
        let headers = credential_headers(definition.adapter, self.secrets.as_ref())?;
        Ok(SemanticHttpRuntime {
            target: ResolvedApiTarget::new(normalized.base_url, headers),
            endpoint: SemanticApiEndpoint::OpenAiCompatible,
            client: Arc::clone(&self.client),
        })
    }
}

#[derive(Clone, Copy)]
enum SemanticOperation {
    Embedding,
    Rerank,
}

impl SemanticOperation {
    fn label(self) -> &'static str {
        match self {
            Self::Embedding => "embedding",
            Self::Rerank => "rerank",
        }
    }
}

fn credential_headers(
    adapter: ProviderAdapter,
    secrets: &dyn SecretStore,
) -> Result<Vec<HttpHeader>, ModelProviderError> {
    let key = match adapter {
        ProviderAdapter::OpenAi => Some(OPENAI_API_KEY_SECRET),
        ProviderAdapter::OpenAiCompatible => None,
        ProviderAdapter::Ollama => None,
        _ => unreachable!("semantic provider support was checked before credential resolution"),
    };
    let Some(key) = key else {
        return Ok(Vec::new());
    };
    let key = SecretKey::new(key).expect("static provider secret key is valid");
    let secret = secrets.load(&key)?.ok_or_else(|| {
        ModelProviderError::Credential(format!("no API key is stored for {key:?}"))
    })?;
    let value = std::str::from_utf8(secret.expose())
        .map_err(|_| ModelProviderError::Credential("stored API key is not UTF-8".into()))?;
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(ModelProviderError::Credential(
            "stored API key is invalid".into(),
        ));
    }
    Ok(vec![HttpHeader::new(
        "Authorization",
        format!("Bearer {value}"),
    )])
}

#[derive(Clone)]
struct SemanticHttpRuntime {
    target: ResolvedApiTarget,
    endpoint: SemanticApiEndpoint,
    client: Arc<dyn OperationClient>,
}

struct ProviderEmbeddingInvoker {
    runtime: SemanticHttpRuntime,
    model: String,
}

impl EmbeddingInvoker for ProviderEmbeddingInvoker {
    fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, ModelProviderError> {
        self.embed_with_cancellation(
            request,
            &zeta_async_utils::CancellationSource::new().token(),
        )
    }

    fn embed_with_cancellation(
        &self,
        request: &EmbeddingRequest,
        cancellation: &zeta_async_utils::CancellationToken,
    ) -> Result<EmbeddingResponse, ModelProviderError> {
        self.runtime
            .endpoint
            .embed_with_client_and_cancellation(
                &self.runtime.target,
                &self.model,
                request.inputs(),
                self.runtime.client.as_ref(),
                cancellation,
            )?
            .into_iter()
            .map(EmbeddingVector::new)
            .collect::<Result<Vec<_>, _>>()
            .and_then(EmbeddingResponse::new)
    }
}

struct ProviderRerankInvoker {
    runtime: SemanticHttpRuntime,
    model: String,
}

impl RerankInvoker for ProviderRerankInvoker {
    fn rerank(&self, request: &RerankRequest) -> Result<RerankResponse, ModelProviderError> {
        self.rerank_with_cancellation(
            request,
            &zeta_async_utils::CancellationSource::new().token(),
        )
    }

    fn rerank_with_cancellation(
        &self,
        request: &RerankRequest,
        cancellation: &zeta_async_utils::CancellationToken,
    ) -> Result<RerankResponse, ModelProviderError> {
        self.runtime
            .endpoint
            .rerank_with_client_and_cancellation(
                &self.runtime.target,
                &self.model,
                request.query(),
                request.documents(),
                self.runtime.client.as_ref(),
                cancellation,
            )
            .map_err(Into::into)
            .and_then(RerankResponse::new)
    }
}
