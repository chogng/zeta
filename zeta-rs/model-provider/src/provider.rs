use crate::providers::ProviderAdapter;
use crate::{ModelProviderError, providers};
use std::sync::Arc;
use zeta_api::{
    ApiProtocol, ContentPart, InputItem, ModelRequest, ModelResponse, OutputItem, StopReason,
};
use zeta_async_utils::{CancellationSource, CancellationToken};
use zeta_client::{OperationClient, ZetaClient};
use zeta_http_client::UreqHttpClient;
use zeta_model_provider_config::{
    Model, ModelCatalogPolicy, ModelId, ModelProviderConfig, NormalizedModelProviderConfig,
    ProviderConfigError, ProviderConfigRegistry, ProviderDefinition, ProviderId,
};
use zeta_protocol::ModelRef;

#[derive(Clone)]
pub struct Provider {
    definition: ProviderDefinition,
    config: NormalizedModelProviderConfig,
    adapter: Arc<dyn ProviderAdapter>,
    client: Arc<dyn OperationClient>,
}

impl Provider {
    pub(crate) fn instantiate(
        definition: ProviderDefinition,
        config: NormalizedModelProviderConfig,
        client: Arc<dyn OperationClient>,
    ) -> Result<Self, ModelProviderError> {
        if definition.id != config.provider {
            return Err(ProviderConfigError::ProviderMismatch {
                configured: config.provider,
                selected: definition.id,
            }
            .into());
        }
        let adapter = providers::instantiate(definition.adapter, &config);
        Ok(Self {
            definition,
            config,
            adapter,
            client,
        })
    }

    pub fn id(&self) -> &ProviderId {
        &self.definition.id
    }

    pub fn definition(&self) -> &ProviderDefinition {
        &self.definition
    }

    pub fn config(&self) -> &NormalizedModelProviderConfig {
        &self.config
    }

    pub fn protocol(&self) -> ApiProtocol {
        self.adapter.protocol()
    }

    pub fn build_model(
        &self,
        model_id: &ModelId,
    ) -> Result<Arc<dyn ModelInvoker>, ModelProviderError> {
        let model = self.resolve_model(model_id)?;
        Ok(Arc::new(RegisteredModelInvoker {
            provider: self.clone(),
            model,
        }))
    }

    pub fn complete(
        &self,
        model_id: &ModelId,
        request: &ModelRequest,
    ) -> Result<ModelResponse, ModelProviderError> {
        self.complete_with_cancellation(model_id, request, &CancellationSource::new().token())
    }

    pub fn complete_with_cancellation(
        &self,
        model_id: &ModelId,
        request: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, ModelProviderError> {
        let model = self.resolve_model(model_id)?;
        self.adapter.complete(
            model.id.as_str(),
            request,
            self.client.as_ref(),
            cancellation,
        )
    }

    fn resolve_model(&self, model_id: &ModelId) -> Result<Model, ModelProviderError> {
        if let Some(model) = self
            .definition
            .models
            .iter()
            .find(|model| &model.id == model_id)
        {
            return Ok(model.clone());
        }
        match self.definition.model_catalog_policy {
            ModelCatalogPolicy::ListedOnly => Err(ModelProviderError::ModelNotRegistered {
                provider: self.definition.id.clone(),
                model: model_id.clone(),
            }),
            ModelCatalogPolicy::AllowUnlisted => {
                Ok(Model::new(model_id.clone(), model_id.as_str()))
            }
        }
    }
}

/// Process-local runtime that instantiates declarative provider configuration.
pub struct ModelProviderRuntime {
    configs: ProviderConfigRegistry,
    client: Arc<dyn OperationClient>,
}

impl ModelProviderRuntime {
    pub fn new(configs: ProviderConfigRegistry) -> Self {
        Self::with_client(
            configs,
            Arc::new(ZetaClient::new(Arc::new(UreqHttpClient::new()))),
        )
    }

    pub fn with_client(configs: ProviderConfigRegistry, client: Arc<dyn OperationClient>) -> Self {
        Self { configs, client }
    }

    pub fn builtin() -> Self {
        Self::new(ProviderConfigRegistry::builtin())
    }

    pub fn builtin_with_client(client: Arc<dyn OperationClient>) -> Self {
        Self::with_client(ProviderConfigRegistry::builtin(), client)
    }

    pub fn instantiate(
        &self,
        config: &ModelProviderConfig,
    ) -> Result<Provider, ModelProviderError> {
        let normalized = self.configs.normalize(config)?;
        self.instantiate_normalized(normalized)
    }

    pub fn build_model(
        &self,
        config: &ModelProviderConfig,
        model_ref: &ModelRef,
    ) -> Result<Arc<dyn ModelInvoker>, ModelProviderError> {
        let normalized = self.configs.normalize_for(config, &model_ref.provider)?;
        self.instantiate_normalized(normalized)?
            .build_model(&model_ref.model)
    }

