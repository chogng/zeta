use super::App;
use super::AppCommand;
use super::Status;
use crate::app::AppEvent;
use crate::components::chat_composer::ChatComposerPaneView;
use crate::components::chat_history::MessageRole;
use crate::components::chat_input::ChatInputItem;
use crate::components::chat_input::ChatInputMode;
use crate::components::chat_input::ChatSubmission;
use crate::components::chat_input::built_in_slash_command_definitions;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneBodyView;
use crate::components::pane::PaneSpec;
use crate::components::suggest::SuggestView;
use crate::features::approval::Approval;
use crate::features::approval::ApprovalSpec;
use crate::features::config::FollowUpMode;
use crate::features::config::TerminalSettings;
use crate::features::config::config_pane_spec;
use crate::features::file_search::FileSearchManager;
use crate::features::keymap::KeymapEditIntent;
use crate::features::keymap::KeymapEditKind;
use crate::features::keymap::keymap_pane_spec;
use crate::features::query::Query;
use crate::features::query::QueryChoice;
use crate::features::query::QueryCustomAnswer;
use crate::features::query::QueryQuestion;
use crate::features::rewind::rewind_pane_spec;
use crate::features::status_line::StatusLineItem;
use crate::features::status_line::StatusLineResource;
use crate::features::theme::ThemePickerCatalog;
use crate::features::theme::ThemePickerChoice;
use crate::features::theme::ThemePickerTarget;
use crate::features::theme::ThemePreviewPalette;
use crate::features::theme::custom_theme_pane_spec;
use crate::features::theme::theme_pane_spec;
use crate::features::thread::TurnActivity;
use crate::keymap::AppKeymap;
use crate::mouse::MouseMode;
use crate::test_support::empty_config_snapshot;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::style::Color;
use std::fs;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_app_server_protocol::protocol::environment::PermissionDto;
use zeta_app_server_protocol::protocol::environment::SessionDirDto;
use zeta_app_server_protocol::protocol::environment::SessionDirListResult;
use zeta_app_server_protocol::protocol::provider::{
    ProviderApiKeyPolicyDto, ProviderCatalogEntryDto, ProviderListResult,
};
use zeta_protocol::ApprovalMode;
use zeta_protocol::ContentDigest;
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillRef;
use zeta_protocol::SkillSourceId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadStatus;
use zeta_protocol::Turn;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;

fn set_follow_up_mode(app: &mut App, mode: FollowUpMode) {
    let mut settings = TerminalSettings::default();
    settings.set_follow_up_mode(mode);
    app.update(AppEvent::ConfigSettingsReceived(settings));
}

fn config_session() -> SessionId {
    SessionId::new("config-state-session").unwrap()
}

fn no_directories() -> SessionDirListResult {
    SessionDirListResult {
        revision: 0,
        dirs: Vec::new(),
    }
}

fn enter_test_session(app: &mut App) {
    app.update(AppEvent::ThreadContextChanged {
        session_id: SessionId::new("test-session").unwrap(),
        thread_id: ThreadId::new("test-thread").unwrap(),
    });
}
use zeta_slash_commands::{
    SlashCommandArgumentMode, SlashCommandCatalog, SlashCommandDefinition, SlashCommandOrigin,
};

#[test]
fn enter_submits_trimmed_input_and_records_the_user_message() {
    let mut app = App::new();
    app.insert_text("  explain this  ");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_text_submission(action, "explain this");
    assert_eq!(app.input(), "");
    assert_eq!(app.messages().len(), 1);
    assert_eq!(app.messages()[0].role, MessageRole::User);
    assert_eq!(app.messages()[0].text, "explain this");
    assert_eq!(app.status(), &Status::Working);
}

#[test]
fn blank_input_does_not_start_a_turn() {
    let mut app = App::new();
    app.insert_text("   ");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, None);
    assert!(app.messages().is_empty());
    assert_eq!(app.status(), &Status::Ready);
}

#[test]
fn approval_preserves_the_hidden_draft_and_stays_open_after_submission_failure() {
    let mut app = App::new();
    enter_test_session(&mut app);
    app.insert_text("draft");
    app.update(AppEvent::ApprovalRequested(Approval::new(ApprovalSpec {
        title: "Approval required".into(),
        reason: "Run tests".into(),
        details: Vec::new(),
    })));

    app.handle_paste("ignored".into());
    let Some(AppCommand::ResolveThreadRequest(response)) =
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("expected an Approval response");
    };
    let request = response.identity();
    app.update(AppEvent::ThreadRequestSubmissionFailed {
        request,
        error: "offline".into(),
    });

    assert_eq!(app.input(), "draft");
    assert_eq!(
        app.approval_view().and_then(|view| view.error),
        Some("offline")
    );
}

