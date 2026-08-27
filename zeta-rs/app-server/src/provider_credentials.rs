use std::sync::Arc;
use zeta_app_server_protocol::protocol::provider::{
    ProviderApiKeyPolicyDto, ProviderCatalogEntryDto,
};
use zeta_model_provider::provider_api_key_secret_key;
use zeta_model_provider_config::{ApiKeyPolicy, ProviderConfigRegistry, ProviderId};
use zeta_secrets::{SecretStore, SecretStoreError, SecretValue};

pub(crate) struct ProviderCredentialService {
    providers: ProviderConfigRegistry,
    secrets: Arc<dyn SecretStore>,
}

impl ProviderCredentialService {
    pub(crate) fn new(providers: ProviderConfigRegistry, secrets: Arc<dyn SecretStore>) -> Self {
        Self { providers, secrets }
    }

    pub(crate) fn list(&self) -> Result<Vec<ProviderCatalogEntryDto>, ProviderCredentialError> {
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
                Ok(ProviderCatalogEntryDto {
                    provider: definition.id.to_string(),
                    display_name: definition.name.clone(),
                    api_key_policy: api_key_policy_dto(definition.api_key_policy),
                    api_key_configured,
                })
            })
            .collect()
    }

    pub(crate) fn set_api_key(
        &self,
        provider: &ProviderId,
        api_key: Vec<u8>,
    ) -> Result<(), ProviderCredentialError> {
        let definition = self
            .providers
            .get(provider)
            .ok_or(ProviderCredentialError::UnknownProvider)?;
        if definition.api_key_policy == ApiKeyPolicy::Unsupported {
            return Err(ProviderCredentialError::ApiKeyUnsupported);
        }
        if api_key.is_empty()
            || api_key.len() > 16 * 1024
            || api_key.iter().any(|byte| byte.is_ascii_control())
        {
            return Err(ProviderCredentialError::InvalidApiKey);
        }
        self.secrets.store(
            &provider_api_key_secret_key(provider),
            &SecretValue::new(api_key),
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum ProviderCredentialError {
    UnknownProvider,
    ApiKeyUnsupported,
    InvalidApiKey,
    SecretStore(SecretStoreError),
}

impl From<SecretStoreError> for ProviderCredentialError {
    fn from(error: SecretStoreError) -> Self {
        Self::SecretStore(error)
    }
}

fn api_key_policy_dto(policy: ApiKeyPolicy) -> ProviderApiKeyPolicyDto {
    match policy {
        ApiKeyPolicy::Unsupported => ProviderApiKeyPolicyDto::Unsupported,
        ApiKeyPolicy::Optional => ProviderApiKeyPolicyDto::Optional,
        ApiKeyPolicy::Required => ProviderApiKeyPolicyDto::Required,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeta_secrets::MemorySecretStore;

    #[test]
    fn builtin_catalog_projects_every_provider_and_api_key_status_without_values() {
        let secrets = Arc::new(MemorySecretStore::default());
        let openai = ProviderId::new("openai").unwrap();
        secrets
            .store(
                &provider_api_key_secret_key(&openai),
                &SecretValue::new(b"secret-openai-key".to_vec()),
            )
            .unwrap();
        let service = ProviderCredentialService::new(ProviderConfigRegistry::builtin(), secrets);

        let providers = service.list().unwrap();

        assert_eq!(providers.len(), 13);
        assert!(providers.iter().any(|provider| {
            provider.provider == "openai"
                && provider.api_key_policy == ProviderApiKeyPolicyDto::Required
                && provider.api_key_configured
        }));
        assert!(providers.iter().any(|provider| {
            provider.provider == "ollama"
                && provider.api_key_policy == ProviderApiKeyPolicyDto::Unsupported
                && !provider.api_key_configured
        }));
    }

    #[test]
    fn api_key_mutation_rejects_local_provider_and_stores_an_exact_supported_key() {
        let secrets = Arc::new(MemorySecretStore::default());
        let service =
            ProviderCredentialService::new(ProviderConfigRegistry::builtin(), secrets.clone());
        let openai = ProviderId::new("openai").unwrap();

        service
            .set_api_key(&openai, b"stored-key".to_vec())
            .unwrap();

        assert_eq!(
            secrets
                .load(&provider_api_key_secret_key(&openai))
                .unwrap()
                .unwrap()
                .expose(),
            b"stored-key"
        );
        assert!(matches!(
            service.set_api_key(&ProviderId::new("ollama").unwrap(), b"key".to_vec()),
            Err(ProviderCredentialError::ApiKeyUnsupported)
        ));
    }
}
