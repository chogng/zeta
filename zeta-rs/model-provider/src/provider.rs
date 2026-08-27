use crate::ModelProviderError;
use crate::lazy_client::LazyOperationClient;
use crate::providers;
use crate::providers::ProviderAdapter;
use crate::providers::ProviderTarget;
use std::sync::Arc;
use zeta_api::ApiProtocol;
use zeta_api::ContentPart;
use zeta_api::InputItem;
use zeta_api::ModelRequest;
use zeta_api::ModelResponse;
use zeta_api::ModelStreamEvent;
use zeta_api::OutputItem;
use zeta_api::StopReason;
use zeta_async_utils::CancellationSource;
use zeta_async_utils::CancellationToken;
use zeta_chatgpt::ChatGptOAuth;
use zeta_client::ClientError;
use zeta_client::OperationClient;
use zeta_client::ResolvedApiTarget;
use zeta_client::ZetaClient;
use zeta_context_engine::ContextTokenMeasurementCapability;
use zeta_context_engine::ContextTokenMeasurementOutcome;
use zeta_http_client::HttpHeader;
use zeta_http_client::UreqHttpClient;
use zeta_kimi::KimiOAuth;
use zeta_model_provider_config::ApiKeyPolicy;
use zeta_model_provider_config::Model;
use zeta_model_provider_config::ModelId;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_model_provider_config::NormalizedModelProviderConfig;
use zeta_model_provider_config::ProviderAdapter as ProviderAdapterKind;
use zeta_model_provider_config::ProviderConfigError;
use zeta_model_provider_config::ProviderConfigRegistry;
use zeta_model_provider_config::ProviderDefinition;
use zeta_model_provider_config::ProviderId;
use zeta_model_provider_config::StaticModelRuntime;
use zeta_model_provider_config::find_static_model;
use zeta_model_tokenizer::LocalTokenizerRegistry;
use zeta_model_tokenizer::LocalTokenizerService;
use zeta_models_manager::ModelRequirements;
use zeta_models_manager::ModelsManager;
use zeta_models_manager::ModelsManagerError;
use zeta_protocol::CapabilitySupport;
use zeta_protocol::ModelOutputTransport;
use zeta_protocol::ModelRef;
use zeta_secrets::SecretKey;
use zeta_secrets::SecretStore;
use zeta_secrets::UnavailableSecretStore;

pub fn provider_api_key_secret_key(provider: &ProviderId) -> SecretKey {
    SecretKey::new(format!("provider/{provider}/default/api-key"))
        .expect("validated provider IDs produce valid secret keys")
}

#[derive(Clone)]
pub struct Provider {
    definition: ProviderDefinition,
    config: NormalizedModelProviderConfig,
    models: ModelsManager,
    adapter: Arc<dyn ProviderAdapter>,
    client: Arc<dyn OperationClient>,
    local_counter: providers::measurement::LocalInputTokenCounter,
}

