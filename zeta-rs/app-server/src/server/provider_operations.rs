use super::AppServer;
use super::RpcError;
use super::decode;
use super::result;
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::provider::ProviderApiKeyPolicyDto;
use zeta_app_server_protocol::protocol::provider::ProviderApiKeySetParams;
use zeta_app_server_protocol::protocol::provider::ProviderApiKeySetResult;
use zeta_app_server_protocol::protocol::provider::ProviderCatalogEntryDto;
use zeta_app_server_protocol::protocol::provider::ProviderListResult;
use zeta_model_provider::ProviderCredentialError;
use zeta_model_provider_config::ApiKeyPolicy;
use zeta_model_provider_config::ProviderId;

impl AppServer {
    pub(super) fn provider_list(&self) -> Result<Value, RpcError> {
        let providers = self
            .provider_credentials
            .as_ref()
            .ok_or_else(provider_credentials_unavailable)?
            .catalog()
            .map_err(provider_credential_error)?;
        let providers = providers
            .into_iter()
            .map(|provider| ProviderCatalogEntryDto {
                provider: provider.provider.to_string(),
                display_name: provider.display_name,
                api_key_policy: api_key_policy_dto(provider.api_key_policy),
                api_key_configured: provider.api_key_configured,
            })
            .collect();
        result(&ProviderListResult { providers })
    }

    pub(super) fn provider_api_key_set(&self, params: Value) -> Result<Value, RpcError> {
        let params: ProviderApiKeySetParams = decode(&params)?;
        let provider = ProviderId::new(params.provider)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        self.provider_credentials
            .as_ref()
            .ok_or_else(provider_credentials_unavailable)?
            .set_api_key(&provider, params.api_key.into_bytes())
            .map_err(provider_credential_error)?;
        result(&ProviderApiKeySetResult {
            provider: provider.to_string(),
            api_key_configured: true,
        })
    }
}

fn provider_credentials_unavailable() -> RpcError {
    RpcError::new(-32093, AppServerErrorName::ProviderCredentialsUnavailable)
}

fn provider_credential_error(error: ProviderCredentialError) -> RpcError {
    match error {
        ProviderCredentialError::UnknownProvider
        | ProviderCredentialError::ApiKeyUnsupported
        | ProviderCredentialError::InvalidApiKey => {
            RpcError::new(-32602, AppServerErrorName::InvalidParams)
        }
        ProviderCredentialError::SecretStore(_) => RpcError::new(
            -32094,
            AppServerErrorName::ProviderCredentialOperationFailed,
        ),
        ProviderCredentialError::ApiKeyMissing(_)
        | ProviderCredentialError::InvalidStoredApiKey(_) => RpcError::new(
            -32094,
            AppServerErrorName::ProviderCredentialOperationFailed,
        ),
    }
}

fn api_key_policy_dto(policy: ApiKeyPolicy) -> ProviderApiKeyPolicyDto {
    match policy {
        ApiKeyPolicy::Unsupported => ProviderApiKeyPolicyDto::Unsupported,
        ApiKeyPolicy::Optional => ProviderApiKeyPolicyDto::Optional,
        ApiKeyPolicy::Required => ProviderApiKeyPolicyDto::Required,
    }
}
