use crate::anthropic;
use crate::{
    deepseek, google, huggingface, kimi, mimo, minimax, ollama, openai, openai_compatible, qwen,
    xai, zai,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use zeta_api::{
    Api, ApiProtocol, HttpHeader, JsonHttpTransport, ModelRequest, ModelResponse,
    ResolvedApiTarget, UreqJsonHttpTransport,
};
use zeta_core::{AgentModel, CoreError};
use zeta_credentials::CredentialStore;

macro_rules! string_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ProviderRegistryError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(ProviderRegistryError(
                        concat!($label, " must not be empty").into(),
                    ));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

string_id!(ProviderId, "provider ID");
string_id!(ModelId, "model ID");

/// A globally unambiguous reference to one model under one provider.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRef {
    pub provider: ProviderId,
    pub model: ModelId,
}

impl ModelRef {
    pub fn new(provider: ProviderId, model: ModelId) -> Self {
        Self { provider, model }
    }
}

/// How a provider chooses the base URL used for requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EndpointPolicy {
    ProviderDefault(String),
    ConfiguredOnly,
}

impl EndpointPolicy {
    pub fn resolve(&self, configured_base_url: &str) -> Result<String, CoreError> {
        if !configured_base_url.trim().is_empty() {
            return Ok(configured_base_url.to_owned());
        }
        match self {
            Self::ProviderDefault(base_url) => Ok(base_url.clone()),
            Self::ConfiguredOnly => Err(CoreError::Model(
                "model provider requires a configured base URL".into(),
            )),
        }
    }
}

/// Whether a model capability is known to be available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilitySupport {
    Supported,
    Unsupported,
    Unknown,
}

/// Context-window metadata for a registered model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextWindow {
    Known(u32),
    Unknown,
}

/// Capabilities advertised for one registered model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelCapabilities {
    pub tools: CapabilitySupport,
    pub reasoning: CapabilitySupport,
}

impl ModelCapabilities {
    pub const UNKNOWN: Self = Self {
        tools: CapabilitySupport::Unknown,
        reasoning: CapabilitySupport::Unknown,
    };
}

/// Metadata for one model mounted under a provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Model {
    pub id: ModelId,
    pub display_name: String,
    pub context_window: ContextWindow,
    pub capabilities: ModelCapabilities,
}

impl Model {
    pub fn new(id: ModelId, display_name: impl Into<String>) -> Self {
        Self {
            id,
            display_name: display_name.into(),
            context_window: ContextWindow::Unknown,
            capabilities: ModelCapabilities::UNKNOWN,
        }
    }
}

/// How the registry handles model IDs not present in a provider's mounted model map.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelCatalogPolicy {
    ListedOnly,
    AllowUnlisted,
}

/// Credential placement used at one provider boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderAuthentication {
    Bearer,
    ApiKeyHeader(String),
    None,
}

/// One provider registration containing its request boundary and mounted model catalog.
#[derive(Clone, Debug)]
pub struct Provider {
    pub id: ProviderId,
    pub name: String,
    pub api: Api,
    pub endpoint: EndpointPolicy,
    pub models: HashMap<ModelId, Model>,
    pub model_catalog_policy: ModelCatalogPolicy,
    pub authentication: ProviderAuthentication,
    pub headers: Vec<HttpHeader>,
}

impl Provider {
    pub fn new(
        id: ProviderId,
        name: impl Into<String>,
        api: Api,
        endpoint: EndpointPolicy,
        model_catalog_policy: ModelCatalogPolicy,
        authentication: ProviderAuthentication,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            api,
            endpoint,
            models: HashMap::new(),
            model_catalog_policy,
            authentication,
            headers: Vec::new(),
        }
    }

    pub fn protocol(&self) -> ApiProtocol {
        self.api.protocol()
    }

    pub fn with_headers(mut self, headers: impl IntoIterator<Item = HttpHeader>) -> Self {
        self.headers.extend(headers);
        self
    }

    pub fn register_model(&mut self, model: Model) -> Result<(), ProviderRegistryError> {
        if self.models.contains_key(&model.id) {
            return Err(ProviderRegistryError(format!(
                "model ID '{}' is already registered under provider '{}'",
                model.id, self.id
            )));
        }
        self.models.insert(model.id.clone(), model);
        Ok(())
    }

    pub fn with_models(
        mut self,
        models: impl IntoIterator<Item = Model>,
    ) -> Result<Self, ProviderRegistryError> {
        for model in models {
            self.register_model(model)?;
        }
        Ok(self)
    }

    fn resolve_model(&self, model_id: &ModelId) -> Result<Model, CoreError> {
        if let Some(model) = self.models.get(model_id) {
            return Ok(model.clone());
        }
        match self.model_catalog_policy {
            ModelCatalogPolicy::ListedOnly => Err(CoreError::Model(format!(
                "model '{}' is not registered under provider '{}'",
                model_id, self.id
            ))),
            ModelCatalogPolicy::AllowUnlisted => {
                Ok(Model::new(model_id.clone(), model_id.as_str()))
            }
        }
    }

    fn resolve_target(
        &self,
        config: &ModelProviderConfig,
        credential_store: &dyn CredentialStore,
    ) -> Result<ResolvedApiTarget, CoreError> {
        let mut headers = self.headers.clone();
        match &self.authentication {
            ProviderAuthentication::Bearer => headers.push(HttpHeader::new(
                "Authorization",
                format!(
                    "Bearer {}",
                    read_credential(credential_store, &config.credential_account)?
                ),
            )),
            ProviderAuthentication::ApiKeyHeader(name) => headers.push(HttpHeader::new(
                name,
                read_credential(credential_store, &config.credential_account)?,
            )),
            ProviderAuthentication::None => {}
        }
        Ok(ResolvedApiTarget::new(
            self.endpoint.resolve(&config.base_url)?,
            headers,
        ))
    }
}

