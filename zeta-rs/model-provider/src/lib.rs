//! Model-provider instantiation, transport configuration, and API adaptation.

mod auth;
mod error;
mod lazy_client;
mod provider;
mod providers;
mod semantic_models;
mod semantic_runtime;

pub use error::ModelProviderError;
pub use provider::EchoModel;
pub use provider::ModelEventSink;
pub use provider::ModelInvoker;
pub use provider::ModelProvider;
pub use provider::ModelProviderRuntime;
pub use provider::ModelRuntimeRequest;
pub use provider::Provider;
pub use provider::UnavailableModel;
pub use semantic_models::EmbeddingInvoker;
pub use semantic_models::EmbeddingRequest;
pub use semantic_models::EmbeddingResponse;
pub use semantic_models::EmbeddingRuntimeIdentity;
pub use semantic_models::EmbeddingRuntimeRequest;
pub use semantic_models::EmbeddingVector;
pub use semantic_models::RerankInvoker;
pub use semantic_models::RerankRequest;
pub use semantic_models::RerankResponse;
pub use semantic_models::RerankRuntimeRequest;
pub use semantic_models::SemanticModelProvider;
pub use semantic_models::SemanticRuntimeLocation;
pub use zeta_api::ApiError;
pub use zeta_api::ApiProtocol;
pub use zeta_model_tokenizer::HttpTokenizerAssetDownloader;
pub use zeta_model_tokenizer::HuggingFaceTokenizerAssetDiscoverer;
pub use zeta_model_tokenizer::LocalTokenCount;
pub use zeta_model_tokenizer::LocalTokenizationOutcome;
pub use zeta_model_tokenizer::LocalTokenizerBinding;
pub use zeta_model_tokenizer::LocalTokenizerError;
pub use zeta_model_tokenizer::LocalTokenizerRegistry;
pub use zeta_model_tokenizer::LocalTokenizerService;
pub use zeta_model_tokenizer::ManagedLocalTokenizerService;
pub use zeta_model_tokenizer::MemoryTokenizerCapacity;
pub use zeta_model_tokenizer::PinnedTokenizerAsset;
pub use zeta_model_tokenizer::RemoteTokenizerAsset;
pub use zeta_model_tokenizer::TokenizerAssetCatalog;
pub use zeta_model_tokenizer::TokenizerAssetDownloader;
pub use zeta_model_tokenizer::TokenizerAssetManifest;
pub use zeta_model_tokenizer::TokenizerPreparationStatus;
pub use zeta_protocol::CapabilitySupport;
pub use zeta_protocol::ContextWindow;
pub use zeta_protocol::Model;
pub use zeta_protocol::ModelCapabilities;
pub use zeta_protocol::ModelId;
pub use zeta_protocol::ModelOutputTransport;
pub use zeta_protocol::ModelRef;
pub use zeta_protocol::ProviderId;

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "auth_tests.rs"]
mod auth_tests;

#[cfg(test)]
#[path = "semantic_model_tests.rs"]
mod semantic_model_tests;
pub use auth::ProviderCredentialError;
pub use auth::ProviderCredentialService;
pub use auth::ProviderCredentialStatus;
pub use auth::provider_api_key_secret_key;
