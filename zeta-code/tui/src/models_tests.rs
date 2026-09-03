use super::ModelSummary;
use super::access_label;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::model::ModelCatalogEntry;
use zeta_app_server_protocol::protocol::model::ModelListResult;
use zeta_protocol::ModelAccess;
use zeta_protocol::ModelCapabilities;
use zeta_protocol::ModelId;
use zeta_protocol::ModelOutputTransport;
use zeta_protocol::ModelRef;
use zeta_protocol::ProviderId;

#[test]
fn model_summary_resolves_the_selected_models_access_path() {
    let preferred = ModelRefDto {
        provider: "openai-chatgpt".into(),
        model: "gpt-5.6".into(),
    };
    let catalog = ModelListResult {
        models: vec![entry(
            "openai-chatgpt",
            "gpt-5.6",
            ModelAccess::Subscription,
        )],
    };

    let summary = ModelSummary::from_catalog(Some(preferred), Some(&catalog));

    assert_eq!(summary.model_label(), "openai-chatgpt/gpt-5.6");
    assert_eq!(summary.access(), ModelAccess::Subscription);
    assert_eq!(access_label(summary.access()), "Subscription");
}

#[test]
fn missing_or_automatic_models_are_reported_without_guessing_access() {
    let configured = ModelSummary::from_catalog(
        Some(ModelRefDto {
            provider: "custom".into(),
            model: "unknown".into(),
        }),
        None,
    );
    let automatic = ModelSummary::from_catalog(None, None);

    assert_eq!(configured.access(), ModelAccess::Unknown);
    assert_eq!(access_label(configured.access()), "Access unknown");
    assert_eq!(automatic.model_label(), "Automatic model");
}

fn entry(provider: &str, model: &str, access: ModelAccess) -> ModelCatalogEntry {
    ModelCatalogEntry {
        model: ModelRef::new(
            ProviderId::new(provider).unwrap(),
            ModelId::new(model).unwrap(),
        ),
        display_name: model.into(),
        access,
        output_transport: ModelOutputTransport::Unary,
        context_window: None,
        auto_compact_token_limit: None,
        available_context_window: None,
        capabilities: ModelCapabilities::UNKNOWN,
        supported_reasoning_efforts: Vec::new(),
        default_reasoning_effort: None,
        default_personality: None,
    }
}
