use std::fmt;
use zeta_protocol::ProviderId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderConfigError {
    DuplicateProvider(ProviderId),
    UnknownProvider(ProviderId),
    InvalidProvider {
        provider: ProviderId,
        message: String,
    },
    MissingBaseUrl(ProviderId),
    InvalidBaseUrl {
        provider: ProviderId,
        base_url: String,
    },
    InvalidMaxOutputTokens(ProviderId),
    ProviderMismatch {
        configured: ProviderId,
        selected: ProviderId,
    },
}

impl fmt::Display for ProviderConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateProvider(provider) => {
                write!(formatter, "provider '{provider}' is already registered")
            }
            Self::UnknownProvider(provider) => {
                write!(formatter, "provider '{provider}' is not registered")
            }
            Self::InvalidProvider { provider, message } => {
                write!(formatter, "provider '{provider}' is invalid: {message}")
            }
            Self::MissingBaseUrl(provider) => {
                write!(formatter, "provider '{provider}' requires a base URL")
            }
            Self::InvalidBaseUrl { provider, base_url } => write!(
                formatter,
                "provider '{provider}' base URL must be a valid HTTP or HTTPS URL: '{base_url}'"
            ),
            Self::InvalidMaxOutputTokens(provider) => write!(
                formatter,
                "provider '{provider}' max output tokens must be greater than zero"
            ),
            Self::ProviderMismatch {
                configured,
                selected,
            } => write!(
                formatter,
                "configured provider '{configured}' does not match selected provider '{selected}'"
            ),
        }
    }
}

impl std::error::Error for ProviderConfigError {}
