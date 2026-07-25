use super::*;

#[test]
fn updates_survive_reopen() {
    let path = std::env::temp_dir().join(format!("zeta-config-{}.json", std::process::id()));
    let store = ConfigStore::open(&path).unwrap();
    store
        .update(ConfigUpdate {
            preferred_model: Some(Some(ModelRef::new(
                zeta_model_provider::ProviderId::new("openai").unwrap(),
                zeta_model_provider::ModelId::new("model").unwrap(),
            ))),
            theme: Some(Some(Theme::Dark)),
        })
        .unwrap();
    assert_eq!(
        ConfigStore::open(&path)
            .unwrap()
            .read()
            .unwrap()
            .preferred_model,
        Some(ModelRef::new(
            zeta_model_provider::ProviderId::new("openai").unwrap(),
            zeta_model_provider::ModelId::new("model").unwrap(),
        ))
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn model_provider_configuration_round_trips_without_secrets() {
    let path = std::env::temp_dir().join(format!(
        "zeta-model-provider-config-{}.json",
        std::process::id()
    ));
    let store = ConfigStore::open(&path).unwrap();
    let config = Config {
        preferred_model: Some(ModelRef::new(
            zeta_model_provider::ProviderId::new("openai").unwrap(),
            zeta_model_provider::ModelId::new("test-model").unwrap(),
        )),
        model_provider: Some(ModelProviderConfig {
            base_url: "https://example.test/v1".into(),
            credential_account: "test-account".into(),
            max_output_tokens: None,
        }),
        theme: None,
    };
    store.write_atomic(&config).unwrap();

    assert_eq!(ConfigStore::open(&path).unwrap().read().unwrap(), config);
    let persisted = std::fs::read_to_string(&path).unwrap();
    assert!(!persisted.contains("apiKey"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn anthropic_configuration_round_trips_without_secrets() {
    let path = std::env::temp_dir().join(format!(
        "zeta-anthropic-provider-config-{}.json",
        std::process::id()
    ));
    let store = ConfigStore::open(&path).unwrap();
    let config = Config {
        preferred_model: Some(ModelRef::new(
            zeta_model_provider::ProviderId::new("anthropic").unwrap(),
            zeta_model_provider::ModelId::new("claude-test").unwrap(),
        )),
        model_provider: Some(ModelProviderConfig {
            base_url: "https://api.anthropic.com".into(),
            credential_account: "anthropic-api-key".into(),
            max_output_tokens: Some(4096),
        }),
        theme: None,
    };
    store.write_atomic(&config).unwrap();

    assert_eq!(ConfigStore::open(&path).unwrap().read().unwrap(), config);
    let persisted = std::fs::read_to_string(&path).unwrap();
    assert!(!persisted.contains("apiKey"));
    let _ = std::fs::remove_file(path);
}
