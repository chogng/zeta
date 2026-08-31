use super::ModelSelectionAction;
use super::model_choices;
use crate::components::list_selection::ListSelectionState;
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
fn model_region_marks_the_preference_and_maps_selection_to_slash_arguments() {
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

    let view = model_choices(&catalog, Some(&preferred_model));
    let state = ListSelectionState::new(view.model.into_body());

    assert_eq!(state.title(), "Model");
    assert_eq!(state.visible_items()[1].label(), "GPT Zeta ✓");
    assert_eq!(state.selected_visible_index(), Some(1));
    assert!(view.actions.values().any(|action| {
        action
            == &ModelSelectionAction::Select {
                preference: "openai/gpt-zeta".into(),
            }
    }));
}
