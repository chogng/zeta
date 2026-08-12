use super::ModelSelectionAction;
use super::model_selection_view;
use crate::components::selection::SelectionViewState;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::model::ModelCatalogEntry;
use zeta_app_server_protocol::protocol::model::ModelListResult;
use zeta_protocol::ModelId;
use zeta_protocol::ModelRef;
use zeta_protocol::ProviderId;

#[test]
fn model_pane_marks_the_preference_and_maps_selection_to_slash_arguments() {
    let catalog = ModelListResult {
        models: vec![ModelCatalogEntry {
            model: ModelRef::new(
                ProviderId::new("openai").unwrap(),
                ModelId::new("gpt-zeta").unwrap(),
            ),
            display_name: "GPT Zeta".into(),
        }],
    };
    let preferred_model = ModelRefDto {
        provider: "openai".into(),
        model: "gpt-zeta".into(),
    };

    let view = model_selection_view(&catalog, Some(&preferred_model));
    let state = SelectionViewState::new(view.model.into_body());

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