#[test]
fn query_paste_uses_its_own_editor_without_changing_the_chat_draft() {
    let mut app = App::new();
    enter_test_session(&mut app);
    app.insert_text("draft");
    app.update(AppEvent::QueryRequested(
        Query::new(vec![QueryQuestion {
            id: "answer".into(),
            header: "Answer".into(),
            prompt: "What next?".into(),
            choices: vec![QueryChoice {
                label: "Default".into(),
                description: "Use the default".into(),
            }],
            custom_answer: QueryCustomAnswer::Allowed,
        }])
        .unwrap(),
    ));

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_paste("custom".into());

    assert_eq!(app.input(), "draft");
    assert_eq!(
        app.query_view().and_then(|view| view.custom_answer),
        Some("custom")
    );
}

#[test]
fn selected_theme_closes_the_theme_pane_after_success() {
    let mut app = App::new();
    app.update(AppEvent::ThemePaneOpened(theme_pane_spec(&theme_catalog())));

    assert_eq!(app.list_selection().unwrap().title(), "Theme");
    let command = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(
        command,
        Some(AppCommand::SetTheme {
            preference: "zeta-code-dark".into(),
        })
    );
    app.update(AppEvent::ThemePanesClosed);
    assert!(app.list_selection().is_none());
}

#[test]
fn pointer_activation_uses_the_feature_pane_action_mapping() {
    let mut app = App::new();
    app.update(AppEvent::ThemePaneOpened(theme_pane_spec(&theme_catalog())));

    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);
    assert!(app.select_visible_item(1));
    assert_eq!(
        app.list_selection().unwrap().selected_visible_index(),
        Some(1)
    );
    assert_eq!(
        app.activate_visible_item(1),
        Some(AppCommand::OpenCustomThemePane)
    );
}

#[test]
fn selected_custom_theme_closes_the_entire_theme_flow_after_success() {
    let catalog = theme_catalog();
    let mut app = App::new();
    app.update(AppEvent::ThemePaneOpened(theme_pane_spec(&catalog)));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::OpenCustomThemePane)
    );
    app.update(AppEvent::ThemePaneOpened(custom_theme_pane_spec(&catalog)));
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::SetCustomTheme {
            preference: "aurora".into(),
        })
    );

    app.update(AppEvent::ThemePanesClosed);
    assert!(app.list_selection().is_none());
}

#[test]
fn selected_rewind_checkpoint_emits_a_typed_rewind_action() {
    let turn_id = TurnId::new("turn-1").unwrap();
    let thread = Thread {
        session_id: SessionId::new("session").unwrap(),
        thread_id: ThreadId::new("thread").unwrap(),
        parent_thread_id: None,
        forked_from_id: None,
        title: "thread".into(),
        status: ThreadStatus::Active,
        sequence: 5,
        usage: zeta_protocol::ModelUsageSummary::default(),
        goal: None,
        turns: vec![Turn {
            turn_id: turn_id.clone(),
            status: TurnStatus::Completed,
            kind: Default::default(),
            instructions: None,
            model: None,
            tool_profile: None,
            tool_mode: zeta_protocol::ToolMode::Direct,
            approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
            usage: zeta_protocol::ModelUsageSummary::default(),
            context_usage: None,
            items: vec![ThreadItem::UserMessage {
                item_id: ItemId::new("item-1").unwrap(),
                turn_id: turn_id.clone(),
                text: "restore here".into(),
            }],
            plan: None,
            pending_interaction: None,
            error: None,
        }],
    };
    let mut app = App::new();
    app.update(AppEvent::RewindPaneOpened(rewind_pane_spec(&thread)));

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::RewindToCheckpoint {
            before_turn_id: turn_id,
            checkpoint_label: "restore here".into(),
        })
    );
}

fn theme_catalog() -> ThemePickerCatalog {
    ThemePickerCatalog {
        choices: vec![
            ThemePickerChoice {
                label: "Dark mode".into(),
                palette_label: "GitHub Dark".into(),
                target: ThemePickerTarget::Preference("zeta-code-dark".into()),
                palette: theme_palette(),
                selected: true,
            },
            ThemePickerChoice {
                label: "Custom color theme".into(),
                palette_label: "User-defined".into(),
                target: ThemePickerTarget::CustomThemes,
                palette: theme_palette(),
                selected: false,
            },
        ],
        custom_choices: vec![ThemePickerChoice {
            label: "Aurora".into(),
            palette_label: "User-defined · Aurora".into(),
            target: ThemePickerTarget::Preference("aurora".into()),
            palette: theme_palette(),
            selected: false,
        }],
    }
}

fn theme_palette() -> ThemePreviewPalette {
    ThemePreviewPalette {
        background: Color::Black,
        border: Color::Gray,
        foreground: Color::White,
        muted: Color::DarkGray,
        highlight: Color::Magenta,
        keyword: Color::Red,
        string: Color::Blue,
        function: Color::Magenta,
        r#type: Color::Cyan,
        variable: Color::Yellow,
        inserted_background: Color::Green,
        removed_background: Color::Red,
        inserted_marker: Color::LightGreen,
        removed_marker: Color::LightRed,
    }
}

