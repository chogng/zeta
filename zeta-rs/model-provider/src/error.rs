use std::fmt;
use zeta_api::ApiError;
use zeta_model_provider_config::{ModelId, ProviderConfigError, ProviderId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelProviderError {
    Config(ProviderConfigError),
    ModelNotRegistered {
        provider: ProviderId,
        model: ModelId,
    },
    Api(ApiError),
    Cancelled(String),
    Unavailable(String),
}

impl fmt::Display for ModelProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::ModelNotRegistered { provider, model } => write!(
                formatter,
                "model '{model}' is not registered under provider '{provider}'"
            ),
            Self::Api(error) => error.fmt(formatter),
            Self::Cancelled(message) => write!(formatter, "model invocation cancelled: {message}"),
            Self::Unavailable(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ModelProviderError {}

impl From<ProviderConfigError> for ModelProviderError {
    fn from(error: ProviderConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<ApiError> for ModelProviderError {
    fn from(error: ApiError) -> Self {
        match error {
            ApiError::Cancelled(message) => Self::Cancelled(message),
            error => Self::Api(error),
        }
    }
}
