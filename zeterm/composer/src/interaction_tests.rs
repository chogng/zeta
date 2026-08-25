use super::ComposerInteractionActivation;
use super::ComposerInteractionModel;
use super::ComposerModelOption;
use super::SelectionDirection;
use crate::ComposerRoute;
use zeta_protocol::{ModelId, ModelRef, ProviderId};

fn model_option(provider: &str, model: &str, display_name: &str) -> ComposerModelOption {
    let model = ModelRef::new(
        ProviderId::new(provider).unwrap(),
        ModelId::new(model).unwrap(),
    );
    ComposerModelOption {
        description: format!("{}/{}", model.provider, model.model),
        label: display_name.into(),
        model,
    }
}

#[test]
fn slash_model_pushes_model_picker_and_escape_returns_to_commands() {
    let mut model = ComposerInteractionModel::new();
    model
        .set_catalog(Vec::new(), vec![model_option("openai", "gpt", "GPT")])
        .unwrap();
    model.sync_for_composer("/model", ComposerRoute::Agent);

    assert_eq!(
        model.activate_selected(),
        Some(ComposerInteractionActivation::ViewChanged)
    );
    assert!(model.is_model_picker_visible());
    assert!(model.dismiss("/model"));
    assert!(!model.is_model_picker_visible());
    assert!(model.is_visible());
}

#[test]
fn model_activation_returns_exact_catalog_identity_and_closes() {
    let mut model = ComposerInteractionModel::new();
    let expected = model_option("anthropic", "sonnet", "Sonnet");
    model
        .set_catalog(Vec::new(), vec![expected.clone()])
        .unwrap();
    model.sync_for_composer("/model", ComposerRoute::Agent);
    model.activate_selected();

    assert_eq!(
        model.activate_selected(),
        Some(ComposerInteractionActivation::Model(expected.model))
    );
    assert!(!model.is_visible());
}

#[test]
fn slash_filter_and_keyboard_selection_share_one_visible_list() {
    let mut model = ComposerInteractionModel::new();
    model.sync_for_composer("/mo", ComposerRoute::Agent);
    let view = model.view().unwrap();
    assert_eq!(view.items().len(), 1);
    assert_eq!(view.items()[0].label(), "/model");

    model.move_selection(SelectionDirection::Next);
    assert_eq!(model.view().unwrap().selected(), 0);
}

#[test]
fn dismissed_slash_view_stays_closed_until_composer_text_changes() {
    let mut model = ComposerInteractionModel::new();
    model.sync_for_composer("/", ComposerRoute::Agent);
    assert!(model.dismiss("/"));
    model.sync_for_composer("/", ComposerRoute::Agent);
    assert!(!model.is_visible());

    model.sync_for_composer("/m", ComposerRoute::Agent);
    assert!(model.is_visible());
}

#[test]
fn shell_route_closes_agent_interactions() {
    let mut model = ComposerInteractionModel::new();
    model.sync_for_composer("/m", ComposerRoute::Agent);
    assert!(model.is_visible());

    model.sync_for_composer("echo done", ComposerRoute::Shell);

    assert!(!model.is_visible());
}
