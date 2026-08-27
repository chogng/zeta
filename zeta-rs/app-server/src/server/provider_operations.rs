use super::{AppServer, RpcError, decode, result};
use crate::provider_credentials::ProviderCredentialError;
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::provider::{
    ProviderApiKeySetParams, ProviderApiKeySetResult, ProviderListResult,
};
use zeta_model_provider_config::ProviderId;

impl AppServer {
    pub(super) fn provider_list(&self) -> Result<Value, RpcError> {
        let providers = self
            .provider_credentials
            .as_ref()
            .ok_or_else(provider_credentials_unavailable)?
            .list()
            .map_err(provider_credential_error)?;
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
        ProviderCredentialError::SecretStore(error) => {
            let _ = error;
            RpcError::new(
                -32094,
                AppServerErrorName::ProviderCredentialOperationFailed,
            )
        }
    }
}
