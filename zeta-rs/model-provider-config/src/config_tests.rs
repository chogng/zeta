use super::*;
use serde_json::json;

fn provider_id(value: &str) -> ProviderId {
    ProviderId::new(value).unwrap()
}

fn definition(id: &str, endpoint: EndpointPolicy) -> ProviderDefinition {
    ProviderDefinition::new(
        provider_id(id),
        format!("{id} provider"),
        ProviderAdapter::OpenAiCompatible,
        ApiProfile::OpenAiChatCompletions,
        endpoint,
        ModelCatalogPolicy::AllowUnlisted,
    )
}

#[test]
fn model_provider_config_is_serializable_and_has_a_schema() {
    let config = ModelProviderConfig {
        provider: provider_id("openai"),
        base_url: Some("https://example.test/v1".into()),
        max_output_tokens: Some(2048),
    };

    let value = serde_json::to_value(&config).unwrap();
    assert_eq!(
        value,
        json!({
            "provider": "openai",
            "baseUrl": "https://example.test/v1",
            "maxOutputTokens": 2048
        })
    );
    assert_eq!(
        serde_json::from_value::<ModelProviderConfig>(value).unwrap(),
        config
    );

    let schema = serde_json::to_value(model_provider_config_schema()).unwrap();
    assert!(schema["properties"]["provider"].is_object());
    assert!(schema["properties"]["baseUrl"].is_object());
}

#[test]
fn registry_applies_endpoint_and_token_defaults_during_normalization() {
    let registry = ProviderConfigRegistry::from_definitions([definition(
        "custom",
        EndpointPolicy::ProviderDefault {
            base_url: " https://example.test/v1/// ".into(),
        },
    )
    .with_defaults(ProviderDefaults {
        max_output_tokens: Some(1024),
    })])
    .unwrap();

    let normalized = registry
        .normalize(&ModelProviderConfig::new(provider_id("custom")))
        .unwrap();

    assert_eq!(normalized.base_url, "https://example.test/v1");
    assert_eq!(normalized.max_output_tokens, Some(1024));
    assert_eq!(normalized.api_profile, ApiProfile::OpenAiChatCompletions);
}

#[test]
fn configured_endpoint_is_required_and_overrides_are_normalized() {
    let registry = ProviderConfigRegistry::from_definitions([definition(
        "custom",
        EndpointPolicy::ConfiguredOnly,
    )])
    .unwrap();
    assert_eq!(
        registry
            .normalize(&ModelProviderConfig::new(provider_id("custom")))
            .unwrap_err(),
        ProviderConfigError::MissingBaseUrl(provider_id("custom"))
    );

    let normalized = registry
        .normalize(&ModelProviderConfig {
            provider: provider_id("custom"),
            base_url: Some(" https://runtime.test/v1/ ".into()),
            max_output_tokens: Some(512),
        })
        .unwrap();
    assert_eq!(normalized.base_url, "https://runtime.test/v1");
}

#[test]
fn static_validation_rejects_invalid_urls_and_zero_token_limits() {
    let invalid_url = ModelProviderConfig {
        provider: provider_id("custom"),
        base_url: Some("file:///tmp/provider".into()),
        max_output_tokens: None,
    };
    assert!(matches!(
        invalid_url.validate_static(),
        Err(ProviderConfigError::InvalidBaseUrl { .. })
    ));

    let invalid_tokens = ModelProviderConfig {
        provider: provider_id("custom"),
        base_url: None,
        max_output_tokens: Some(0),
    };
    assert_eq!(
        invalid_tokens.validate_static().unwrap_err(),
        ProviderConfigError::InvalidMaxOutputTokens(provider_id("custom"))
    );
}

#[test]
fn registry_merge_has_explicit_conflict_semantics() {
    let mut registry = ProviderConfigRegistry::from_definitions([definition(
        "custom",
        EndpointPolicy::ConfiguredOnly,
    )])
    .unwrap();
    let replacement = definition(
        "custom",
        EndpointPolicy::ProviderDefault {
            base_url: "https://replacement.test/v1".into(),
        },
    );

    assert_eq!(
        registry
            .merge(
                ProviderConfigRegistry::from_definitions([replacement.clone()]).unwrap(),
                RegistryMergePolicy::RejectConflicts,
            )
            .unwrap_err(),
        ProviderConfigError::DuplicateProvider(provider_id("custom"))
    );
    assert_eq!(
        registry.get(&provider_id("custom")).unwrap().endpoint,
        EndpointPolicy::ConfiguredOnly
    );

    registry
        .merge(
            ProviderConfigRegistry::from_definitions([replacement]).unwrap(),
            RegistryMergePolicy::ReplaceExisting,
        )
        .unwrap();
    assert!(matches!(
        registry.get(&provider_id("custom")).unwrap().endpoint,
        EndpointPolicy::ProviderDefault { .. }
    ));
}

#[test]
fn builtins_are_valid_and_include_all_supported_adapters() {
    let registry = ProviderConfigRegistry::builtin();
    assert_eq!(registry.providers().count(), 13);
    assert_eq!(
        registry.get(&provider_id("openai")).unwrap().adapter,
        ProviderAdapter::OpenAi
    );
    assert_eq!(
        registry.get(&provider_id("openai")).unwrap().api_profile,
        ApiProfile::OpenAiResponses
    );
    assert_eq!(
        registry
            .normalize(&ModelProviderConfig::new(provider_id("anthropic")))
            .unwrap()
            .max_output_tokens,
        Some(1024)
    );
}

#[test]
fn normalization_rejects_a_selected_provider_mismatch() {
    let registry = ProviderConfigRegistry::builtin();
    let error = registry
        .normalize_for(
            &ModelProviderConfig::new(provider_id("openai")),
            &provider_id("anthropic"),
        )
        .unwrap_err();

    assert_eq!(
        error,
        ProviderConfigError::ProviderMismatch {
            configured: provider_id("openai"),
            selected: provider_id("anthropic"),
        }
    );
}