#[test]
fn large_paste_uses_a_placeholder_and_expands_on_submit() {
    let mut app = App::new();
    let pasted = "你".repeat(1001);

    app.handle_paste(pasted.clone());

    assert_eq!(app.input(), "[Pasted Content 1001 chars]");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_text_submission(action, &pasted);
    assert_eq!(app.messages()[0].text, pasted);
    assert_eq!(app.input(), "");
}

#[test]
fn repeated_large_pastes_with_the_same_size_have_distinct_placeholders() {
    let mut app = App::new();
    let first = "a".repeat(1001);
    let second = "b".repeat(1001);

    app.handle_paste(first.clone());
    app.insert_text(" ");
    app.handle_paste(second.clone());

    assert_eq!(
        app.input(),
        "[Pasted Content 1001 chars] [Pasted Content 1001 chars] #2"
    );

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_text_submission(action, &format!("{first} {second}"));
}

#[test]
fn deleting_a_large_paste_placeholder_discards_its_payload() {
    let mut app = App::new();
    app.handle_paste("a".repeat(1001));

    app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    assert_eq!(app.input(), "");
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    assert!(app.messages().is_empty());
}

#[test]
fn editing_before_a_large_paste_keeps_its_payload_binding() {
    let mut app = App::new();
    let pasted = "p".repeat(1001);
    app.insert_text("xa");
    app.handle_paste(pasted.clone());
    app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_text_submission(action, &format!("a{pasted}"));
}

#[test]
fn pasted_image_path_submits_a_structured_image() {
    let path = std::env::temp_dir().join(format!(
        "zeta-tui-app-image-{}-{}.png",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(&path, b"\x89PNG\r\n\x1a\npayload").unwrap();
    let mut app = App::new();

    app.handle_paste(path.to_string_lossy().into_owned());

    assert_eq!(app.input(), "[Image #1] ");
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let Some(AppCommand::SubmitTurn { submission, .. }) = action else {
        panic!("expected image submission");
    };
    assert_eq!(submission.display_text, "[Image #1]");
    assert_eq!(submission.input.len(), 1);
    assert!(matches!(
        &submission.input[0],
        ChatInputItem::Image { url } if url.starts_with("data:image/png;base64,")
    ));
    assert_eq!(app.messages()[0].text, "[Image #1]");
    let _ = fs::remove_file(path);
}

#[test]
fn control_v_requests_a_clipboard_image_read() {
    let mut app = App::new();

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));

    assert_eq!(action, Some(AppCommand::ReadClipboardImage));
    assert_eq!(app.status(), &Status::Ready);
}

#[test]
fn control_o_requests_copy_and_control_z_requests_suspend() {
    let mut app = App::new();

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL)),
        Some(AppCommand::CopyLastResponse)
    );
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL)),
        Some(AppCommand::Suspend)
    );
}

#[test]
fn copy_and_export_slash_commands_stay_in_the_terminal_host() {
    let mut app = App::new();
    app.insert_text("/copy");
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::CopyLastResponse)
    );

    app.insert_text("/export notes/conversation.md");
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::ExportTranscript {
            requested_path: Some(std::path::PathBuf::from("notes/conversation.md")),
        })
    );
}

#[test]
fn export_rejects_image_arguments_before_host_io() {
    let mut app = App::new();
    app.insert_text("/export ");
    app.update(AppEvent::ClipboardImageRead(Ok(
        b"\x89PNG\r\n\x1a\npayload".to_vec(),
    )));

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, None);
    assert_eq!(app.status(), &Status::Ready);
    assert!(
        app.messages()
            .last()
            .unwrap()
            .text
            .contains("relative text path")
    );
}

#[test]
fn local_conversation_commands_do_not_replace_a_running_turn() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.insert_text("/new");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, None);
    assert_eq!(app.status(), &Status::Working);
    assert!(
        app.messages()
            .last()
            .unwrap()
            .text
            .contains("is unavailable")
    );
}

#[test]
fn control_home_requests_an_older_history_page() {
    let mut app = App::new();

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL)),
        Some(AppCommand::LoadOlderHistory)
    );
}

#[test]
fn clipboard_png_submits_through_the_existing_attachment_path() {
    let mut app = App::new();
    app.update(AppEvent::ClipboardImageRead(Ok(
        b"\x89PNG\r\n\x1a\npayload".to_vec(),
    )));

    assert_eq!(app.input(), "[Image #1] ");
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let Some(AppCommand::SubmitTurn { submission, .. }) = action else {
        panic!("expected image submission");
    };
    assert_eq!(submission.display_text, "[Image #1]");
    assert!(matches!(
        &submission.input[0],
        ChatInputItem::Image { url } if url.starts_with("data:image/png;base64,")
    ));
}

