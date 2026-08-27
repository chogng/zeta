use super::App;
use super::AppCommand;
use super::Status;
use crate::app::AppEvent;
use crate::components::composer::ComposerInput;
use crate::components::composer::ComposerSubmission;
use crate::components::composer::built_in_slash_command_definitions;
use crate::components::transcript::MessageRole;
use crate::features::config::TerminalSettings;
use crate::features::config::config_view;
use crate::features::rewind::rewind_selection_view;
use crate::features::shortcuts::ShortcutEditIntent;
use crate::features::shortcuts::ShortcutEditKind;
use crate::features::shortcuts::shortcut_view;
use crate::features::status_line::StatusLineItem;
use crate::features::status_line::StatusLineResource;
use crate::features::theme::ThemePickerCatalog;
use crate::features::theme::ThemePickerChoice;
use crate::features::theme::ThemePickerTarget;
use crate::features::theme::ThemePreviewPalette;
use crate::features::theme::custom_theme_selection_view;
use crate::features::theme::theme_selection_view;
use crate::features::thread::TurnActivity;
use crate::features::workspace_files::FileSearchManager;
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
fn selected_theme_closes_the_theme_pane_after_success() {
    let mut app = App::new();
    app.update(AppEvent::ThemeViewOpened(theme_selection_view(
        &theme_catalog(),
    )));

    assert_eq!(app.selection_view().unwrap().title(), "Theme");
    let command = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(
        command,
        Some(AppCommand::SetTheme {
            preference: "zeta-code-dark".into(),
        })
    );
    app.update(AppEvent::ThemeViewClosed);
    assert!(app.selection_view().is_none());
}

#[test]
fn pointer_activation_uses_the_feature_pane_action_mapping() {
    let mut app = App::new();
    app.update(AppEvent::ThemeViewOpened(theme_selection_view(
        &theme_catalog(),
    )));

    assert_eq!(app.mouse_mode(), MouseMode::UiClick);
    assert!(app.select_visible_item(1));
    assert_eq!(
        app.selection_view().unwrap().selected_visible_index(),
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
    app.update(AppEvent::ThemeViewOpened(theme_selection_view(&catalog)));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::OpenCustomThemePane)
    );
    app.update(AppEvent::ThemeViewOpened(custom_theme_selection_view(
        &catalog,
    )));
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::SetCustomTheme {
            preference: "aurora".into(),
        })
    );

    app.update(AppEvent::ThemeViewClosed);
    assert!(app.selection_view().is_none());
}