impl Provider {
    pub(crate) fn instantiate(
        definition: ProviderDefinition,
        config: NormalizedModelProviderConfig,
        models: ModelsManager,
        client: Arc<dyn OperationClient>,
        local_tokenizers: Arc<dyn LocalTokenizerService>,
        adapter_target: Option<ProviderTarget>,
    ) -> Result<Self, ModelProviderError> {
        if definition.id != config.provider {
            return Err(ProviderConfigError::ProviderMismatch {
                configured: config.provider,
                selected: definition.id,
            }
            .into());
        }
        let adapter = providers::instantiate(definition.adapter, &config, adapter_target);
        let local_counter = providers::measurement::LocalInputTokenCounter::new(
            config.provider.clone(),
            local_tokenizers,
        );
        Ok(Self {
            definition,
            config,
            models,
            adapter,
            client,
            local_counter,
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
        let mut request = request.clone();
        let supports_original =
            model.capabilities.image_detail_original == CapabilitySupport::Supported;
        let _image_detail_decisions = request.sanitize_image_details(supports_original);
        self.adapter.complete(
            model.id.as_str(),
            &request,
            self.client.as_ref(),
            cancellation,
        )
    }

    pub fn stream_with_cancellation(
        &self,
        model_id: &ModelId,
        request: &ModelRequest,
        cancellation: &CancellationToken,
        sink: &mut dyn ModelEventSink,
    ) -> Result<ModelResponse, ModelProviderError> {
        let model = self.resolve_model(model_id)?;
        let mut request = request.clone();
        let supports_original =
            model.capabilities.image_detail_original == CapabilitySupport::Supported;
        let _image_detail_decisions = request.sanitize_image_details(supports_original);
        self.adapter.stream(
            model.id.as_str(),
            &request,
            self.client.as_ref(),
            cancellation,
            sink,
        )
    }

    pub fn input_token_measurement_capability(
        &self,
        model_id: &ModelId,
    ) -> Result<ContextTokenMeasurementCapability, ModelProviderError> {
        let model = self.resolve_model(model_id)?;
        let provider = self
            .adapter
            .input_token_measurement_capability(model.id.as_str());
        if provider != ContextTokenMeasurementCapability::Unavailable {
            Ok(provider)
        } else if self.local_counter.supports(model.id.as_str()) {
            Ok(ContextTokenMeasurementCapability::Local)
        } else {
            Ok(ContextTokenMeasurementCapability::Unavailable)
        }
    }

    pub fn measure_input_with_cancellation(
        &self,
        model_id: &ModelId,
        request: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ContextTokenMeasurementOutcome, ModelProviderError> {
        let model = self.resolve_model(model_id)?;
        let provider = self.adapter.measure_input(
            model.id.as_str(),
            request,
            self.client.as_ref(),
            cancellation,
        );
        match provider {
            Ok(ContextTokenMeasurementOutcome::Measured(measurement)) => {
                return Ok(ContextTokenMeasurementOutcome::Measured(measurement));
            }
            Err(ModelProviderError::Cancelled(message)) => {
                return Err(ModelProviderError::Cancelled(message));
            }
            Ok(ContextTokenMeasurementOutcome::Unavailable) | Err(_) => {}
        }
        match self
            .local_counter
            .count(model.id.as_str(), request, cancellation)
        {
            Ok(outcome) => Ok(outcome),
            Err(ModelProviderError::Cancelled(message)) => {
                Err(ModelProviderError::Cancelled(message))
            }
            Err(_) => Ok(ContextTokenMeasurementOutcome::Unavailable),
        }
    }

    fn resolve_model(&self, model_id: &ModelId) -> Result<Model, ModelProviderError> {
        self.models
            .resolve_static(
                &ModelRef::new(self.definition.id.clone(), model_id.clone()),
                &ModelRequirements::agent(),
            )
            .map(|resolved| resolved.entry().info().clone())
            .map_err(model_resolution_error)
    }
}

/// Process-local runtime that instantiates declarative provider configuration.
pub struct ModelProviderRuntime {
    configs: ProviderConfigRegistry,
    models: ModelsManager,
    client: Arc<dyn OperationClient>,
    secrets: Arc<dyn SecretStore>,
    enforce_api_keys: bool,
    local_tokenizers: Arc<dyn LocalTokenizerService>,
    chatgpt_oauth: Option<Arc<ChatGptOAuth>>,
    kimi_oauth: Option<Arc<KimiOAuth>>,
}

impl ModelProviderRuntime {
    pub fn new(configs: ProviderConfigRegistry) -> Self {
        Self::with_client(
            configs,
            Arc::new(LazyOperationClient::new(production_client)),
        )
    }

    pub fn with_client(configs: ProviderConfigRegistry, client: Arc<dyn OperationClient>) -> Self {
        let models = ModelsManager::new(configs.clone());
        Self {
            configs,
            models,
            client,
            secrets: Arc::new(UnavailableSecretStore),
            enforce_api_keys: false,
            local_tokenizers: Arc::new(LocalTokenizerRegistry::new()),
            chatgpt_oauth: None,
            kimi_oauth: None,
        }
    }

    pub fn with_secrets(configs: ProviderConfigRegistry, secrets: Arc<dyn SecretStore>) -> Self {
        Self::with_client_and_secrets(
            configs,
            Arc::new(LazyOperationClient::new(production_client)),
            secrets,
        )
    }

    pub fn with_client_and_secrets(
        configs: ProviderConfigRegistry,
        client: Arc<dyn OperationClient>,
        secrets: Arc<dyn SecretStore>,
    ) -> Self {
        let models = ModelsManager::new(configs.clone());
        Self {
            configs,
            models,
            client,
            secrets,
            enforce_api_keys: true,
            local_tokenizers: Arc::new(LocalTokenizerRegistry::new()),
            chatgpt_oauth: None,
            kimi_oauth: None,
        }
    }

    /// Installs the read-only local tokenizer service used by provider/model adapters.
    ///
    /// The host must finish loading and validating model bindings before composition. Existing
    /// model invokers remain immutable and retain the service snapshot they were created with.
    pub fn with_local_tokenizers(
        mut self,
        local_tokenizers: Arc<dyn LocalTokenizerService>,
    ) -> Self {
        self.local_tokenizers = local_tokenizers;
        self
    }

    /// Installs the native Kimi Code OAuth authority used by subscription model rows.
    pub fn with_kimi_oauth(mut self, kimi_oauth: Arc<KimiOAuth>) -> Self {
        self.kimi_oauth = Some(kimi_oauth);
        self
    }

    /// Installs the native ChatGPT OAuth authority used by subscription model rows.
    pub fn with_chatgpt_oauth(mut self, chatgpt_oauth: Arc<ChatGptOAuth>) -> Self {
        self.chatgpt_oauth = Some(chatgpt_oauth);
        self
    }

    pub fn builtin() -> Self {
        Self::new(ProviderConfigRegistry::builtin())
    }

    pub fn builtin_with_client(client: Arc<dyn OperationClient>) -> Self {
        Self::with_client(ProviderConfigRegistry::builtin(), client)
    }

    /// Returns the shared catalog manager used by this runtime for model resolution.
    pub fn models_manager(&self) -> ModelsManager {
        self.models.clone()
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
        let adapter_target = self.adapter_target(&normalized, model_ref)?;
        self.instantiate_normalized_with_target(normalized, adapter_target)?
            .build_model(&model_ref.model)
    }

    pub fn complete(
        &self,
        config: &ModelProviderConfig,
        model_ref: &ModelRef,
        request: &ModelRequest,
    ) -> Result<ModelResponse, ModelProviderError> {
        let normalized = self.configs.normalize_for(config, &model_ref.provider)?;
        let adapter_target = self.adapter_target(&normalized, model_ref)?;
        self.instantiate_normalized_with_target(normalized, adapter_target)?
            .complete(&model_ref.model, request)
    }

    fn instantiate_normalized(
        &self,
        normalized: NormalizedModelProviderConfig,
    ) -> Result<Provider, ModelProviderError> {
        self.instantiate_normalized_with_target(normalized, None)
    }

    fn instantiate_normalized_with_target(
        &self,
        normalized: NormalizedModelProviderConfig,
        adapter_target: Option<ProviderTarget>,
    ) -> Result<Provider, ModelProviderError> {
        let definition = self
            .configs
            .get(&normalized.provider)
            .expect("normalization only succeeds for registered providers")
            .clone();
        Provider::instantiate(
            definition,
            normalized,
            self.models.clone(),
            self.client.clone(),
            self.local_tokenizers.clone(),
            adapter_target,
        )
    }

    fn adapter_target(
        &self,
        config: &NormalizedModelProviderConfig,
        model: &ModelRef,
    ) -> Result<Option<ProviderTarget>, ModelProviderError> {
        match find_static_model(model).map(|spec| spec.runtime) {
            Some(StaticModelRuntime::KimiCode) => self
                .kimi_oauth
                .as_ref()
                .ok_or_else(|| {
                    ModelProviderError::Credential("Kimi Code OAuth is unavailable".into())
                })?
                .api_target()
                .map(|target| Some(ProviderTarget::subscription(target)))
                .map_err(|error| ModelProviderError::Credential(error.to_string())),
            Some(StaticModelRuntime::ChatGptSubscription) => self
                .chatgpt_oauth
                .as_ref()
                .ok_or_else(|| {
                    ModelProviderError::Credential("ChatGPT OAuth is unavailable".into())
                })?
                .api_target()
                .map(|target| Some(ProviderTarget::subscription(target)))
                .map_err(|error| ModelProviderError::Credential(error.to_string())),
            Some(StaticModelRuntime::ProviderApi) | None => self.provider_api_target(config),
        }
    }

    fn provider_api_target(
        &self,
        config: &NormalizedModelProviderConfig,
    ) -> Result<Option<ProviderTarget>, ModelProviderError> {
        if !self.enforce_api_keys {
            return Ok(None);
        }
        let definition = self
            .configs
            .get(&config.provider)
            .expect("normalized provider is registered");
        if definition.api_key_policy == ApiKeyPolicy::Unsupported {
            return Ok(None);
        }
        let secret = self
            .secrets
            .load(&provider_api_key_secret_key(&config.provider))?;
        let Some(secret) = secret else {
            return match definition.api_key_policy {
                ApiKeyPolicy::Optional => Ok(None),
                ApiKeyPolicy::Required => Err(ModelProviderError::Credential(format!(
                    "no API key is stored for provider '{}'",
                    config.provider
                ))),
                ApiKeyPolicy::Unsupported => unreachable!("handled above"),
            };
        };
        let value = std::str::from_utf8(secret.expose())
            .map_err(|_| ModelProviderError::Credential("stored API key is not UTF-8".into()))?;
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(ModelProviderError::Credential(
                "stored API key is invalid".into(),
            ));
        }
        let mut headers = match definition.adapter {
            ProviderAdapterKind::Google => {
                vec![HttpHeader::new("x-goog-api-client", "zeta/0.1")]
            }
            ProviderAdapterKind::Zai => {
                vec![HttpHeader::new("Accept-Language", "en-US,en")]
            }
            _ => Vec::new(),
        };
        headers.push(match definition.adapter {
            ProviderAdapterKind::Anthropic => HttpHeader::new("x-api-key", value),
            ProviderAdapterKind::Google => HttpHeader::new("x-goog-api-key", value),
            ProviderAdapterKind::Ollama => unreachable!("Ollama rejects API-key storage"),
            _ => HttpHeader::new("Authorization", format!("Bearer {value}")),
        });
        Ok(Some(ProviderTarget::provider_api(ResolvedApiTarget::new(
            config.base_url.clone(),
            headers,
        ))))
    }
}

fn model_resolution_error(error: ModelsManagerError) -> ModelProviderError {
    match error {
        ModelsManagerError::ModelNotListed { provider, model } => {
            ModelProviderError::ModelNotRegistered { provider, model }
        }
        error => ModelProviderError::Unavailable(error.to_string()),
    }
}

fn production_client() -> Result<Arc<dyn OperationClient>, ClientError> {
    let transport = UreqHttpClient::new()?;
    Ok(Arc::new(ZetaClient::new(Arc::new(transport))))
}

impl crate::SemanticModelProvider for ModelProviderRuntime {
    fn embedding_runtime(
        &self,
        request: crate::EmbeddingRuntimeRequest,
    ) -> Result<Arc<dyn crate::EmbeddingInvoker>, ModelProviderError> {
        crate::semantic_runtime::SemanticRuntimeResolver {
            configs: self.configs.clone(),
            client: Arc::clone(&self.client),
            secrets: Arc::clone(&self.secrets),
        }
        .embedding_runtime(request)
    }

    fn rerank_runtime(
        &self,
        request: crate::RerankRuntimeRequest,
    ) -> Result<Arc<dyn crate::RerankInvoker>, ModelProviderError> {
        crate::semantic_runtime::SemanticRuntimeResolver {
            configs: self.configs.clone(),
            client: Arc::clone(&self.client),
            secrets: Arc::clone(&self.secrets),
        }
        .rerank_runtime(request)
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

/// Receives provider-neutral model deltas from one immutable model invocation.
///
/// Implementations must preserve event order and should return an error when
/// cancellation or downstream lifecycle prevents more output from being
/// accepted.
pub trait ModelEventSink {
    fn emit(&mut self, event: ModelStreamEvent) -> Result<(), ModelProviderError>;
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

    /// Reports whether this immutable runtime uses a native provider stream or a unary call.
    fn output_transport(&self) -> ModelOutputTransport {
        ModelOutputTransport::Unary
    }

    /// Reports the cost category of this immutable model's input-token measurement contract.
    fn input_token_measurement_capability(&self) -> ContextTokenMeasurementCapability {
        ContextTokenMeasurementCapability::Unavailable
    }

    /// Measures one fully assembled request using a fresh compatibility cancellation scope.
    fn measure_input(
        &self,
        request: &ModelRequest,
    ) -> Result<ContextTokenMeasurementOutcome, ModelProviderError> {
        self.measure_input_with_cancellation(request, &CancellationSource::new().token())
    }

    /// Measures input tokens within one caller-owned cancellation scope.
    ///
    /// Implementations that declare a local or remote capability must override this method and
    /// measure the same canonical request snapshot that will be passed to invocation.
    fn measure_input_with_cancellation(
        &self,
        _: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ContextTokenMeasurementOutcome, ModelProviderError> {
        check_cancellation(cancellation)?;
        Ok(ContextTokenMeasurementOutcome::Unavailable)
    }

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

    /// Streams incremental output within one caller-owned cancellation scope.
    ///
    /// The compatibility default invokes the unary implementation and emits
    /// final text and reasoning items as one event each. Wire-streaming model
    /// runtimes should override this method.
    fn stream_with_cancellation(
        &self,
        request: &ModelRequest,
        cancellation: &CancellationToken,
        sink: &mut dyn ModelEventSink,
    ) -> Result<ModelResponse, ModelProviderError> {
        let response = self.invoke_with_cancellation(request, cancellation)?;
        emit_model_response(&response, sink)?;
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
        let request = self.prepare_request(request);
        self.provider
            .complete_with_cancellation(&self.model.id, &request, cancellation)
    }

    fn output_transport(&self) -> ModelOutputTransport {
        self.provider.definition.output_transport
    }

    fn stream_with_cancellation(
        &self,
        request: &ModelRequest,
        cancellation: &CancellationToken,
        sink: &mut dyn ModelEventSink,
    ) -> Result<ModelResponse, ModelProviderError> {
        let request = self.prepare_request(request);
        self.provider
            .stream_with_cancellation(&self.model.id, &request, cancellation, sink)
    }

    fn input_token_measurement_capability(&self) -> ContextTokenMeasurementCapability {
        self.provider
            .input_token_measurement_capability(&self.model.id)
            .unwrap_or(ContextTokenMeasurementCapability::Unavailable)
    }

    fn measure_input_with_cancellation(
        &self,
        request: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ContextTokenMeasurementOutcome, ModelProviderError> {
        let request = self.prepare_request(request);
        self.provider
            .measure_input_with_cancellation(&self.model.id, &request, cancellation)
    }
}

impl RegisteredModelInvoker {
    fn prepare_request(&self, request: &ModelRequest) -> ModelRequest {
        let mut request = request.clone();
        request.max_output_tokens = request
            .max_output_tokens
            .or(self.provider.config.max_output_tokens);
        request
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

fn emit_model_response(
    response: &ModelResponse,
    sink: &mut dyn ModelEventSink,
) -> Result<(), ModelProviderError> {
    for item in &response.output {
        let event = match item {
            OutputItem::Text(text) => Some(ModelStreamEvent::TextDelta(text.clone())),
            OutputItem::Reasoning(text) => Some(ModelStreamEvent::ReasoningDelta(text.clone())),
            OutputItem::Refusal(_) | OutputItem::ToolCall(_) => None,
        };
        if let Some(event) = event {
            sink.emit(event)?;
        }
    }
    Ok(())
}