#[test]
fn active_turn_accepts_clipboard_images_for_a_follow_up() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));

    assert_eq!(action, Some(AppCommand::ReadClipboardImage));
    assert_eq!(app.input(), "");
}

#[test]
fn control_c_requests_quit() {
    let mut app = App::new();

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(action, Some(AppCommand::Quit));
}

#[test]
fn quit_slash_command_requests_quit_without_starting_a_turn() {
    let mut app = App::new();
    app.insert_text("/quit");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, Some(AppCommand::Quit));
    assert_eq!(app.status(), &Status::Ready);
    assert!(app.messages().is_empty());
}

#[test]
fn product_command_is_delegated_to_the_typed_dispatcher() {
    let mut app = App::new();
    app.insert_text("/status");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let Some(AppCommand::ExecuteProductCommand(invocation)) = action else {
        panic!("expected product command action");
    };
    assert_eq!(invocation.command.name, "status");
    assert_eq!(invocation.origin, SlashCommandOrigin::Local);
    assert!(invocation.arguments.is_empty());
    assert_eq!(app.status(), &Status::Ready);
    assert!(app.messages().is_empty());
}

#[test]
fn shortcut_slash_command_is_owned_by_the_local_host() {
    let mut app = App::new();
    app.insert_text("/shortcuts");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, Some(AppCommand::OpenKeymapPane));
    assert!(app.messages().is_empty());
}

#[test]
fn config_slash_command_is_owned_by_the_local_host() {
    let mut app = App::new();
    app.insert_text("/config");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, Some(AppCommand::OpenConfigPane));
    assert!(app.messages().is_empty());
}

#[test]
fn config_mouse_selection_emits_a_revision_bound_edit() {
    let config = empty_config_snapshot();
    let mut app = App::new();
    app.update(AppEvent::ConfigPaneOpened(config_pane_spec(
        &config,
        &ProviderListResult { providers: vec![] },
        TerminalSettings::default(),
        7,
        &config_session(),
        &no_directories(),
    )));

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(
        action,
        Some(AppCommand::EditConfig(edit))
            if edit.terminal.expected_revision == 7
                && !edit.terminal.settings.mouse_interactions()
    ));
}

#[test]
fn config_follow_up_mode_supports_arrow_selection_and_enter_toggle() {
    let config = empty_config_snapshot();
    let mut app = App::new();
    app.update(AppEvent::ConfigPaneOpened(config_pane_spec(
        &config,
        &ProviderListResult { providers: vec![] },
        TerminalSettings::default(),
        7,
        &config_session(),
        &no_directories(),
    )));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    assert!(matches!(
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        Some(AppCommand::EditConfig(edit))
            if edit.terminal.expected_revision == 7
                && edit.terminal.settings.follow_up_mode() == FollowUpMode::Steer
    ));
    assert!(matches!(
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        Some(AppCommand::EditConfig(edit))
            if edit.terminal.settings.follow_up_mode() == FollowUpMode::Queue
    ));
    assert!(matches!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::EditConfig(edit))
            if edit.terminal.settings.follow_up_mode() == FollowUpMode::Steer
    ));

    set_follow_up_mode(&mut app, FollowUpMode::Steer);
    assert!(matches!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::EditConfig(edit))
            if edit.terminal.settings.follow_up_mode() == FollowUpMode::Queue
    ));
}

#[test]
fn config_directory_permission_selection_emits_a_revision_bound_server_edit() {
    let config = empty_config_snapshot();
    let directories = SessionDirListResult {
        revision: 3,
        dirs: vec![SessionDirDto {
            contributions: Default::default(),
            path: "/dir/shared".into(),
            permissions: vec![PermissionDto::ReadFiles, PermissionDto::WriteFiles],
        }],
    };
    let mut app = App::new();
    app.update(AppEvent::ConfigPaneOpened(config_pane_spec(
        &config,
        &ProviderListResult { providers: vec![] },
        TerminalSettings::default(),
        7,
        &config_session(),
        &directories,
    )));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    for _ in 0..15 {
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(
        action,
        Some(AppCommand::EditPermissions(edit))
            if edit.params.expected_revision == 3
                && edit.params.permissions == vec![PermissionDto::WriteFiles]
    ));
}

