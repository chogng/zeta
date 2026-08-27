use super::config_view;
use super::provider_api_key_view;
use crate::components::selection::SelectionViewState;
use crate::features::config::ConfigSelectionAction;
use crate::features::config::TerminalSettings;
use crate::test_support::empty_config_snapshot;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_app_server_protocol::protocol::provider::{
    ProviderApiKeyPolicyDto, ProviderCatalogEntryDto, ProviderListResult,
};

fn providers() -> ProviderListResult {
    ProviderListResult {
        providers: vec![
            ProviderCatalogEntryDto {
                provider: "openai".into(),
                display_name: "OpenAI".into(),
                api_key_policy: ProviderApiKeyPolicyDto::Required,
                api_key_configured: false,
            },
            ProviderCatalogEntryDto {
                provider: "ollama".into(),
                display_name: "Ollama".into(),
                api_key_policy: ProviderApiKeyPolicyDto::Unsupported,
                api_key_configured: false,
            },
        ],
    }
}

#[test]
fn config_pane_organizes_the_snapshot_into_searchable_tabs() {
    let mut config = empty_config_snapshot();
    config.revision = 4;
    config.generation = 5;
    let providers = providers();
    let view = config_view(&config, &providers, TerminalSettings::default(), 7);
    let mut state = SelectionViewState::new(view.model.into_body());

    assert_eq!(state.title(), "Config");
    assert!(state.search().is_some());
    assert_eq!(
        state
            .tabs()
            .iter()
            .map(|tab| tab.label())
            .collect::<Vec<_>>(),
        vec!["Config", "Providers", "Language servers"]
    );
    let mouse = &state.visible_items()[0];
    assert_eq!(mouse.label(), "Mouse interactions");
    assert_eq!(
        mouse.description(),
        Some("Clicks and hover in interactive panes true")
    );
    assert!(matches!(
        view.actions.get(mouse.id().unwrap()).unwrap(),
        ConfigSelectionAction::SetMouseInteractions(edit)
            if edit.terminal.expected_revision == 7 && !edit.terminal.mouse_interactions
    ));

    let _ = state.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(state.visible_items().len(), 2);
    assert_eq!(state.visible_items()[0].label(), "OpenAI");
    assert_eq!(state.visible_items()[1].label(), "Ollama");
    assert!(matches!(
        view.actions
            .get(state.visible_items()[0].id().unwrap())
            .unwrap(),
        ConfigSelectionAction::OpenProviderApiKey { provider, .. } if provider == "openai"
    ));
    assert!(state.visible_items()[1].id().is_none());
}

#[test]
fn provider_api_key_input_is_masked_and_submits_a_provider_bound_action() {
    let view = provider_api_key_view("openai".into(), "OpenAI".into());
    let mut state = SelectionViewState::new(view.model.into_body());

    state.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    let outcome = state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL));

    assert!(state.search().unwrap().masked());
    assert!(matches!(
        outcome,
        crate::components::selection::SelectionInputOutcome::ActivateFreeForm { item_id, value }
            if value == "sk"
                && matches!(
                    view.actions.get(&item_id),
                    Some(ConfigSelectionAction::SetProviderApiKey { provider })
                        if provider == "openai"
                )
    ));
}
