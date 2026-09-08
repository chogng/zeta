use super::config_choices;
use super::provider_api_key_prompt;
use crate::config::ConfigSelectionAction;
use crate::config::TerminalSettings;
use crate::nls::Language;
use crate::status::StatusLineSettings;
use crate::test_support::empty_config_snapshot;
use crate::thread::composer::ChatInputMode;
use crate::widgets::list_selection::ListSelectionState;
use crate::widgets::text_prompt::TextPrompt;
use crate::widgets::text_prompt::TextPromptOutcome;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_app_server_protocol::protocol::config::LanguageServerConfigDto;
use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;
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
    assert_eq!(
        view.model.key_hints().text(),
        "Enter/Space to change  ·  / to search  ·  Esc to close"
    );
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
    assert!(
        state
            .visible_items()
            .iter()
            .all(|item| !matches!(
                item.label(),
                "Revision"
                    | "Generation"
                    | "Preferred model"
                    | "Approval review model"
                    | "Providers"
            ))
    );
    assert!(
        state
            .visible_items()
            .iter()
            .all(|item| item.label() != "Language servers")
    );
    let mouse = &state.visible_items()[0];
    assert_eq!(mouse.label(), "Enhanced TUI");
    assert_eq!(
        mouse.description(),
        Some("Click, scroll, hover, and auto-copy text in overlays only [ ✔ ]")
    );
    assert!(matches!(
        view.actions.get(mouse.id().unwrap()).unwrap(),
        ConfigSelectionAction::SetTerminalSettings(edit)
            if edit.server_config.revision == 4
                && !edit.terminal.mouse_interactions()
    ));
    let vim_mode = &state.visible_items()[1];
    assert_eq!(vim_mode.label(), "Vim mode");
    assert_eq!(
        vim_mode.description(),
        Some("Use Vim editing in ChatInput [   ]")
    );
    assert!(matches!(
        view.actions.get(vim_mode.id().unwrap()).unwrap(),
        ConfigSelectionAction::SetVimMode(edit)
            if edit.terminal.input_mode() == ChatInputMode::Vim
    ));
    let memory_diagnostics = &state.visible_items()[2];
    assert_eq!(memory_diagnostics.label(), "Memory diagnostics");
    assert_eq!(
        memory_diagnostics.description(),
        Some("Continuously collect bounded memory evidence [   ]")
    );
    assert!(matches!(
        view.actions
            .get(memory_diagnostics.id().unwrap())
            .unwrap(),
        ConfigSelectionAction::SetTerminalSettings(edit)
            if edit.terminal.memory_diagnostics()
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
    let language = &state.visible_items()[4];
    assert_eq!(language.label(), "Language");
    assert_eq!(
        language.description(),
        Some("Change the interface language English")
    );
    assert!(matches!(
        view.actions.get(language.id().unwrap()).unwrap(),
        ConfigSelectionAction::SetLanguage(edit)
            if edit.terminal.language() == Language::English
    ));

    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
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
        ConfigSelectionAction::OpenOpenAi(_)
    ));
    assert!(state.visible_items()[1].id().is_none());
}