#[test]
fn config_provider_api_key_enter_saves_and_returns_to_config() {
    let config = empty_config_snapshot();
    let providers = ProviderListResult {
        providers: vec![ProviderCatalogEntryDto {
            provider: "openai".into(),
            display_name: "OpenAI".into(),
            api_key_policy: ProviderApiKeyPolicyDto::Required,
            api_key_configured: false,
        }],
    };
    let mut app = App::new();
    app.update(AppEvent::ConfigPaneOpened(config_pane_spec(
        &config,
        &providers,
        TerminalSettings::default(),
        7,
        &config_session(),
        &no_directories(),
    )));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let Some(AppCommand::SetProviderApiKey(edit)) = action else {
        panic!("provider API key input must emit a typed secret edit");
    };
    assert!(!format!("{edit:?}").contains("api_key: \"sk\""));
    assert_eq!(edit.into_parts(), ("openai".into(), "sk".into()));

    app.update(AppEvent::ConfigApiKeySaved {
        provider: "openai".into(),
        pane_spec: config_pane_spec(
            &config,
            &providers,
            TerminalSettings::default(),
            7,
            &config_session(),
            &no_directories(),
        ),
    });

    assert_eq!(app.list_selection().unwrap().title(), "Config");
}

#[test]
fn one_escape_cancels_provider_api_key_input_and_returns_to_config() {
    let config = empty_config_snapshot();
    let providers = ProviderListResult {
        providers: vec![ProviderCatalogEntryDto {
            provider: "openai".into(),
            display_name: "OpenAI".into(),
            api_key_policy: ProviderApiKeyPolicyDto::Required,
            api_key_configured: false,
        }],
    };
    let mut app = App::new();
    app.update(AppEvent::ConfigPaneOpened(config_pane_spec(
        &config,
        &providers,
        TerminalSettings::default(),
        7,
        &config_session(),
        &no_directories(),
    )));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        None
    );
    assert_eq!(app.list_selection().unwrap().title(), "Config");
}

#[test]
fn statusline_slash_command_is_owned_by_the_local_host() {
    let mut app = App::new();
    app.insert_text("/statusline");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, Some(AppCommand::OpenStatusLinePane));
    assert!(app.messages().is_empty());
}

#[test]
fn statusline_selection_emits_a_revision_bound_edit() {
    let directory = tempfile::tempdir().unwrap();
    let mut resource = StatusLineResource::new(directory.path().join("statusline.json"));
    resource.refresh().unwrap();
    let mut app = App::new();
    app.update(AppEvent::StatusLinePaneOpened(resource.setup_pane_spec()));

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(
        action,
        Some(AppCommand::EditStatusLine(edit))
            if edit.expected_revision == 1
                && edit.item == StatusLineItem::Permissions
                && !edit.enabled
    ));
}

#[test]
fn shortcut_capture_emits_a_revision_bound_edit() {
    let mut app = App::new();
    app.update(AppEvent::KeymapPaneOpened(keymap_pane_spec(
        AppKeymap::default().setup_actions(),
        Path::new("/profile/zeta-code/keybindings.json"),
        &[],
        7,
    )));

    assert_eq!(app.list_selection().unwrap().title(), "Keymap");
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    assert_eq!(app.list_selection().unwrap().title(), "Cycle approval mode");
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    assert!(matches!(
        app.input_pane_views().as_slice(),
        [ChatComposerPaneView::Stacked(view)]
            if matches!(view.body(), PaneBodyView::KeyCapture(body) if body.title() == "Record shortcut")
    ));

    let edit = app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
    assert_eq!(
        edit,
        Some(AppCommand::EditKeymap(
            crate::features::keymap::KeymapEdit {
                expected_revision: 7,
                command_id: "zetaCode.action.cycleApprovalMode".into(),
                kind: KeymapEditKind::Set {
                    key: "ctrl+y".into(),
                    intent: KeymapEditIntent::ReplaceUser,
                },
            }
        ))
    );
}

#[test]
fn inline_product_arguments_reach_the_typed_dispatcher() {
    let mut app = App::new();
    app.insert_text("/model provider/model");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let Some(AppCommand::ExecuteProductCommand(invocation)) = action else {
        panic!("expected product command action");
    };
    assert_eq!(invocation.command.name, "model");
    assert_eq!(invocation.origin, SlashCommandOrigin::Local);
    assert_eq!(invocation.display_arguments, "provider/model");
    assert_eq!(
        invocation.arguments,
        vec![ChatInputItem::Text("provider/model".into())]
    );
    assert_eq!(app.status(), &Status::Ready);
    assert!(app.messages().is_empty());
}

