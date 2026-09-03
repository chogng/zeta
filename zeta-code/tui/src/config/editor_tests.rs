use super::config_choices;
use super::provider_api_key_prompt;
use crate::config::ConfigSelectionAction;
use crate::config::FollowUpMode;
use crate::config::TerminalSettings;
use crate::status::StatusLineSettings;
use crate::test_support::empty_config_snapshot;
use crate::thread::composer::ChatInputMode;
use crate::widgets::list_selection::ListSelectionState;
use crate::widgets::text_prompt::TextPrompt;
use crate::widgets::text_prompt::TextPromptOutcome;
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
fn config_editor_organizes_the_snapshot_into_searchable_tabs() {
    let mut config = empty_config_snapshot();
    config.revision = 4;
    config.generation = 5;
    let providers = providers();
    let view = config_choices(
        &config,
        &providers,
        TerminalSettings::default(),
        StatusLineSettings::default(),
    );
    assert_eq!(view.model.key_hints().text(), "Enter/Space to change");
    let mut state = ListSelectionState::new(view.model);

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
        Some("Select and auto-copy text, click, and hover [ ✔ ]")
    );
    assert!(matches!(
        view.actions.get(mouse.id().unwrap()).unwrap(),
        ConfigSelectionAction::SetTerminalSettings(edit)
            if edit.server_config.revision == 4
                && !edit.terminal.mouse_interactions()
    ));
    let input_mode = &state.visible_items()[2];
    assert_eq!(input_mode.label(), "Input mode");
    assert_eq!(
        input_mode.description(),
        Some("Standard or Vim editing inside ChatInput Standard")
    );
    assert!(matches!(
        view.actions.get(input_mode.id().unwrap()).unwrap(),
        ConfigSelectionAction::ChooseInputMode { standard, vim }
            if standard.terminal.input_mode() == ChatInputMode::Standard
                && vim.terminal.input_mode() == ChatInputMode::Vim
    ));
    let git_changes = &state.visible_items()[3];
    assert_eq!(git_changes.label(), "Show Git changes as diff");
    assert_eq!(
        git_changes.description(),
        Some("Show added and deleted lines instead of changed files [   ]")
    );
    assert!(matches!(
        view.actions.get(git_changes.id().unwrap()).unwrap(),
        ConfigSelectionAction::SetShowGitChangesAsDiff(edit)
            if edit.status_line.show_git_changes_as_diff()
    ));
    let follow_up = &state.visible_items()[1];
    assert_eq!(follow_up.label(), "Follow-up messages");
    assert_eq!(
        follow_up.description(),
        Some("How Enter sends a message while a Turn is running Queue")
    );
    assert!(matches!(
        view.actions.get(follow_up.id().unwrap()).unwrap(),
        ConfigSelectionAction::ChooseFollowUpMode { queue, steer }
            if queue.server_config.revision == 4
                && queue.terminal.follow_up_mode() == FollowUpMode::Queue
                && steer.terminal.follow_up_mode() == FollowUpMode::Steer
    ));

    let _ = state.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(state.visible_items().len(), 2);
    assert_eq!(state.visible_items()[0].label(), "OpenAI");
    assert_eq!(state.visible_items()[1].label(), "Ollama");
    assert!(
        state
            .visible_items()
            .iter()
            .all(|item| item.description().is_none())
    );
    assert!(matches!(
        view.actions
            .get(state.visible_items()[0].id().unwrap())
            .unwrap(),
        ConfigSelectionAction::OpenProviderApiKey { provider, .. } if provider == "openai"
    ));
    assert!(state.visible_items()[1].id().is_none());
}

#[test]
fn config_editor_uses_an_empty_unicode_checkbox_when_mouse_interactions_are_disabled() {
    let mut terminal = TerminalSettings::default();
    terminal.set_mouse_interactions(false);

    let view = config_choices(
        &empty_config_snapshot(),
        &providers(),
        terminal,
        StatusLineSettings::default(),
    );
    let state = ListSelectionState::new(view.model);

    assert_eq!(
        state.visible_items()[0].description(),
        Some("Select and auto-copy text, click, and hover [   ]")
    );
}

#[test]
fn provider_api_key_input_is_masked_keeps_its_explanation_and_submits_with_enter() {
    let prompt = provider_api_key_prompt("openai".into(), "OpenAI".into());
    let mut state = TextPrompt::new(prompt.spec);

    state.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    let outcome = state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(state.input().masked());
    assert_eq!(
        state.explanation(),
        "The key is hidden and stored in the profile secret store"
    );
    assert!(matches!(
        outcome,
        TextPromptOutcome::Submit(value) if value == "sk"
    ));
    assert_eq!(prompt.provider, "openai");
}