/// A two-level registry indexed by provider ID and then model ID.
pub struct ProviderRegistry {
    providers: HashMap<ProviderId, Provider>,
    transport: Arc<dyn JsonHttpTransport>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::with_transport(Arc::new(UreqJsonHttpTransport::new()))
    }

    pub fn with_transport(transport: Arc<dyn JsonHttpTransport>) -> Self {
        Self {
            providers: HashMap::new(),
            transport,
        }
    }

    pub fn builtin() -> Self {
        let mut registry = Self::new();
        register_builtins(&mut registry);
        registry
    }

    pub fn builtin_with_transport(transport: Arc<dyn JsonHttpTransport>) -> Self {
        let mut registry = Self::with_transport(transport);
        register_builtins(&mut registry);
        registry
    }

    pub fn register_provider(&mut self, provider: Provider) -> Result<(), ProviderRegistryError> {
        if self.providers.contains_key(&provider.id) {
            return Err(ProviderRegistryError(format!(
                "provider ID '{}' is already registered",
                provider.id
            )));
        }
        self.providers.insert(provider.id.clone(), provider);
        Ok(())
    }

    pub fn get_provider(&self, provider_id: &ProviderId) -> Option<&Provider> {
        self.providers.get(provider_id)
    }

    pub fn get_model(&self, model_ref: &ModelRef) -> Option<&Model> {
        self.providers
            .get(&model_ref.provider)?
            .models
            .get(&model_ref.model)
    }

    pub fn providers(&self) -> impl Iterator<Item = &Provider> {
        self.providers.values()
    }

    pub fn build_model(
        &self,
        config: &ModelProviderConfig,
        model_ref: &ModelRef,
        credential_store: Arc<dyn CredentialStore>,
    ) -> Result<Arc<dyn AgentModel>, CoreError> {
        let provider = self.providers.get(&model_ref.provider).ok_or_else(|| {
            CoreError::Model(format!(
                "model provider '{}' is not registered",
                model_ref.provider
            ))
        })?;
        let model = provider.resolve_model(&model_ref.model)?;
        provider.resolve_target(config, credential_store.as_ref())?;
        Ok(Arc::new(RegisteredAgentModel {
            provider: provider.clone(),
            model,
            config: config.clone(),
            credential_store,
            transport: self.transport.clone(),
        }))
    }

    pub fn complete(
        &self,
        config: &ModelProviderConfig,
        model_ref: &ModelRef,
        credential_store: &dyn CredentialStore,
        request: &ModelRequest,
    ) -> Result<ModelResponse, CoreError> {
        let provider = self.providers.get(&model_ref.provider).ok_or_else(|| {
            CoreError::Model(format!(
                "model provider '{}' is not registered",
                model_ref.provider
            ))
        })?;
        let model = provider.resolve_model(&model_ref.model)?;
        let target = provider.resolve_target(config, credential_store)?;
        provider
            .api
            .complete_with_transport(&target, model.id.as_str(), request, self.transport.as_ref())
            .map_err(|error| CoreError::Model(error.to_string()))
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

struct RegisteredAgentModel {
    provider: Provider,
    model: Model,
    config: ModelProviderConfig,
    credential_store: Arc<dyn CredentialStore>,
    transport: Arc<dyn JsonHttpTransport>,
}

impl AgentModel for RegisteredAgentModel {
    fn respond(&self, prompt: &str) -> Result<String, CoreError> {
        let target = self
            .provider
            .resolve_target(&self.config, self.credential_store.as_ref())?;
        let mut request = ModelRequest::text(prompt);
        request.max_output_tokens = self.config.max_output_tokens;
        let response = self
            .provider
            .api
            .complete_with_transport(
                &target,
                self.model.id.as_str(),
                &request,
                self.transport.as_ref(),
            )
            .map_err(|error| CoreError::Model(error.to_string()))?;
        let text = response.text();
        if !text.trim().is_empty() {
            return Ok(text);
        }
        if response.tool_calls().next().is_some() {
            return Err(CoreError::Model(
                "model returned a tool call, but the current AgentModel port only accepts text"
                    .into(),
            ));
        }
        Err(CoreError::Model(
            "model provider returned no text response".into(),
        ))
    }
}

fn register_builtins(registry: &mut ProviderRegistry) {
    for provider in [
        openai::provider(),
        openai_compatible::provider(),
        google::provider(),
        xai::provider(),
        qwen::provider(),
        kimi::provider(),
        deepseek::provider(),
        ollama::provider(),
        huggingface::provider(),
        zai::provider(),
        minimax::provider(),
        mimo::provider(),
        anthropic::provider(),
    ] {
        registry
            .register_provider(provider)
            .expect("unique provider ID");
    }
}

fn read_credential(
    credential_store: &dyn CredentialStore,
    credential_account: &str,
) -> Result<String, CoreError> {
    credential_store
        .read_secret(credential_account)
        .map_err(|_| CoreError::Model("model credential lookup failed".into()))?
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| CoreError::Model("model API key is missing".into()))
}

/// Registration failure caused by an invalid or duplicate provider or model identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderRegistryError(pub String);

impl fmt::Display for ProviderRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ProviderRegistryError {}

/// Non-secret runtime settings for the selected provider.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProviderConfig {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub credential_account: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}