#[test]
fn runtime_command_registry_drives_popup_and_submission_consistently() {
    let dir = temporary_dir("dynamic-slash-command");
    let registry = SlashCommandCatalog::with_local_and_server(
        built_in_slash_command_definitions(),
        [SlashCommandDefinition {
            name: "diagnose".into(),
            description: "inspect the current dir".into(),
            argument_mode: SlashCommandArgumentMode::Optional,
        }],
    )
    .unwrap();
    let mut app = App::for_dir_with_slash_commands(&dir, registry);
    app.insert_text("/diag logs");
    app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.input(), "/diagnose logs");
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        action,
        Some(AppCommand::SubmitTurn {
            submission: ChatSubmission {
                display_text: "/diagnose logs".into(),
                input: vec![ChatInputItem::Text("/diagnose logs".into())],
            },
        })
    );
    assert_eq!(app.status(), &Status::Working);
    assert_eq!(app.messages()[0].role, MessageRole::User);
    assert_eq!(app.messages()[0].text, "/diagnose logs");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dollar_skill_selector_submits_exact_skill_ref_with_visible_intent() {
    let dir = temporary_dir("skill-selector");
    let skill = SkillRef::pinned(
        SkillId::new(
            SkillSourceId::new("user:skill-source:test").unwrap(),
            SkillName::new("commit").unwrap(),
        ),
        ContentDigest::sha256(b"commit skill"),
    );
    let registry = SlashCommandCatalog::with_local_and_server(
        built_in_slash_command_definitions(),
        std::iter::empty(),
    )
    .unwrap();
    let mut app = App::for_dir_with_slash_commands(&dir, registry.clone());
    app.replace_chat_input_catalog(
        registry,
        vec![crate::components::suggest::SkillSelectorItem::new(
            "commit".into(),
            "draft a commit message".into(),
            skill.clone(),
        )],
        Vec::new(),
    );
    app.insert_text("$com");
    assert!(matches!(
        app.suggest(),
        Some(SuggestView::Skill(view)) if view.items[0].name() == "commit"
    ));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.insert_text("staged changes");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        action,
        Some(AppCommand::SubmitTurn {
            submission: ChatSubmission {
                display_text: "$commit staged changes".into(),
                input: vec![
                    ChatInputItem::Skill { skill },
                    ChatInputItem::Text("$commit staged changes".into()),
                ],
            },
        })
    );
    assert_eq!(app.messages()[0].text, "$commit staged changes");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn activating_a_slash_command_by_index_uses_the_command_dispatch_path() {
    let mut app = App::new();
    app.insert_text("/q");

    let action = app.activate_input_overlay_choice(0);

    assert_eq!(action, Some(AppCommand::Quit));
    assert!(app.input().is_empty());
    assert!(app.messages().is_empty());
}

#[test]
fn unknown_slash_input_remains_a_prompt() {
    let mut app = App::new();
    app.insert_text("/explain");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_text_submission(action, "/explain");
    assert_eq!(app.status(), &Status::Working);
    assert_eq!(app.messages()[0].text, "/explain");
}

#[test]
fn slash_popup_selection_executes_without_an_exact_query() {
    let mut app = App::new();
    app.insert_text("/q");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, Some(AppCommand::Quit));
    assert!(app.input().is_empty());
    assert!(app.messages().is_empty());
}

#[test]
fn mouse_interactions_capture_pointer_input_on_every_screen() {
    let mut app = App::new();
    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);

    app.insert_text("/");
    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);

    let dir = temporary_dir("mouse-interaction-mention");
    fs::write(dir.join("notes.md"), "notes").unwrap();
    let mut app = App::for_dir(&dir);
    app.insert_text("@notes");
    wait_for_mention_results(&mut app, &dir);

    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn disabled_mouse_interactions_leave_selection_to_the_terminal() {
    let mut app = App::new();
    app.insert_text("/");
    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);
    app.begin_screen_selection(ratatui::layout::Position::new(1, 1));
    app.drag_screen_selection(ratatui::layout::Position::new(3, 1));
    assert!(app.screen_selection().range().is_some());

    let mut settings = TerminalSettings::default();
    settings.set_mouse_interactions(false);
    app.update(AppEvent::ConfigSettingsReceived(settings));

    assert_eq!(app.mouse_mode(), MouseMode::TerminalSelection);
    assert!(app.screen_selection().range().is_none());
}

#[test]
fn terminal_settings_apply_vim_mode_to_the_active_chat_input() {
    let mut app = App::new();
    let mut settings = TerminalSettings::default();
    settings.set_input_mode(ChatInputMode::Vim);

    app.update(AppEvent::ConfigSettingsReceived(settings));
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.thread_presentations.active().input.prompt(), "N ");
}

#[test]
fn tab_completes_the_selected_slash_command_without_executing_it() {
    let mut app = App::new();
    app.insert_text("/q");

    let action = app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

    assert_eq!(action, None);
    assert_eq!(app.input(), "/quit ");
    assert_eq!(app.status(), &Status::Ready);
}

