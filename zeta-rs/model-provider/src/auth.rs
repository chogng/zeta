use std::fmt;
use std::sync::Arc;

use zeta_http_client::HttpHeader;
use zeta_model_provider_config::ApiKeyHeader;
use zeta_model_provider_config::ApiKeyPolicy;
use zeta_model_provider_config::ProviderConfigRegistry;
use zeta_model_provider_config::ProviderDefinition;
use zeta_model_provider_config::ProviderId;
use zeta_secrets::SecretKey;
use zeta_secrets::SecretStore;
use zeta_secrets::SecretStoreError;
use zeta_secrets::SecretValue;

const MAX_API_KEY_BYTES: usize = 16 * 1024;

pub fn provider_api_key_secret_key(provider: &ProviderId) -> SecretKey {
    SecretKey::new(format!("provider/{provider}/default/api-key"))
        .expect("validated provider IDs produce valid secret keys")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderCredentialStatus {
    pub provider: ProviderId,
    pub display_name: String,
    pub api_key_policy: ApiKeyPolicy,
    pub api_key_configured: bool,
}

/// Owns provider-scoped API-key persistence and direct-request authentication.
///
/// App Server adapters use this service to mutate credentials, while model runtimes use the same
/// service to resolve request headers. Callers never receive stored secret values.
#[derive(Clone)]
pub struct ProviderCredentialService {
    providers: ProviderConfigRegistry,
    secrets: Arc<dyn SecretStore>,
}

impl ProviderCredentialService {
    pub fn new(providers: ProviderConfigRegistry, secrets: Arc<dyn SecretStore>) -> Self {
        Self { providers, secrets }
    }

    pub fn catalog(&self) -> Result<Vec<ProviderCredentialStatus>, ProviderCredentialError> {
        self.providers
            .providers()
            .map(|definition| {
                let api_key_configured = match definition.api_key_policy {
                    ApiKeyPolicy::Unsupported => false,
                    ApiKeyPolicy::Optional | ApiKeyPolicy::Required => self
                        .secrets
                        .load(&provider_api_key_secret_key(&definition.id))?
                        .is_some(),
                };
                Ok(ProviderCredentialStatus {
                    provider: definition.id.clone(),
                    display_name: definition.name.clone(),
                    api_key_policy: definition.api_key_policy,
                    api_key_configured,
                })
            })
            .collect()
    }

    pub fn set_api_key(
        &self,
        provider: &ProviderId,
        api_key: Vec<u8>,
    ) -> Result<(), ProviderCredentialError> {
        let definition = self.definition(provider)?;
        if definition.api_key_policy == ApiKeyPolicy::Unsupported {
            return Err(ProviderCredentialError::ApiKeyUnsupported);
        }
        validate_api_key(&api_key)?;
        self.secrets.store(
            &provider_api_key_secret_key(provider),
            &SecretValue::new(api_key),
        )?;
        Ok(())
    }

    pub(crate) fn request_headers(
        &self,
        provider: &ProviderId,
    ) -> Result<Vec<HttpHeader>, ProviderCredentialError> {
        let definition = self.definition(provider)?;
        if definition.api_key_policy == ApiKeyPolicy::Unsupported {
            return Ok(Vec::new());
        }
        let Some(secret) = self.secrets.load(&provider_api_key_secret_key(provider))? else {
            return match definition.api_key_policy {
                ApiKeyPolicy::Optional => Ok(Vec::new()),
                ApiKeyPolicy::Required => {
                    Err(ProviderCredentialError::ApiKeyMissing(provider.clone()))
                }
                ApiKeyPolicy::Unsupported => unreachable!("handled above"),
            };
        };
        validate_api_key(secret.expose())?;
        let value = std::str::from_utf8(secret.expose())
            .map_err(|_| ProviderCredentialError::InvalidStoredApiKey(provider.clone()))?;
        let header = match definition.api_key_header {
            ApiKeyHeader::Bearer => HttpHeader::new("Authorization", format!("Bearer {value}")),
            ApiKeyHeader::XApiKey => HttpHeader::new("x-api-key", value),
            ApiKeyHeader::XGoogApiKey => HttpHeader::new("x-goog-api-key", value),
        };
        Ok(vec![header])
    }

    fn definition(
        &self,
        provider: &ProviderId,
    ) -> Result<&ProviderDefinition, ProviderCredentialError> {
        self.providers
            .get(provider)
            .ok_or(ProviderCredentialError::UnknownProvider)
    }
}

#[derive(Debug)]
pub enum ProviderCredentialError {
    UnknownProvider,
    ApiKeyUnsupported,
    InvalidApiKey,
    ApiKeyMissing(ProviderId),
    InvalidStoredApiKey(ProviderId),
    SecretStore(SecretStoreError),
}

impl fmt::Display for ProviderCredentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProvider => formatter.write_str("unknown model provider"),
            Self::ApiKeyUnsupported => formatter.write_str("provider does not accept an API key"),
            Self::InvalidApiKey => formatter.write_str("API key is invalid"),
            Self::ApiKeyMissing(provider) => {
                write!(formatter, "no API key is stored for provider '{provider}'")
            }
            Self::InvalidStoredApiKey(provider) => {
                write!(
                    formatter,
                    "stored API key for provider '{provider}' is invalid"
                )
            }
            Self::SecretStore(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ProviderCredentialError {}

impl From<SecretStoreError> for ProviderCredentialError {
    fn from(error: SecretStoreError) -> Self {
        Self::SecretStore(error)
    }
}

fn validate_api_key(api_key: &[u8]) -> Result<(), ProviderCredentialError> {
    if api_key.is_empty()
        || api_key.len() > MAX_API_KEY_BYTES
        || api_key.iter().any(|byte| byte.is_ascii_control())
    {
        return Err(ProviderCredentialError::InvalidApiKey);
    }
    Ok(())
}