#[test]
fn language_setting_cycles_with_activation_and_directional_keys() {
    let choices = || {
        config_choices(
            &empty_config_snapshot(),
            &providers(),
            TerminalSettings::default(),
            StatusLineSettings::default(),
        )
    };
    let mut editor = super::ConfigEditor::new(choices());
    for _ in 0..4 {
        editor.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }

    assert!(matches!(
        editor.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        super::ConfigEditorOutcome::Action(ConfigSelectionAction::SetLanguage(edit))
            if edit.terminal.language() == Language::Japanese
    ));

    let mut editor = super::ConfigEditor::new(choices());
    for _ in 0..4 {
        editor.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    assert!(matches!(
        editor.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
        super::ConfigEditorOutcome::Action(ConfigSelectionAction::SetLanguage(edit))
            if edit.terminal.language() == Language::Japanese
    ));

    let mut editor = super::ConfigEditor::new(choices());
    for _ in 0..4 {
        editor.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    assert!(matches!(
        editor.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        super::ConfigEditorOutcome::Action(ConfigSelectionAction::SetLanguage(edit))
            if edit.terminal.language() == Language::French
    ));
}

#[test]
fn config_root_uses_the_selected_language_through_nls() {
    let mut terminal = TerminalSettings::default();
    terminal.set_language(Language::Chinese);
    let view = config_choices(
        &empty_config_snapshot(),
        &providers(),
        terminal,
        StatusLineSettings::default(),
    );
    let state = ListSelectionState::new(view.model);

    assert_eq!(state.title(), "配置");
    assert_eq!(
        state
            .tabs()
            .iter()
            .map(|tab| tab.label())
            .collect::<Vec<_>>(),
        vec!["配置", "提供商", "语言服务器"]
    );
    assert_eq!(state.visible_items()[0].label(), "增强 TUI");
    assert_eq!(state.visible_items()[2].label(), "内存诊断");
    assert_eq!(state.visible_items()[4].label(), "语言");
    assert_eq!(
        state.visible_items()[4].description(),
        Some("切换界面语言 中文")
    );
}

#[test]
fn language_server_tab_exposes_one_switch_per_configured_server() {
    let mut config = empty_config_snapshot();
    config.revision = 7;
    config.language_servers.insert(
        "rust-analyzer".into(),
        LanguageServerConfigDto {
            mode: LanguageServerModeDto::Enabled,
            executable: None,
        },
    );
    config.language_servers.insert(
        "typescript-language-server".into(),
        LanguageServerConfigDto {
            mode: LanguageServerModeDto::Disabled,
            executable: Some("C:\\tools\\typescript-language-server.exe".into()),
        },
    );
    let view = config_choices(
        &config,
        &providers(),
        TerminalSettings::default(),
        StatusLineSettings::default(),
    );
    let mut state = ListSelectionState::new(view.model);

    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    let _ = state.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let _ = state.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

    assert_eq!(state.active_tab().label(), "Language servers");
    assert_eq!(state.visible_items().len(), 2);
    assert_eq!(state.visible_items()[0].label(), "rust-analyzer");
    assert_eq!(state.visible_items()[0].description(), Some(" [ ✔ ]"));
    assert!(matches!(
        view.actions
            .get(state.visible_items()[0].id().unwrap())
            .unwrap(),
        ConfigSelectionAction::SetLanguageServerMode(edit)
            if edit.expected_revision == 7
                && edit.server_id == "rust-analyzer"
                && edit.config.mode == LanguageServerModeDto::Disabled
                && edit.config.executable.is_none()
    ));
    assert_eq!(
        state.visible_items()[1].label(),
        "typescript-language-server"
    );
    assert_eq!(
        state.visible_items()[1].description(),
        Some("C:\\tools\\typescript-language-server.exe [   ]")
    );
    assert!(matches!(
        view.actions
            .get(state.visible_items()[1].id().unwrap())
            .unwrap(),
        ConfigSelectionAction::SetLanguageServerMode(edit)
            if edit.expected_revision == 7
                && edit.server_id == "typescript-language-server"
                && edit.config.mode == LanguageServerModeDto::Enabled
                && edit.config.executable.as_deref()
                    == Some("C:\\tools\\typescript-language-server.exe")
    ));
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
    let mut state = ListSelectionState::new(view.model);

    assert_eq!(
        state.visible_items()[0].description(),
        Some("Click, scroll, hover, and auto-copy text in overlays only [   ]")
    );
    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert!(state.search().unwrap().input_active());
    state.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
    assert_eq!(state.query(), "v");
    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(state.active_tab().label(), "Providers");
}

#[test]
fn config_option_arrows_and_tabs_toggle_values_without_switching_pages() {
    for key in [
        KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT),
    ] {
        let mut editor = super::ConfigEditor::new(config_choices(
            &empty_config_snapshot(),
            &providers(),
            TerminalSettings::default(),
            StatusLineSettings::default(),
        ));
        assert!(matches!(editor.handle_key(key),
            super::ConfigEditorOutcome::Action(ConfigSelectionAction::SetTerminalSettings(edit))
                if !edit.terminal.mouse_interactions()
        ));
    }
}

#[test]
fn config_editor_shows_a_checked_vim_mode_when_enabled() {
    let mut terminal = TerminalSettings::default();
    terminal.set_input_mode(ChatInputMode::Vim);

    let view = config_choices(
        &empty_config_snapshot(),
        &providers(),
        terminal,
        StatusLineSettings::default(),
    );
    let state = ListSelectionState::new(view.model);

    assert_eq!(
        state.visible_items()[1].description(),
        Some("Use Vim editing in ChatInput [ ✔ ]")
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

fn openai_editor() -> super::ConfigEditor {
    let mut editor = super::ConfigEditor::new(config_choices(
        &empty_config_snapshot(),
        &providers(),
        TerminalSettings::default(),
        StatusLineSettings::default(),
    ));
    for key in [
        KeyCode::Up,
        KeyCode::Up,
        KeyCode::Tab,
        KeyCode::Down,
        KeyCode::Down,
        KeyCode::Enter,
    ] {
        editor.handle_key(KeyEvent::new(key, KeyModifiers::NONE));
    }
    editor
}

#[test]
fn openai_form_owns_keyboard_input_and_esc_returns_to_providers() {
    let mut editor = openai_editor();
    assert!(matches!(editor.page(), super::ConfigEditorPage::OpenAi(_)));
    editor.handle_paste("unconfirmed-key".into());
    for _ in 0..3 {
        editor.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    }
    assert_eq!(
        editor.selection().unwrap().active_tab().label(),
        "Providers"
    );
}

#[test]
fn created_provider_remains_available_after_closing_and_reopening_openai() {
    let mut editor = openai_editor();
    editor.openai.as_mut().unwrap().select_tab(2);
    for value in ["Example", "https://example.test/v1", ""] {
        editor.handle_paste(value.into());
        editor.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    }
    editor.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let super::ConfigEditorOutcome::Action(ConfigSelectionAction::Connection(request)) =
        editor.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("expected creation")
    };
    let mut config = empty_config_snapshot();
    config.revision = 1;
    config
        .providers
        .insert(request.config.provider.clone(), request.config.clone());
    editor.complete_connection(crate::config::openai::Reply {
        id: request.id,
        result: Ok((
            config_choices(
                &config,
                &providers(),
                TerminalSettings::default(),
                StatusLineSettings::default(),
            ),
            None,
        )),
    });
    for code in [KeyCode::Esc, KeyCode::Esc, KeyCode::Enter] {
        editor.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }
    assert!(matches!(editor.page(), super::ConfigEditorPage::OpenAi(_)));
    for _ in 0..2 {
        editor.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT));
    }
    editor.handle_paste(" renamed".into());
    let super::ConfigEditorOutcome::Action(ConfigSelectionAction::Connection(renamed)) =
        editor.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("expected existing connection edit")
    };
    assert_eq!(renamed.config.provider, request.config.provider);
    assert_eq!(renamed.revision, 1);
}