#[test]
fn at_file_popup_completes_an_atomic_path_before_submission() {
    let dir = temporary_dir("mention-completion");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/lib.rs"), "fn main() {}").unwrap();
    let mut app = App::for_dir(&dir);
    app.insert_text("review @lib");
    wait_for_mention_results(&mut app, &dir);

    assert!(matches!(
        app.suggest(),
        Some(SuggestView::Mention(view)) if view.matches[0].label == "src/lib.rs"
    ));
    let completion = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(completion, None);
    assert_eq!(app.input(), "review src/lib.rs ");
    assert_eq!(app.status(), &Status::Ready);

    let submission = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_text_submission(submission, "review src/lib.rs");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn escape_dismisses_an_at_file_popup_and_is_inert_at_the_root() {
    let dir = temporary_dir("mention-dismiss");
    fs::write(dir.join("notes.md"), "notes").unwrap();
    let mut app = App::for_dir(&dir);
    app.insert_text("@notes");

    let dismissed = app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(dismissed, None);
    assert!(app.suggest().is_none());
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        None
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn escape_does_not_exit_the_idle_root_view() {
    let mut app = App::new();

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        None
    );
    assert_eq!(app.status(), &Status::Ready);
}

#[test]
fn two_root_escape_presses_within_the_gesture_window_open_rewind() {
    let mut app = App::new();
    let started = Instant::now();

    assert_eq!(
        app.handle_key_at(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), started,),
        None
    );
    assert_eq!(
        app.handle_key_at(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            started + Duration::from_millis(200),
        ),
        Some(AppCommand::OpenRewindPane)
    );
}

#[test]
fn escape_from_a_view_does_not_count_toward_the_root_rewind_sequence() {
    let mut app = App::new();
    let started = Instant::now();
    assert_eq!(
        app.handle_key_at(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), started),
        None
    );
    app.update(AppEvent::ListSelectionPaneOpened(PaneSpec::new(
        ListSelectionModel::new(
            "Feature",
            vec![ListSelectionGroup::new(
                "Items",
                vec![ListSelectionItem::new("Item")],
            )],
        ),
        "Esc back",
    )));

    assert_eq!(
        app.handle_key_at(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            started + Duration::from_millis(100),
        ),
        None
    );
    assert!(app.list_selection().is_none());
    assert_eq!(
        app.handle_key_at(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            started + Duration::from_millis(200),
        ),
        None
    );
}

#[test]
fn root_escape_gesture_expires_and_is_reset_by_other_input() {
    let mut app = App::new();
    let started = Instant::now();
    let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);

    assert_eq!(app.handle_key_at(escape, started), None);
    assert_eq!(
        app.handle_key_at(escape, started + Duration::from_millis(600)),
        None
    );
    app.handle_key_at(
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
        started + Duration::from_millis(650),
    );
    assert_eq!(
        app.handle_key_at(escape, started + Duration::from_millis(700)),
        None
    );
}

#[test]
fn control_c_interrupts_a_working_turn_without_exiting() {
    let mut app = App::new();
    app.insert_text("hello");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(action, Some(AppCommand::Interrupt));
    assert_eq!(app.status(), &Status::Cancelling);
}

#[test]
fn a_second_control_c_does_not_duplicate_an_interrupt() {
    let mut app = App::new();
    app.insert_text("hello");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(action, None);
    assert_eq!(app.status(), &Status::Cancelling);
}

#[test]
fn control_c_interrupts_a_turn_waiting_for_user_input() {
    let mut app = App::new();
    app.update(AppEvent::TurnActivityChanged(
        TurnActivity::WaitingForUserInput,
    ));

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(action, Some(AppCommand::Interrupt));
    assert_eq!(app.status(), &Status::Cancelling);
}

#[test]
fn enter_steers_the_working_turn_and_tracks_delivery() {
    let mut app = App::new();
    set_follow_up_mode(&mut app, FollowUpMode::Steer);
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));

    app.insert_text("second");
    app.handle_paste("third".into());
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let Some(AppCommand::SteerTurn {
        steer_id,
        submission,
    }) = action
    else {
        panic!("expected active Turn steer");
    };
    assert_eq!(submission.display_text, "secondthird");
    assert_eq!(app.input(), "");
    assert_eq!(app.messages().len(), 2);
    assert_eq!(app.messages()[1].text, "secondthird");
    assert!(app.input_pane_views().is_empty());

    app.update(AppEvent::SteerCompleted(steer_id));

    assert!(app.input_pane_views().is_empty());
    assert_eq!(app.status(), &Status::Working);
}

#[test]
fn enter_queues_a_new_turn_by_default_while_the_current_turn_is_working() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("next turn");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, None);
    assert_eq!(app.queue_view().items[0].text, "next turn");
    assert_eq!(app.status(), &Status::Working);

    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.input(), "next turn");
    assert_eq!(app.queue_view().items[0].text, "next turn");
}

#[test]
fn queue_command_restores_the_selected_message_without_bare_up_interception() {
    let mut app = App::new();
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("restore me");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(AppEvent::TurnCompleted);
    app.insert_text("/queue");

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    assert_eq!(app.list_selection().unwrap().title(), "Queue");
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));

    assert_eq!(app.input(), "restore me");
    assert!(app.list_selection().is_none());
    assert!(app.queue_view().items.is_empty());
}

