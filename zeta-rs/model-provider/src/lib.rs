//! Model-provider instantiation, transport configuration, and API adaptation.

mod error;
mod provider;
mod providers;

pub use error::ModelProviderError;
pub use provider::EchoModel;
pub use provider::ModelInvoker;
pub use provider::ModelProvider;
pub use provider::ModelProviderRuntime;
pub use provider::ModelRuntimeRequest;
pub use provider::Provider;
pub use provider::UnavailableModel;
pub use zeta_api::ApiProtocol;
pub use zeta_protocol::CapabilitySupport;
pub use zeta_protocol::ContextWindow;
pub use zeta_protocol::Model;
pub use zeta_protocol::ModelCapabilities;
pub use zeta_protocol::ModelId;
pub use zeta_protocol::ModelRef;
pub use zeta_protocol::ProviderId;

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
