use super::config_pane_spec;
use super::provider_api_key_prompt;
use crate::components::list_selection::ListSelectionState;
use crate::components::text_prompt::TextPrompt;
use crate::components::text_prompt::TextPromptOutcome;
use crate::features::config::ConfigSelectionAction;
use crate::features::config::TerminalSettings;
use crate::test_support::empty_config_snapshot;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_app_server_protocol::protocol::provider::{
    ProviderApiKeyPolicyDto, ProviderCatalogEntryDto, ProviderListResult,
};
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryPermissionDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustStateDto;
use zeta_protocol::SessionId;

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

fn session_id() -> SessionId {
    SessionId::new("config-session").unwrap()
}

fn no_additional_directories() -> WorkspaceAdditionalDirectoryListResult {
    WorkspaceAdditionalDirectoryListResult {
        revision: 0,
        directories: Vec::new(),
    }
}

#[test]
fn config_pane_organizes_the_snapshot_into_searchable_tabs() {
    let mut config = empty_config_snapshot();
    config.revision = 4;
    config.generation = 5;
    let providers = providers();
    let view = config_pane_spec(
        &config,
        &providers,
        TerminalSettings::default(),
        7,
        &session_id(),
        &no_additional_directories(),
    );
    let mut state = ListSelectionState::new(view.model.into_body());

    assert_eq!(state.title(), "Config");
    assert!(state.search().is_some());
    assert_eq!(
        state
            .tabs()
            .iter()
            .map(|tab| tab.label())
            .collect::<Vec<_>>(),
        vec!["Config", "Add-dir", "Providers", "Language servers"]
    );
    let mouse = &state.visible_items()[0];
    assert_eq!(mouse.label(), "Mouse interactions");
    assert_eq!(
        mouse.description(),
        Some("Clicks and hover in interactive panes [ ✔ ]")
    );
    assert!(matches!(
        view.actions.get(mouse.id().unwrap()).unwrap(),
        ConfigSelectionAction::SetTerminalSettings(edit)
            if edit.terminal.expected_revision == 7
                && !edit.terminal.settings.mouse_interactions()
    ));

    let _ = state.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(state.visible_items().len(), 12);
    assert_eq!(state.visible_items()[0].label(), "Read files");
    assert_eq!(state.visible_items()[1].label(), "Modify files");
    assert!(matches!(
        view.actions
            .get(state.visible_items()[5].id().unwrap())
            .unwrap(),
        ConfigSelectionAction::SetTerminalSettings(edit)
            if edit.terminal.settings.additional_directory_permissions() == vec![
                WorkspaceAdditionalDirectoryPermissionDto::ReadFiles,
                WorkspaceAdditionalDirectoryPermissionDto::WriteFiles,
                WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceSearch,
            ]
    ));
    let _ = state.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
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
fn config_pane_uses_an_empty_unicode_checkbox_when_mouse_interactions_are_disabled() {
    let mut terminal = TerminalSettings::default();
    terminal.set_mouse_interactions(false);

    let view = config_pane_spec(
        &empty_config_snapshot(),
        &providers(),
        terminal,
        0,
        &session_id(),
        &no_additional_directories(),
    );
    let state = ListSelectionState::new(view.model.into_body());

    assert_eq!(
        state.visible_items()[0].description(),
        Some("Clicks and hover in interactive panes [   ]")
    );
}

#[test]
fn add_dir_items_emit_revision_bound_complete_permission_sets() {
    let directories = WorkspaceAdditionalDirectoryListResult {
        revision: 4,
        directories: vec![WorkspaceAdditionalDirectoryDto {
            contributions: Default::default(),
            root: "/workspace/shared".into(),
            trust: WorkspaceTrustStateDto::Trusted,
            permissions: vec![
                WorkspaceAdditionalDirectoryPermissionDto::ReadFiles,
                WorkspaceAdditionalDirectoryPermissionDto::WriteFiles,
            ],
        }],
    };
    let view = config_pane_spec(
        &empty_config_snapshot(),
        &providers(),
        TerminalSettings::default(),
        0,
        &session_id(),
        &directories,
    );
    let mut state = ListSelectionState::new(view.model.into_body());
    let _ = state.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

    assert_eq!(state.visible_items().len(), 24);
    assert_eq!(
        state.visible_items()[12].label(),
        "Read files · /workspace/shared"
    );
    assert_eq!(
        state.visible_items()[12].description(),
        Some("Allow read_file, grep and glob [ ✔ ]")
    );
    let execute = &state.visible_items()[14];
    assert_eq!(
        execute.description(),
        Some("Allow shell-command and Session terminals; requires Read files [   ]")
    );
    assert_eq!(
        state.visible_items()[15].description(),
        Some(
            "Refresh authorized project configuration after file changes; requires Read files [   ]"
        )
    );
    assert_eq!(
        state.visible_items()[16].description(),
        Some("Show this directory in Workspace Files; requires Read files [   ]")
    );
    assert_eq!(
        state.visible_items()[18].description(),
        Some("Load .zeta/instructions and .zeta/agents; requires Read files [   ]")
    );
    assert_eq!(
        state.visible_items()[20].description(),
        Some("Authorize MCP declarations (0 found); connect them separately [   ]")
    );
    assert!(matches!(
        view.actions.get(execute.id().unwrap()).unwrap(),
        ConfigSelectionAction::SetAdditionalDirectoryPermissions(edit)
            if edit.params.expected_revision == 4
                && edit.params.permissions == vec![
                    WorkspaceAdditionalDirectoryPermissionDto::ReadFiles,
                    WorkspaceAdditionalDirectoryPermissionDto::WriteFiles,
                    WorkspaceAdditionalDirectoryPermissionDto::ExecuteCommands,
                ]
    ));
}

#[test]
fn provider_api_key_input_is_masked_keeps_its_explanation_and_submits_with_enter() {
    let prompt = provider_api_key_prompt("openai".into(), "OpenAI".into());
    let (spec, key_hints) = prompt.spec.into_parts();
    let mut state = TextPrompt::new(spec);

    state.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    let outcome = state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(state.input().masked());
    assert_eq!(key_hints, "Enter save  ·  Esc cancel");
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