#[test]
fn sessions_and_agents_commands_open_the_manager_root() {
    for command in ["/sessions", "/agents"] {
        let mut app = App::new();
        app.insert_text(command);

        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            None
        );
        assert!(app.session_manager_view().is_some());
    }
}

#[test]
fn queued_turn_stays_editable_when_automatic_submission_is_rejected() {
    let mut app = App::new();
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("keep this message");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(AppEvent::TurnCompleted);

    let Some(AppCommand::SubmitQueuedTurn {
        queue_id,
        submission,
    }) = app.dispatch_next_queued_turn()
    else {
        panic!("expected queued Turn dispatch");
    };
    assert_eq!(submission.display_text, "keep this message");
    assert!(app.queue_view().items[0].sending);

    app.update(AppEvent::QueueSubmissionFailed {
        queue_id,
        error: "server unavailable".into(),
    });
    assert!(!app.queue_view().items[0].sending);
}

#[test]
fn empty_enter_does_not_bypass_queue_dispatch_order() {
    let mut app = App::new();
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("send this now");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    set_follow_up_mode(&mut app, FollowUpMode::Steer);

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    assert_eq!(app.queue_view().items[0].text, "send this now");
}

#[test]
fn a_created_turn_does_not_claim_the_running_steer_action() {
    let mut app = App::new();
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Starting));
    app.insert_text("after the queued turn");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, None);
    assert_eq!(app.queue_view().items[0].text, "after the queued turn");
    assert_eq!(app.status(), &Status::Working);
}

#[test]
fn rejected_steer_removes_only_its_pending_row_and_keeps_the_turn_working() {
    let mut app = App::new();
    set_follow_up_mode(&mut app, FollowUpMode::Steer);
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("change direction");
    let Some(AppCommand::SteerTurn { steer_id, .. }) =
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("expected active Turn steer");
    };

    app.update(AppEvent::SteerSubmissionFailed {
        steer_id,
        error: "sequence conflict".into(),
    });

    assert!(app.input_pane_views().is_empty());
    assert_eq!(app.status(), &Status::Working);
    assert!(
        app.messages()
            .last()
            .unwrap()
            .text
            .contains("could not steer the active Turn: sequence conflict")
    );
}

#[test]
fn completion_returns_the_app_to_ready_without_appending_transcript_content() {
    let mut app = App::new();
    app.insert_text("hello");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    app.update(AppEvent::TurnCompleted);

    assert_eq!(app.messages().len(), 1);
    assert_eq!(app.messages()[0].role, MessageRole::User);
    assert_eq!(app.messages()[0].text, "hello");
    assert_eq!(app.status(), &Status::Ready);
}

#[test]
fn client_error_is_visible_in_history_and_status() {
    let mut app = App::new();

    app.update(AppEvent::FailureReported("provider unavailable".into()));

    assert_eq!(app.messages().len(), 1);
    assert_eq!(app.messages()[0].role, MessageRole::Error);
    assert_eq!(app.messages()[0].text, "provider unavailable");
    assert_eq!(app.status(), &Status::Error);
}

#[test]
fn interrupted_turn_returns_to_ready_with_a_notice() {
    let mut app = App::new();
    app.insert_text("hello");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    app.update(AppEvent::TurnInterrupted);

    assert_eq!(app.status(), &Status::Ready);
    assert_eq!(app.messages().last().unwrap().role, MessageRole::Notice);
    assert_eq!(app.messages().last().unwrap().text, "turn interrupted");
}

fn assert_text_submission(action: Option<AppCommand>, expected: &str) {
    let Some(AppCommand::SubmitTurn { submission }) = action else {
        panic!("expected text submission");
    };
    assert_eq!(submission.display_text, expected);
    assert_eq!(
        submission.input,
        vec![ChatInputItem::Text(expected.to_owned())]
    );
}

#[test]
fn backtab_cycles_the_next_turn_approval_mode() {
    let mut app = App::new();

    let action = app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(action, Some(AppCommand::CycleNextApprovalMode));
    assert_eq!(app.approval_mode(), ApprovalMode::AskPermissions);

    app.set_next_approval_mode(ApprovalMode::AutoReview);
    let action = app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(action, Some(AppCommand::CycleNextApprovalMode));
}

fn wait_for_mention_results(app: &mut App, dir: &Path) {
    let mut file_search = FileSearchManager::new(dir.to_path_buf());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(query) = app.mention_query() {
            file_search.update_query(query);
        } else {
            file_search.stop();
        }
        for snapshot in file_search.poll() {
            app.update(AppEvent::FileSearchSnapshotReceived(snapshot));
        }
        if matches!(
            app.suggest(),
            Some(SuggestView::Mention(popup)) if !popup.matches.is_empty()
        ) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for mention search results"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn temporary_dir(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "zeta-tui-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}
