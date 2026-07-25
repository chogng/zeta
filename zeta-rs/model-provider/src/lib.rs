//! Provider registration, model catalogs, endpoints, and authentication.

mod anthropic;
mod deepseek;
mod google;
mod huggingface;
mod kimi;
mod mimo;
mod minimax;
mod ollama;
mod openai;
mod openai_compatible;
mod qwen;
mod registry;
mod xai;
mod zai;

pub use openai_compatible::EchoModel;
pub use openai_compatible::UnavailableModel;
pub use registry::CapabilitySupport;
pub use registry::ContextWindow;
pub use registry::EndpointPolicy;
pub use registry::Model;
pub use registry::ModelCapabilities;
pub use registry::ModelCatalogPolicy;
pub use registry::ModelId;
pub use registry::ModelProviderConfig;
pub use registry::ModelRef;
pub use registry::Provider;
pub use registry::ProviderAuthentication;
pub use registry::ProviderId;
pub use registry::ProviderRegistry;
pub use registry::ProviderRegistryError;
pub use zeta_api::ApiProtocol;

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