#[test]
fn selected_rewind_checkpoint_emits_a_typed_rewind_action() {
    let turn_id = TurnId::new("turn-1").unwrap();
    let thread = Thread {
        session_id: SessionId::new("session").unwrap(),
        thread_id: ThreadId::new("thread").unwrap(),
        title: "thread".into(),
        status: ThreadStatus::Active,
        sequence: 5,
        usage: zeta_protocol::ModelUsageSummary::default(),
        goal: None,
        turns: vec![Turn {
            turn_id: turn_id.clone(),
            status: TurnStatus::Completed,
            model: None,
            tool_profile: None,
            tool_mode: zeta_protocol::ToolMode::Direct,
            usage: zeta_protocol::ModelUsageSummary::default(),
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
    app.update(AppEvent::RewindViewOpened(rewind_selection_view(&thread)));

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
        ComposerInput::Image { url } if url.starts_with("data:image/png;base64,")
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
        ComposerInput::Image { url } if url.starts_with("data:image/png;base64,")
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
fn control_c_requests_exit() {
    let mut app = App::new();

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(action, Some(AppCommand::Quit));
}

#[test]
fn quit_slash_command_requests_exit_without_starting_a_turn() {
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

    assert_eq!(action, Some(AppCommand::OpenShortcutsPane));
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
    app.update(AppEvent::ConfigViewOpened(config_view(
        &config,
        &ProviderListResult { providers: vec![] },
        TerminalSettings::default(),
        7,
    )));

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(
        action,
        Some(AppCommand::EditConfig(edit))
            if edit.terminal.expected_revision == 7 && !edit.terminal.mouse_interactions
    ));
}

#[test]
fn config_provider_selection_opens_a_masked_api_key_input_and_emits_a_secret_edit() {
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
    app.update(AppEvent::ConfigViewOpened(config_view(
        &config,
        &providers,
        TerminalSettings::default(),
        7,
    )));
    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL));

    let Some(AppCommand::SetProviderApiKey(edit)) = action else {
        panic!("provider API key input must emit a typed secret edit");
    };
    assert!(!format!("{edit:?}").contains("api_key: \"sk\""));
    assert_eq!(edit.into_parts(), ("openai".into(), "sk".into()));
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
    app.update(AppEvent::StatusLineViewOpened(resource.setup_view()));

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
    app.update(AppEvent::ShortcutViewOpened(shortcut_view(
        AppKeymap::default().setup_actions(),
        Path::new("/profile/zeta-code/keybindings.json"),
        &[],
        7,
    )));

    assert_eq!(app.selection_view().unwrap().title(), "Shortcuts");
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    assert_eq!(app.selection_view().unwrap().title(), "Cycle approval mode");
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    assert_eq!(app.selection_view().unwrap().title(), "Record shortcut");

    let edit = app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
    assert_eq!(
        edit,
        Some(AppCommand::EditShortcut(
            crate::features::shortcuts::ShortcutEdit {
                expected_revision: 7,
                command_id: "zetaCode.action.cycleApprovalMode".into(),
                kind: ShortcutEditKind::Set {
                    key: "ctrl+y".into(),
                    intent: ShortcutEditIntent::ReplaceUser,
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
        vec![ComposerInput::Text("provider/model".into())]
    );
    assert_eq!(app.status(), &Status::Ready);
    assert!(app.messages().is_empty());
}

#[test]
fn runtime_command_registry_drives_popup_and_submission_consistently() {
    let workspace = temporary_workspace("dynamic-slash-command");
    let registry = SlashCommandCatalog::with_local_and_server(
        built_in_slash_command_definitions(),
        [SlashCommandDefinition {
            name: "diagnose".into(),
            description: "inspect the current workspace".into(),
            argument_mode: SlashCommandArgumentMode::Optional,
        }],
    )
    .unwrap();
    let mut app = App::for_workspace_with_slash_commands(&workspace, registry);
    app.insert_text("/diag logs");
    app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.input(), "/diagnose logs");
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        action,
        Some(AppCommand::SubmitTurn {
            submission: ComposerSubmission {
                display_text: "/diagnose logs".into(),
                input: vec![ComposerInput::Text("/diagnose logs".into())],
            },
            approval_mode: ApprovalMode::AskPermissions,
        })
    );
    assert_eq!(app.status(), &Status::Working);
    assert_eq!(app.messages()[0].role, MessageRole::User);
    assert_eq!(app.messages()[0].text, "/diagnose logs");
    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn direct_skill_slash_command_submits_exact_skill_ref_with_visible_intent() {
    let workspace = temporary_workspace("direct-skill-command");
    let skill = SkillRef::pinned(
        SkillId::new(
            SkillSourceId::new("user:skill-source:test").unwrap(),
            SkillName::new("commit").unwrap(),
        ),
        ContentDigest::sha256(b"commit skill"),
    );
    let registry = SlashCommandCatalog::with_local_server_and_skills(
        built_in_slash_command_definitions(),
        std::iter::empty(),
        [SlashCommandDefinition {
            name: "commit".into(),
            description: "draft a commit message".into(),
            argument_mode: SlashCommandArgumentMode::Optional,
        }],
    )
    .unwrap();
    let mut app = App::for_workspace_with_slash_commands(&workspace, registry.clone());
    app.replace_slash_commands(
        registry,
        [("commit".into(), skill.clone())].into_iter().collect(),
    );
    app.insert_text("/commit staged changes");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        action,
        Some(AppCommand::SubmitTurn {
            submission: ComposerSubmission {
                display_text: "/commit staged changes".into(),
                input: vec![
                    ComposerInput::Skill { skill },
                    ComposerInput::Text("/commit staged changes".into()),
                ],
            },
            approval_mode: ApprovalMode::AskPermissions,
        })
    );
    assert_eq!(app.messages()[0].text, "/commit staged changes");
    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn activating_a_slash_command_by_index_uses_the_command_dispatch_path() {
    let mut app = App::new();
    app.insert_text("/q");

    let action = app.activate_slash_command(0);

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
fn clickable_composer_popups_declare_ui_click_mouse_mode() {
    let mut app = App::new();
    assert_eq!(app.mouse_mode(), MouseMode::TerminalSelection);

    app.insert_text("/");
    assert_eq!(app.mouse_mode(), MouseMode::UiClick);

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.mouse_mode(), MouseMode::TerminalSelection);

    let workspace = temporary_workspace("mouse-interaction-mention");
    fs::write(workspace.join("notes.md"), "notes").unwrap();
    let mut app = App::for_workspace(&workspace);
    app.insert_text("@notes");
    wait_for_mention_results(&mut app, &workspace);

    assert_eq!(app.mouse_mode(), MouseMode::UiClick);
    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn disabled_mouse_interactions_leave_selection_to_the_terminal() {
    let mut app = App::new();
    app.insert_text("/");
    assert_eq!(app.mouse_mode(), MouseMode::UiClick);

    let mut settings = TerminalSettings::default();
    settings.set_mouse_interactions(false);
    app.update(AppEvent::ConfigSettingsReceived(settings));

    assert_eq!(app.mouse_mode(), MouseMode::TerminalSelection);
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
fn at_file_popup_completes_an_atomic_workspace_path_before_submission() {
    let workspace = temporary_workspace("mention-completion");
    fs::create_dir_all(workspace.join("src")).unwrap();
    fs::write(workspace.join("src/lib.rs"), "fn main() {}").unwrap();
    let mut app = App::for_workspace(&workspace);
    app.insert_text("review @lib");
    wait_for_mention_results(&mut app, &workspace);

    assert_eq!(app.mention_popup().unwrap().matches[0].path, "src/lib.rs");
    let completion = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(completion, None);
    assert_eq!(app.input(), "review src/lib.rs ");
    assert_eq!(app.status(), &Status::Ready);

    let submission = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_text_submission(submission, "review src/lib.rs");
    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn escape_dismisses_an_at_file_popup_and_is_inert_at_the_root() {
    let workspace = temporary_workspace("mention-dismiss");
    fs::write(workspace.join("notes.md"), "notes").unwrap();
    let mut app = App::for_workspace(&workspace);
    app.insert_text("@notes");

    let dismissed = app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(dismissed, None);
    assert_eq!(app.mention_popup(), None);
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        None
    );
    let _ = fs::remove_dir_all(workspace);
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
fn working_turn_accepts_and_submits_a_follow_up_prompt() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    app.insert_text("second");
    app.handle_paste("third".into());
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(action, Some(AppCommand::SubmitTurn { .. })));
    assert_eq!(app.input(), "");
    assert_eq!(app.messages().len(), 2);
    assert_eq!(app.messages()[1].text, "secondthird");
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
    let Some(AppCommand::SubmitTurn {
        submission,
        approval_mode,
    }) = action
    else {
        panic!("expected text submission");
    };
    assert_eq!(approval_mode, ApprovalMode::AskPermissions);
    assert_eq!(submission.display_text, expected);
    assert_eq!(
        submission.input,
        vec![ComposerInput::Text(expected.to_owned())]
    );
}

#[test]
fn backtab_cycles_approval_mode_and_submission_freezes_the_selected_mode() {
    let mut app = App::new();

    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(app.approval_mode(), ApprovalMode::AutoReview);
    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(app.approval_mode(), ApprovalMode::BypassPermissions);
    app.insert_text("run it");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(matches!(
        action,
        Some(AppCommand::SubmitTurn {
            approval_mode: ApprovalMode::BypassPermissions,
            ..
        })
    ));

    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(app.approval_mode(), ApprovalMode::AskPermissions);
}

fn wait_for_mention_results(app: &mut App, workspace: &Path) {
    let mut file_search = FileSearchManager::new(workspace.to_path_buf());
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
        if app
            .mention_popup()
            .is_some_and(|popup| !popup.matches.is_empty())
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for mention search results"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn temporary_workspace(label: &str) -> std::path::PathBuf {
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
