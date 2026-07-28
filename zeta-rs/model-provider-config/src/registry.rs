use crate::config::{is_http_url, normalize_base_url};
use crate::{
    ApprovalReviewModelDefault, EndpointPolicy, ModelCatalogPolicy, ModelProviderConfig,
    NormalizedModelProviderConfig, ProviderConfigError, ProviderDefinition, ProviderId, providers,
};
use std::collections::BTreeMap;
use zeta_protocol::ModelRef;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryMergePolicy {
    RejectConflicts,
    ReplaceExisting,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProviderConfigRegistry {
    providers: BTreeMap<ProviderId, ProviderDefinition>,
}

impl ProviderConfigRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builtin() -> Self {
        Self::from_definitions(providers::builtin())
            .expect("built-in provider definitions must be valid and unique")
    }

    pub fn from_definitions(
        definitions: impl IntoIterator<Item = ProviderDefinition>,
    ) -> Result<Self, ProviderConfigError> {
        let mut registry = Self::new();
        for definition in definitions {
            registry.register(definition)?;
        }
        Ok(registry)
    }

    pub fn register(&mut self, definition: ProviderDefinition) -> Result<(), ProviderConfigError> {
        definition.validate()?;
        if self.providers.contains_key(&definition.id) {
            return Err(ProviderConfigError::DuplicateProvider(
                definition.id.clone(),
            ));
        }
        self.providers.insert(definition.id.clone(), definition);
        Ok(())
    }

    pub fn merge(
        &mut self,
        incoming: Self,
        policy: RegistryMergePolicy,
    ) -> Result<(), ProviderConfigError> {
        for definition in incoming.providers.values() {
            definition.validate()?;
            if policy == RegistryMergePolicy::RejectConflicts
                && self.providers.contains_key(&definition.id)
            {
                return Err(ProviderConfigError::DuplicateProvider(
                    definition.id.clone(),
                ));
            }
        }
        self.providers.extend(incoming.providers);
        Ok(())
    }

    pub fn get(&self, provider: &ProviderId) -> Option<&ProviderDefinition> {
        self.providers.get(provider)
    }

    pub fn providers(&self) -> impl Iterator<Item = &ProviderDefinition> {
        self.providers.values()
    }

    /// Selects the provider-owned automatic approval-review model for an active Agent model.
    ///
    /// Providers may name a dedicated review default. Providers without one reuse the active
    /// model, which keeps custom and local providers usable without inventing a model identifier.
    pub fn automatic_approval_review_model(
        &self,
        active_model: &ModelRef,
    ) -> Result<ModelRef, ProviderConfigError> {
        let definition = self
            .get(&active_model.provider)
            .ok_or_else(|| ProviderConfigError::UnknownProvider(active_model.provider.clone()))?;
        let model = match &definition.defaults.approval_review_model {
            ApprovalReviewModelDefault::ActiveModel => active_model.model.clone(),
            ApprovalReviewModelDefault::Model { model } => model.clone(),
        };
        Ok(ModelRef::new(active_model.provider.clone(), model))
    }

    /// Validates the part of model availability represented by the provider's static catalog.
    ///
    /// Providers that allow unlisted model IDs still require runtime validation because account
    /// entitlement and remote availability cannot be proven from local configuration.
    pub fn validate_model_selection(&self, model: &ModelRef) -> Result<(), ProviderConfigError> {
        let definition = self
            .get(&model.provider)
            .ok_or_else(|| ProviderConfigError::UnknownProvider(model.provider.clone()))?;
        if definition.model_catalog_policy == ModelCatalogPolicy::ListedOnly
            && !definition
                .models
                .iter()
                .any(|candidate| candidate.id == model.model)
        {
            return Err(ProviderConfigError::ModelNotRegistered {
                provider: model.provider.clone(),
                model: model.model.clone(),
            });
        }
        Ok(())
    }

    pub fn normalize(
        &self,
        config: &ModelProviderConfig,
    ) -> Result<NormalizedModelProviderConfig, ProviderConfigError> {
        config.validate_static()?;
        let definition = self
            .get(&config.provider)
            .ok_or_else(|| ProviderConfigError::UnknownProvider(config.provider.clone()))?;
        let configured_base_url = config
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|base_url| !base_url.is_empty());
        let base_url = match (configured_base_url, &definition.endpoint) {
            (Some(base_url), _) => base_url,
            (None, EndpointPolicy::ProviderDefault { base_url }) => base_url,
            (None, EndpointPolicy::ConfiguredOnly) => {
                return Err(ProviderConfigError::MissingBaseUrl(config.provider.clone()));
            }
        };
        let base_url = normalize_base_url(base_url, definition.base_url_normalization);
        if !is_http_url(&base_url) {
            return Err(ProviderConfigError::InvalidBaseUrl {
                provider: config.provider.clone(),
                base_url,
            });
        }
        Ok(NormalizedModelProviderConfig {
            provider: config.provider.clone(),
            api_profile: definition.api_profile,
            base_url,
            max_output_tokens: config
                .max_output_tokens
                .or(definition.defaults.max_output_tokens),
        })
    }

    pub fn normalize_for(
        &self,
        config: &ModelProviderConfig,
        selected_provider: &ProviderId,
    ) -> Result<NormalizedModelProviderConfig, ProviderConfigError> {
        if &config.provider != selected_provider {
            return Err(ProviderConfigError::ProviderMismatch {
                configured: config.provider.clone(),
                selected: selected_provider.clone(),
            });
        }
        self.normalize(config)
    }
}
