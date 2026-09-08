use super::ModelSelectionAction;
use super::model_choices;
use crate::widgets::list_selection::ListSelectionState;
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
fn model_picker_shows_names_only_and_keeps_selection_identity_and_pin_state() {
    let catalog = ModelListResult {
        models: vec![ModelCatalogEntry {
            model: ModelRef::new(
                ProviderId::new("openai").unwrap(),
                ModelId::new("gpt-zeta").unwrap(),
            ),
            display_name: "GPT Zeta".into(),
            access: ModelAccess::Unknown,
            output_transport: ModelOutputTransport::Unary,
            context_window: None,
            auto_compact_token_limit: None,
            available_context_window: None,
            capabilities: ModelCapabilities::UNKNOWN,
            supported_reasoning_efforts: Vec::new(),
            default_reasoning_effort: None,
            default_personality: None,
        }],
    };
    let preferred_model = ModelRefDto {
        provider: "openai".into(),
        model: "gpt-zeta".into(),
    };

    let mut config = crate::test_support::empty_config_snapshot();
    config.preferred_model = Some(preferred_model.clone());
    config
        .tui
        .0
        .insert("pinnedModels".into(), serde_json::json!([preferred_model]));
    let view = model_choices(
        &catalog,
        &config,
        &zeta_app_server_protocol::protocol::provider::ProviderListResult { providers: vec![] },
    )
    .unwrap();
    let state = ListSelectionState::new(view.model);

    assert_eq!(state.title(), "Model");
    assert!(state.search().is_none());
    assert_eq!(state.visible_items()[0].label(), "GPT Zeta");
    assert_eq!(state.visible_items()[0].description(), None);
    assert_eq!(state.selected_visible_index(), Some(0));
    assert!(view.actions.values().any(|action| {
        action
            == &ModelSelectionAction::Select {
                preference: "openai/gpt-zeta".into(),
                pinned: true,
            }
    }));
}

#[test]
fn malformed_or_duplicate_pins_are_rejected() {
    let mut tui = zeta_app_server_protocol::protocol::config::FrontendConfigDto::default();
    for value in [
        serde_json::json!("bad"),
        serde_json::json!([{"provider":"openai","model":"gpt-x"},{"provider":"openai","model":"gpt-x"}]),
    ] {
        tui.0.insert("pinnedModels".into(), value);
        assert!(super::pinned_models(&tui).is_err());
    }
}