    pub fn complete(
        &self,
        config: &ModelProviderConfig,
        model_ref: &ModelRef,
        request: &ModelRequest,
    ) -> Result<ModelResponse, ModelProviderError> {
        let normalized = self.configs.normalize_for(config, &model_ref.provider)?;
        self.instantiate_normalized(normalized)?
            .complete(&model_ref.model, request)
    }

    fn instantiate_normalized(
        &self,
        normalized: NormalizedModelProviderConfig,
    ) -> Result<Provider, ModelProviderError> {
        let definition = self
            .configs
            .get(&normalized.provider)
            .expect("normalization only succeeds for registered providers")
            .clone();
        Provider::instantiate(definition, normalized, self.client.clone())
    }
}

impl Default for ModelProviderRuntime {
    fn default() -> Self {
        Self::builtin()
    }
}

#[derive(Clone)]
pub struct ModelRuntimeRequest {
    pub model: ModelRef,
    pub config: ModelProviderConfig,
}

impl ModelRuntimeRequest {
    pub fn new(model: ModelRef, config: ModelProviderConfig) -> Self {
        Self { model, config }
    }
}

/// Invokes one immutable provider/model selection with a canonical request.
///
/// Implementations own provider transport and wire adaptation. They must not read Core Thread
/// state or mutable product configuration; a newly resolved invoker is used when configuration
/// changes should affect a later invocation.
pub trait ModelInvoker: Send + Sync {
    fn invoke(&self, request: &ModelRequest) -> Result<ModelResponse, ModelProviderError>;

    /// Invokes this immutable model snapshot within one caller-owned cancellation scope.
    ///
    /// Implementations with a cancellable transport must override this method and propagate the
    /// token to the active operation. The compatibility default rejects cancellation before and
    /// after synchronous implementations so their late result cannot be accepted.
    fn invoke_with_cancellation(
        &self,
        request: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, ModelProviderError> {
        check_cancellation(cancellation)?;
        let response = self.invoke(request)?;
        check_cancellation(cancellation)?;
        Ok(response)
    }
}

/// Resolves declarative provider configuration into immutable Zeta model runtimes.
///
/// Implementations must validate and normalize configuration before creating runtime state,
/// resolve the provider-specific API adapter and endpoint, and keep transport or client state out
/// of the serializable configuration layer.
pub trait ModelProvider: Send + Sync {
    fn runtime(
        &self,
        request: ModelRuntimeRequest,
    ) -> Result<Arc<dyn ModelInvoker>, ModelProviderError>;
}

impl ModelProvider for ModelProviderRuntime {
    fn runtime(
        &self,
        request: ModelRuntimeRequest,
    ) -> Result<Arc<dyn ModelInvoker>, ModelProviderError> {
        self.build_model(&request.config, &request.model)
    }
}

struct RegisteredModelInvoker {
    provider: Provider,
    model: Model,
}

impl ModelInvoker for RegisteredModelInvoker {
    fn invoke(&self, request: &ModelRequest) -> Result<ModelResponse, ModelProviderError> {
        self.invoke_with_cancellation(request, &CancellationSource::new().token())
    }

    fn invoke_with_cancellation(
        &self,
        request: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, ModelProviderError> {
        let mut request = request.clone();
        request.max_output_tokens = request
            .max_output_tokens
            .or(self.provider.config.max_output_tokens);
        self.provider
            .complete_with_cancellation(&self.model.id, &request, cancellation)
    }
}

/// Returns a clear model error when Zeta cannot resolve a configured model runtime.
pub struct UnavailableModel {
    message: String,
}

impl UnavailableModel {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl ModelInvoker for UnavailableModel {
    fn invoke(&self, _: &ModelRequest) -> Result<ModelResponse, ModelProviderError> {
        Err(ModelProviderError::Unavailable(self.message.clone()))
    }
}

/// Deterministic model adapter used only by unit tests and local protocol fixtures.
pub struct EchoModel;

impl ModelInvoker for EchoModel {
    fn invoke(&self, request: &ModelRequest) -> Result<ModelResponse, ModelProviderError> {
        let prompt = request
            .input
            .iter()
            .rev()
            .find_map(|item| match item {
                InputItem::Message(message) => message.content.iter().find_map(|content| {
                    let ContentPart::Text(text) = content else {
                        return None;
                    };
                    Some(text.as_str())
                }),
                InputItem::ToolResult(_) => None,
            })
            .unwrap_or_default();
        Ok(ModelResponse {
            output: vec![OutputItem::Text(format!("Zeta: {prompt}"))],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

fn check_cancellation(cancellation: &CancellationToken) -> Result<(), ModelProviderError> {
    cancellation
        .check()
        .map_err(|signal| ModelProviderError::Cancelled(signal.reason().to_string()))
}
