use super::App;
use super::AppCommand;
use super::Status;
use crate::app::AppEvent;
use crate::config::Command as ConfigCommand;
use crate::config::Event as ConfigEvent;
use crate::config::TerminalSettings;
use crate::config::config_choices;
use crate::dirs::Command as DirCommand;
use crate::host::Command as HostCommand;
use crate::host::Event as HostEvent;
use crate::host::process_resources::ProcessResourceDemand;
use crate::host::process_resources::ProcessResourceRequest;
use crate::host::process_resources::ProcessResourceUsage;
use crate::host::process_resources::ProcessResourcesReading;
use crate::keymap::Command as KeymapCommand;
use crate::keymap::Event as KeymapEvent;
use crate::keymap::KeymapEditIntent;
use crate::keymap::KeymapEditKind;
use crate::keymap::KeymapEditorUpdate;
use crate::keymap::keymap_choices;
use crate::keymap::settings_from_tui as keymap_settings_from_tui;
use crate::render::RenderTheme;
use crate::sessions::Command as SessionCommand;
use crate::sessions::Event as SessionEvent;
use crate::skills::Event as SkillEvent;
use crate::status::Command as StatusCommand;
use crate::status::Event as StatusEvent;
use crate::status::ProcessMemoryCurrent;
use crate::status::StatusLineEditorUpdate;
use crate::status::StatusLineItem;
use crate::status::StatusLineSettings;
use crate::status::StatusViewData;
use crate::status::status_line_choices;
use crate::status::status_panel;
use crate::terminal::mouse::MouseMode;
use crate::test_support::empty_config_snapshot;
use crate::theme::Command as ThemeCommand;
use crate::theme::Event as ThemeEvent;
use crate::theme::ThemePickerCatalog;
use crate::theme::ThemePickerChoice;
use crate::theme::ThemePickerTarget;
use crate::theme::ThemePreviewPalette;
use crate::theme::custom_theme_choices;
use crate::theme::theme_choices;
use crate::thread::Command as ThreadCommand;
use crate::thread::Event as ThreadEvent;
use crate::thread::TurnActivity;
use crate::thread::composer::ChatInputItem;
use crate::thread::composer::ChatInputMode;
use crate::thread::composer::ChatSubmission;
use crate::thread::composer::CompletionView;
use crate::thread::composer::SteerSource;
use crate::thread::composer::built_in_slash_command_definitions;
use crate::thread::composer::file_search::FileSearchManager;
use crate::thread::interaction::approval::Approval;
use crate::thread::interaction::approval::ApprovalSpec;
use crate::thread::interaction::query::Query;
use crate::thread::interaction::query::QueryChoice;
use crate::thread::interaction::query::QueryCustomAnswer;
use crate::thread::interaction::query::QueryQuestion;
use crate::thread::rewind::rewind_choices;
use crate::thread::transcript::CommandStatus;
use crate::thread::transcript::MessageRole;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionModel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Rect;
use ratatui::style::Color;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_app_server_protocol::protocol::config::FrontendConfigDto;
use zeta_app_server_protocol::protocol::config::LanguageServerConfigDto;
use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;
use zeta_app_server_protocol::protocol::environment::PermissionDto;
use zeta_app_server_protocol::protocol::environment::SessionDirDto;
use zeta_app_server_protocol::protocol::environment::SessionDirListResult;
use zeta_app_server_protocol::protocol::provider::{
    ProviderApiKeyPolicyDto, ProviderCatalogEntryDto, ProviderListResult,
};
use zeta_app_server_protocol::protocol::skills::{SkillDiagnosticCodeDto, SkillDiagnosticDto};
use zeta_protocol::ApprovalMode;
use zeta_protocol::ContentDigest;
use zeta_protocol::ItemId;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
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
use zeta_terminal_detection::ColorLevel;

#[test]
fn skill_diagnostics_are_notices_and_are_suppressed_until_they_clear() {
    let mut app = App::new();
    let diagnostic = SkillDiagnosticDto {
        source: "user:skill-source:personal".into(),
        subject: Some("broken/SKILL.md".into()),
        code: SkillDiagnosticCodeDto::InvalidFrontmatter,
        message: "frontmatter is invalid".into(),
    };

    app.update(SkillEvent::DiagnosticsReceived(vec![diagnostic.clone()]));
    assert_eq!(app.messages().len(), 2);
    assert!(
        app.messages()
            .iter()
            .all(|message| message.role == MessageRole::Notice)
    );
    assert_eq!(
        app.messages()[1].text,
        "broken/SKILL.md: frontmatter is invalid"
    );

    app.update(SkillEvent::DiagnosticsReceived(vec![diagnostic.clone()]));
    assert_eq!(app.messages().len(), 2);

    app.update(SkillEvent::DiagnosticsReceived(Vec::new()));
    app.update(SkillEvent::DiagnosticsReceived(vec![diagnostic]));
    assert_eq!(app.messages().len(), 4);
}

fn config_session() -> SessionId {
    SessionId::new("config-state-session").unwrap()
}

fn enter_test_session(app: &mut App) {
    app.update(ThreadEvent::ContextChanged {
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
    app.update(ThreadEvent::ApprovalRequested(Approval::new(
        ApprovalSpec {
            title: "Approval required".into(),
            reason: "Run tests".into(),
            details: Vec::new(),
        },
    )));

    app.handle_paste("ignored".into());
    let Some(AppCommand::Thread(ThreadCommand::ResolveRequest(response))) =
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("expected an Approval response");
    };
    let request = response.identity();
    app.update(ThreadEvent::RequestSubmissionFailed {
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
    app.update(ThreadEvent::QueryRequested(
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
fn selected_theme_closes_the_theme_picker_immediately() {
    let mut app = App::new();
    app.update(ThemeEvent::PickerOpened(theme_choices(&theme_catalog())));

    assert_eq!(app.list_selection().unwrap().title(), "Theme");
    let command = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(
        command,
        Some(AppCommand::Theme(ThemeCommand::Set {
            preference: "zeta-code-dark".into(),
        }))
    );
    assert!(app.list_selection().is_none());
}

#[test]
fn selected_render_theme_is_read_through_the_frame_context() {
    let mut app = App::new();
    let theme =
        RenderTheme::from_palette(crate::render::ThemePalette::light(), ColorLevel::TrueColor);

    app.update(ThemeEvent::RenderChanged(theme));

    assert_eq!(app.render_context().background(), Color::Rgb(255, 255, 255));
}

#[test]
fn pointer_activation_uses_the_feature_action_mapping() {
    let mut app = App::new();
    app.update(ThemeEvent::PickerOpened(theme_choices(&theme_catalog())));

    assert_eq!(app.mouse_mode(), MouseMode::TuiCapture);
    assert_eq!(
        app.activate_visible_item(1),
        Some(AppCommand::Theme(ThemeCommand::OpenCustomPicker))
    );
    assert_eq!(
        app.list_selection().unwrap().selected_visible_index(),
        Some(0)
    );
}

#[test]
fn selected_custom_theme_closes_the_entire_theme_flow_immediately() {
    let catalog = theme_catalog();
    let mut app = App::new();
    app.update(ThemeEvent::PickerOpened(theme_choices(&catalog)));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::Theme(ThemeCommand::OpenCustomPicker))
    );
    app.update(ThemeEvent::PickerOpened(custom_theme_choices(&catalog)));
    let command = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(
        command,
        Some(AppCommand::Theme(ThemeCommand::SetCustom {
            preference: "aurora".into(),
        }))
    );
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
        reference_cost: zeta_protocol::ModelReferenceCostSummary::default(),
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
    app.update(ThreadEvent::RewindPickerOpened(rewind_choices(&thread)));

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::Thread(ThreadCommand::RewindToCheckpoint {
            before_turn_id: turn_id,
            checkpoint_label: "restore here".into(),
        }))
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
        focus: Color::Magenta,
        selection_foreground: Color::Magenta,
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
    let Some(AppCommand::Thread(ThreadCommand::SubmitTurn { submission, .. })) = action else {
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

    assert_eq!(
        action,
        Some(AppCommand::Host(HostCommand::ReadClipboardImage))
    );
    assert_eq!(app.status(), &Status::Ready);
}

#[test]
fn control_o_requests_copy_and_control_z_requests_suspend() {
    let mut app = App::new();

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL)),
        Some(AppCommand::Host(HostCommand::CopyLastResponse))
    );
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL)),
        Some(AppCommand::Suspend)
    );
}

#[test]
fn export_slash_command_stays_in_the_terminal_host() {
    let mut app = App::new();
    app.insert_text("/export notes/conversation.md");
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::Host(HostCommand::ExportTranscript {
            requested_path: Some(std::path::PathBuf::from("notes/conversation.md")),
        }))
    );
}

#[test]
fn export_rejects_image_arguments_before_host_io() {
    let mut app = App::new();
    app.insert_text("/export ");
    app.update(HostEvent::ClipboardImageRead(Ok(
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
        Some(AppCommand::Thread(ThreadCommand::LoadOlderHistory))
    );
}

#[test]
fn page_up_at_loaded_start_anchors_the_view_and_requests_older_history() {
    let mut app = App::new();
    app.update(ThreadEvent::FailureReported("only loaded message".into()));

    assert_eq!(
        app.handle_key_in_area(
            KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE),
            Rect::new(0, 0, 80, 24),
        ),
        Some(AppCommand::Thread(ThreadCommand::LoadOlderHistory))
    );
    assert!(app.transcript_scroll().anchor().is_some());
}

#[test]
fn submitting_after_manual_scroll_restores_follow_latest() {
    let mut app = App::new();
    for index in 0..20 {
        app.update(ThreadEvent::FailureReported(format!("failure {index}")));
    }
    assert!(app.scroll_transcript(
        crate::thread::transcript::TranscriptScrollDirection::Up,
        Rect::new(0, 0, 80, 20),
    ));
    assert!(app.transcript_scroll().anchor().is_some());

    app.insert_text("continue");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.transcript_scroll().anchor(), None);
}

#[test]
fn clipboard_png_submits_through_the_existing_attachment_path() {
    let mut app = App::new();
    app.update(HostEvent::ClipboardImageRead(Ok(
        b"\x89PNG\r\n\x1a\npayload".to_vec(),
    )));

    assert_eq!(app.input(), "[Image #1] ");
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let Some(AppCommand::Thread(ThreadCommand::SubmitTurn { submission, .. })) = action else {
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

    assert_eq!(
        action,
        Some(AppCommand::Host(HostCommand::ReadClipboardImage))
    );
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

    let Some(AppCommand::Thread(ThreadCommand::ExecuteProductCommand(invocation))) = action else {
        panic!("expected product command action");
    };
    assert_eq!(invocation.command.name, "status");
    assert_eq!(invocation.origin, SlashCommandOrigin::Local);
    assert!(invocation.arguments.is_empty());
    assert_eq!(app.status(), &Status::Ready);
    assert_eq!(app.messages().len(), 1);
    assert_eq!(app.messages()[0].role, MessageRole::Command);
    assert_eq!(app.messages()[0].text, "/status");
    assert_eq!(
        app.messages()[0].command_status,
        Some(CommandStatus::Submitted)
    );
}

#[test]
fn shortcut_slash_command_is_owned_by_the_local_host() {
    let mut app = App::new();
    app.insert_text("/shortcuts");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, Some(AppCommand::Keymap(KeymapCommand::OpenEditor)));
    assert_eq!(app.messages()[0].text, "/shortcuts");
}

#[test]
fn config_slash_command_is_owned_by_the_local_host() {
    let mut app = App::new();
    app.insert_text("/config");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, Some(AppCommand::Config(ConfigCommand::OpenEditor)));
    assert_eq!(app.messages()[0].text, "/config");
}

#[test]
fn startup_slash_command_opens_a_read_only_context_panel() {
    let mut app = App::new();
    app.insert_text("/startup");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, None);
    assert_eq!(
        app.list_selection().map(|selection| selection.title()),
        Some("Startup")
    );
    assert_eq!(app.command_panel_key_hints(), Some("Esc to close"));
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    assert!(app.command_panel().is_some());
}

#[test]
fn theme_slash_command_is_owned_by_the_tui_host() {
    let mut app = App::new();
    app.insert_text("/theme");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, Some(AppCommand::Theme(ThemeCommand::OpenPicker)));
}

#[test]
fn inline_theme_selection_requests_a_tui_config_edit() {
    let mut app = App::new();
    app.insert_text("/theme zeta-code-light");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        action,
        Some(AppCommand::Theme(ThemeCommand::Set {
            preference: "zeta-code-light".into(),
        }))
    );
}

#[test]
fn config_mouse_selection_emits_a_revision_bound_edit() {
    let mut config = empty_config_snapshot();
    config.revision = 7;
    let mut app = App::new();
    app.update(ConfigEvent::EditorOpened(config_choices(
        &config,
        &ProviderListResult { providers: vec![] },
        TerminalSettings::default(),
        StatusLineSettings::default(),
    )));

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(
        action,
        Some(AppCommand::Config(ConfigCommand::Edit(edit)))
            if edit.server_config.revision == 7
                && !edit.terminal.mouse_interactions()
    ));
}

#[test]
fn config_vim_mode_toggles_on_enter() {
    let mut config = empty_config_snapshot();
    config.revision = 7;
    let mut app = App::new();
    app.update(ConfigEvent::EditorOpened(config_choices(
        &config,
        &ProviderListResult { providers: vec![] },
        TerminalSettings::default(),
        StatusLineSettings::default(),
    )));
    for _ in 0..1 {
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }

    assert!(matches!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::Config(ConfigCommand::Edit(edit)))
            if edit.server_config.revision == 7
                && edit.terminal.input_mode() == ChatInputMode::Vim
    ));
}

#[test]
fn config_show_git_changes_as_diff_toggles_on_enter() {
    let mut config = empty_config_snapshot();
    config.revision = 7;
    let mut app = App::new();
    app.update(ConfigEvent::EditorOpened(config_choices(
        &config,
        &ProviderListResult { providers: vec![] },
        TerminalSettings::default(),
        StatusLineSettings::default(),
    )));
    for _ in 0..2 {
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }

    assert!(matches!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::Config(ConfigCommand::Edit(edit)))
            if edit.status_line.show_git_changes_as_diff()
    ));
}

#[test]
fn config_language_server_switch_emits_a_revision_bound_backend_edit() {
    let mut config = empty_config_snapshot();
    config.revision = 7;
    config.language_servers.insert(
        "rust-analyzer".into(),
        LanguageServerConfigDto {
            mode: LanguageServerModeDto::Disabled,
            executable: None,
        },
    );
    let mut app = App::new();
    app.update(ConfigEvent::EditorOpened(config_choices(
        &config,
        &ProviderListResult {
            providers: vec![ProviderCatalogEntryDto {
                provider: "ollama".into(),
                display_name: "Ollama".into(),
                api_key_policy: ProviderApiKeyPolicyDto::Unsupported,
                api_key_configured: false,
            }],
        },
        TerminalSettings::default(),
        StatusLineSettings::default(),
    )));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(
        app.list_selection().unwrap().active_tab().label(),
        "Language servers"
    );
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    assert!(matches!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::Config(ConfigCommand::SetLanguageServerMode(edit)))
            if edit.expected_revision == 7
                && edit.server_id == "rust-analyzer"
                && edit.config.mode == LanguageServerModeDto::Enabled
    ));
}

#[test]
fn directory_permission_selection_emits_a_revision_bound_server_edit() {
    let directories = SessionDirListResult {
        revision: 3,
        dirs: vec![SessionDirDto {
            contributions: Default::default(),
            path: "/dir/shared".into(),
            permissions: vec![PermissionDto::ReadFiles, PermissionDto::WriteFiles],
        }],
    };
    let mut app = App::new();
    app.update(crate::dirs::Event::PickerOpened(crate::dirs::choices(
        &config_session(),
        directories,
    )));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(
        action,
        Some(AppCommand::Dirs(DirCommand::SetPermissions(params)))
            if params.expected_revision == 3
                && params.permissions == vec![PermissionDto::WriteFiles]
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
    app.update(ConfigEvent::EditorOpened(config_choices(
        &config,
        &providers,
        TerminalSettings::default(),
        StatusLineSettings::default(),
    )));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let Some(AppCommand::Config(ConfigCommand::SetProviderApiKey(edit))) = action else {
        panic!("provider API key input must emit a typed secret edit");
    };
    assert!(!format!("{edit:?}").contains("api_key: \"sk\""));
    assert_eq!(edit.into_parts(), ("openai".into(), "sk".into()));

    app.update(ConfigEvent::ApiKeySaved {
        provider: "openai".into(),
        choices: config_choices(
            &config,
            &providers,
            TerminalSettings::default(),
            StatusLineSettings::default(),
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
    app.update(ConfigEvent::EditorOpened(config_choices(
        &config,
        &providers,
        TerminalSettings::default(),
        StatusLineSettings::default(),
    )));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
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

    assert_eq!(
        action,
        Some(AppCommand::Status(StatusCommand::OpenLineEditor))
    );
    assert_eq!(app.messages()[0].text, "/statusline");
}

#[test]
fn statusline_selection_emits_a_revision_bound_edit() {
    let settings = StatusLineSettings::default();
    let choices = status_line_choices(&settings, 7);
    let mut app = App::new();
    app.update(StatusEvent::LineEditorOpened(StatusLineEditorUpdate {
        settings,
        choices,
    }));

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(
        action,
        Some(AppCommand::Status(StatusCommand::EditLine(edit)))
            if edit.expected_revision == 7
                && edit.item == StatusLineItem::Permissions
                && !edit.enabled
    ));
}

#[test]
fn statusline_accounting_follows_the_current_thread_context() {
    let mut app = App::new();
    let mut settings = StatusLineSettings::default();
    settings.set(StatusLineItem::CacheHitRate, true);
    settings.set(StatusLineItem::ReferenceCost, true);
    settings.set(StatusLineItem::Permissions, false);
    settings.set(StatusLineItem::Model, false);
    settings.set(StatusLineItem::GitBranch, false);
    settings.set(StatusLineItem::GitChanges, false);
    app.update(StatusEvent::LineSettingsReceived(settings));
    let mut usage = zeta_protocol::ModelUsageSummary::default();
    usage.model_invocations = 1;
    usage.input_tokens.reported = 2_000;
    usage.cached_input_tokens.reported = 500;
    app.update(ThreadEvent::AccountingChanged {
        usage,
        reference_cost: zeta_protocol::ModelReferenceCostSummary {
            known_amounts: vec![zeta_protocol::ModelMoneyAmount {
                currency: "USD".into(),
                pico_units: "1250000000".into(),
            }],
            complete: true,
        },
    });

    assert_eq!(
        app.status_line()
            .top_text_for_width(80, app.status_line_runtime()),
        "cache hit 25.0% · cost $0.00125"
    );

    app.update(ThreadEvent::ContextChanged {
        session_id: zeta_protocol::SessionId::new("session-next").unwrap(),
        thread_id: zeta_protocol::ThreadId::new("thread-next").unwrap(),
    });

    assert_eq!(
        app.status_line()
            .top_text_for_width(80, app.status_line_runtime()),
        ""
    );
}

#[test]
fn process_resource_sample_updates_the_optional_statusline_items() {
    let mut app = App::new();
    let request = ProcessResourceRequest {
        revision: 1,
        cpu_cycle: 1,
        demand: ProcessResourceDemand::Processes,
    };
    app.apply_process_resource_request(request);

    app.update(HostEvent::ProcessResourcesSampled(
        ProcessResourcesReading {
            request,
            tui: Ok(ProcessResourceUsage {
                resident_bytes: Some(128 * 1024 * 1024),
                cpu_tenths_percent: Some(124),
            }),
            app_server: None,
            sampled_at: Instant::now(),
        },
    ));
    assert_eq!(
        app.status_line_runtime().process_resources.local.memory,
        ProcessMemoryCurrent::Available(128 * 1024 * 1024)
    );

    let mut settings = StatusLineSettings::default();
    for item in StatusLineItem::ALL {
        settings.set(
            item,
            matches!(item, StatusLineItem::Memory | StatusLineItem::Cpu),
        );
    }
    app.update(StatusEvent::LineSettingsReceived(settings));

    assert_eq!(
        app.status_line()
            .top_text_for_width(80, app.status_line_runtime()),
        "memory 128.0 MiB · cpu 12.4%"
    );
}

#[test]
fn shortcut_capture_emits_a_revision_bound_edit() {
    let mut app = App::new();
    let settings = keymap_settings_from_tui(&Default::default()).unwrap();
    let choices = keymap_choices(settings.keymap.setup_actions(), &[], 7);
    app.update(KeymapEvent::EditorOpened(KeymapEditorUpdate {
        settings,
        choices,
        notice: None,
    }));

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
    assert!(app.list_selection().is_none());

    let edit = app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
    assert_eq!(
        edit,
        Some(AppCommand::Keymap(KeymapCommand::Edit(
            crate::keymap::KeymapEdit {
                expected_revision: 7,
                command_id: "zetaCode.action.cycleApprovalMode".into(),
                kind: KeymapEditKind::Set {
                    key: "ctrl+y".into(),
                    intent: KeymapEditIntent::ReplaceUser,
                },
            }
        )))
    );
}

#[test]
fn inline_product_arguments_reach_the_typed_dispatcher() {
    let mut app = App::new();
    app.insert_text("/model provider/model");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let Some(AppCommand::Thread(ThreadCommand::ExecuteProductCommand(invocation))) = action else {
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
    assert_eq!(app.messages()[0].text, "/model provider/model");
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
        Some(AppCommand::Thread(ThreadCommand::SubmitTurn {
            submission: ChatSubmission {
                display_text: "/diagnose logs".into(),
                input: vec![ChatInputItem::Text("/diagnose logs".into())],
            },
        }))
    );
    assert_eq!(app.status(), &Status::Working);
    assert_eq!(app.messages()[0].role, MessageRole::User);
    assert_eq!(app.messages()[0].text, "/diagnose logs");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn help_uses_the_runtime_command_catalog_and_descriptions() {
    let dir = temporary_dir("runtime-slash-help");
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
    let settings = keymap_settings_from_tui(&FrontendConfigDto(BTreeMap::from([(
        "keybindings".into(),
        serde_json::json!([{
            "key": "ctrl+y",
            "command": "zetaCode.action.copyLastResponse"
        }]),
    )])))
    .unwrap();
    app.update(KeymapEvent::SettingsReceived(settings));
    app.insert_text("/help");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, None);
    assert_eq!(app.status(), &Status::Ready);
    let selection = app.list_selection().unwrap();
    assert_eq!(selection.active_tab().label(), "Shortcuts");
    assert_eq!(
        selection
            .tabs()
            .iter()
            .map(ListSelectionGroup::label)
            .collect::<Vec<_>>(),
        vec!["Shortcuts", "Commands", "Custom commands"]
    );
    let shortcuts = selection.visible_items();
    assert!(shortcuts.iter().any(|item| item.label() == "Esc Esc"));
    assert!(!shortcuts.iter().any(|item| matches!(
        item.label(),
        "Tab" | "Home / End" | "PageUp / PageDown" | "Ctrl-Home / Ctrl-End"
    )));
    let custom = shortcuts
        .iter()
        .find(|item| item.label() == "ctrl+y")
        .expect("the active user shortcut is included in help");
    assert_eq!(custom.description(), Some("Copy last response · custom"));

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let selection = app.list_selection().unwrap();
    assert_eq!(selection.active_tab().label(), "Commands");
    let commands = selection.visible_items();
    let status = commands
        .iter()
        .find(|item| item.label() == "/status")
        .expect("the local command is included in help");
    assert_eq!(
        status.description(),
        Some("show the active session, thread, and model")
    );
    assert!(commands.iter().all(|item| item.label() != "/diagnose"));

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let selection = app.list_selection().unwrap();
    assert_eq!(selection.active_tab().label(), "Custom commands");
    let custom_commands = selection.visible_items();
    let diagnose = custom_commands
        .iter()
        .find(|item| item.label() == "/diagnose")
        .expect("the server command is included in custom commands");
    assert_eq!(diagnose.description(), Some("inspect the current dir"));
    assert!(custom_commands.iter().all(|item| item.label() != "/status"));
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
    app.replace_chat_input_catalog(crate::thread::composer::ChatInputCatalog::new(
        registry,
        vec![crate::thread::composer::SkillCompletionItem::new(
            "commit".into(),
            "draft a commit message".into(),
            skill.clone(),
        )],
        Vec::new(),
    ));
    app.insert_text("$com");
    assert!(matches!(
        app.completion(),
        Some(CompletionView::Skill(view)) if view.items[0].name() == "commit"
    ));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.insert_text("staged changes");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        action,
        Some(AppCommand::Thread(ThreadCommand::SubmitTurn {
            submission: ChatSubmission {
                display_text: "$commit staged changes".into(),
                input: vec![
                    ChatInputItem::Skill { skill },
                    ChatInputItem::Text("$commit staged changes".into()),
                ],
            },
        }))
    );
    assert_eq!(app.messages()[0].text, "$commit staged changes");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn activating_a_slash_command_by_index_uses_the_command_dispatch_path() {
    let mut app = App::new();
    app.insert_text("/q");

    let action = app.activate_input_completion(0);

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
    app.update(ConfigEvent::SettingsReceived(settings));

    assert_eq!(app.mouse_mode(), MouseMode::TerminalSelection);
    assert!(app.screen_selection().range().is_none());
}

#[test]
fn terminal_settings_apply_vim_mode_to_the_active_chat_input() {
    let mut app = App::new();
    let mut settings = TerminalSettings::default();
    settings.set_input_mode(ChatInputMode::Vim);

    app.update(ConfigEvent::SettingsReceived(settings));
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
        app.completion(),
        Some(CompletionView::Mention(view)) if view.matches[0].label == "src/lib.rs"
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
fn escape_dismisses_an_at_file_popup_and_is_inert_on_the_session_screen() {
    let dir = temporary_dir("mention-dismiss");
    fs::write(dir.join("notes.md"), "notes").unwrap();
    let mut app = App::for_dir(&dir);
    app.insert_text("@notes");

    let dismissed = app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(dismissed, None);
    assert!(app.completion().is_none());
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        None
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn escape_does_not_exit_the_idle_session_screen() {
    let mut app = App::new();

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        None
    );
    assert_eq!(app.status(), &Status::Ready);
}

#[test]
fn terminal_screen_change_closes_command_panels_including_status() {
    let mut app = App::new();
    app.update(AppEvent::HelpOpened(ListSelectionModel::new(
        "Help",
        vec![ListSelectionGroup::new(
            "Commands",
            vec![ListSelectionItem::new("/status")],
        )],
    )));
    assert!(app.command_panel().is_some());

    enter_test_session(&mut app);
    assert!(app.command_panel().is_none());

    let usage = zeta_protocol::ModelUsageSummary::default();
    let reference_cost = zeta_protocol::ModelReferenceCostSummary::default();
    app.update(StatusEvent::PanelOpened(status_panel(StatusViewData {
        model: "openai/gpt",
        full_context_window: None,
        available_context_window: None,
        remaining_context_window: crate::status::RemainingContextWindow::Unknown,
        usage: &usage,
        reference_cost: &reference_cost,
        session_id: "session-1",
        thread_id: "thread-1",
        thread_sequence: 1,
    })));
    assert!(app.command_panel().is_some());
    app.update(ThreadEvent::ContextChanged {
        session_id: SessionId::new("other-session").unwrap(),
        thread_id: ThreadId::new("other-thread").unwrap(),
    });
    assert!(app.command_panel().is_none());
}

#[test]
fn two_screen_escape_presses_within_the_gesture_window_open_rewind() {
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
        Some(AppCommand::Thread(ThreadCommand::OpenRewindPicker))
    );
}

#[test]
fn screen_escape_gesture_preserves_a_nonempty_draft() {
    let mut app = App::new();
    let started = Instant::now();
    app.insert_text("keep this draft");

    assert_eq!(
        app.handle_key_at(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), started),
        None
    );
    assert_eq!(
        app.handle_key_at(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            started + Duration::from_millis(200),
        ),
        None
    );
    assert_eq!(app.input(), "keep this draft");
}

#[test]
fn escape_from_command_panel_does_not_count_toward_the_screen_rewind_sequence() {
    let mut app = App::new();
    let started = Instant::now();
    assert_eq!(
        app.handle_key_at(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), started),
        None
    );
    app.update(AppEvent::HelpOpened(ListSelectionModel::new(
        "Feature",
        vec![ListSelectionGroup::new(
            "Items",
            vec![ListSelectionItem::new("Item")],
        )],
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
fn screen_escape_gesture_expires_and_is_reset_by_other_input() {
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

    assert_eq!(action, Some(AppCommand::Thread(ThreadCommand::Interrupt)));
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
    app.update(ThreadEvent::TurnActivityChanged(
        TurnActivity::WaitingForUserInput,
    ));

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(action, Some(AppCommand::Thread(ThreadCommand::Interrupt)));
    assert_eq!(app.status(), &Status::Cancelling);
}

#[test]
fn control_enter_steers_the_working_turn_and_tracks_delivery() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(ThreadEvent::TurnActivityChanged(TurnActivity::Working));

    app.insert_text("second");
    app.handle_paste("third".into());
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL));

    let Some(AppCommand::Thread(ThreadCommand::SteerTurn {
        source,
        steer_id,
        submission,
    })) = action
    else {
        panic!("expected active Turn steer");
    };
    assert_eq!(source, SteerSource::Composer);
    assert_eq!(submission.display_text, "secondthird");
    assert_eq!(app.input(), "");
    assert_eq!(app.messages().len(), 2);
    assert_eq!(app.messages()[1].text, "secondthird");
    assert!(app.command_panel().is_none());

    app.update(ThreadEvent::SteerCompleted { source, steer_id });

    assert!(app.command_panel().is_none());
    assert_eq!(app.status(), &Status::Working);
}

#[test]
fn queue_selection_can_steer_an_older_message_immediately() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(ThreadEvent::TurnActivityChanged(TurnActivity::Working));
    for message in ["second", "third"] {
        app.insert_text(message);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    }
    let second_id = app.queue_view().items[0].id;

    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL));

    let Some(AppCommand::Thread(ThreadCommand::SteerTurn {
        source: SteerSource::Queue(queue_id),
        steer_id,
        submission,
    })) = action
    else {
        panic!("expected the selected Queue message to steer");
    };
    assert_eq!(queue_id, second_id);
    assert_eq!(submission.display_text, "second");
    assert!(app.queue_view().items[0].sending);
    assert!(!app.queue_focused());

    app.update(ThreadEvent::SteerCompleted {
        source: SteerSource::Queue(queue_id),
        steer_id,
    });

    assert_eq!(app.queue_view().items.len(), 1);
    assert_eq!(app.queue_view().items[0].text, "third");
}

#[test]
fn enter_queues_a_new_turn_by_default_while_the_current_turn_is_working() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(ThreadEvent::TurnActivityChanged(TurnActivity::Working));
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
fn alt_up_focuses_the_queue_and_enter_restores_the_selected_message() {
    let mut app = App::new();
    app.update(ThreadEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("restore me");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT)),
        None
    );
    assert!(app.queue_focused());
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.input(), "restore me");
    assert!(!app.queue_focused());
    assert!(app.queue_view().items[0].editing);
}

#[test]
fn queue_focus_keeps_global_interrupt_available() {
    let mut app = App::new();
    app.update(ThreadEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("queued");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Some(AppCommand::Thread(ThreadCommand::Interrupt))
    );
}

#[test]
fn sessions_and_agents_commands_open_the_manager_screen() {
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
fn manager_keys_operate_on_sessions_while_group_headings_remain_static() {
    let mut app = App::new();
    app.update(SessionEvent::CatalogReceived(vec![
        manager_state_session("one"),
        manager_state_session("two"),
    ]));
    app.insert_text("/sessions");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

    assert_eq!(
        app.session_manager_hint(),
        "Space to preview · Ctrl+X to archive"
    );
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL)),
        Some(AppCommand::Sessions(SessionCommand::Archive {
            session_ids: vec![SessionId::new("one").unwrap()],
        }))
    );

    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
        None
    );
    assert_eq!(app.overlay().unwrap().title(), "Session preview");
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.overlay().is_none());
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        app.session_manager_hint(),
        "Space to preview · Ctrl+X to archive"
    );
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::Sessions(SessionCommand::Resume {
            session_id: "two".into(),
            preferred_thread_id: None,
        }))
    );
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL)),
        Some(AppCommand::Sessions(SessionCommand::Archive {
            session_ids: vec![SessionId::new("two").unwrap()],
        }))
    );
}

#[test]
fn queued_turn_stays_editable_when_automatic_submission_is_rejected() {
    let mut app = App::new();
    app.update(ThreadEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("keep this message");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(ThreadEvent::TurnCompleted);

    let Some(AppCommand::Thread(ThreadCommand::SubmitQueuedTurn {
        queue_id,
        submission,
    })) = app.dispatch_next_queued_turn()
    else {
        panic!("expected queued Turn dispatch");
    };
    assert_eq!(submission.display_text, "keep this message");
    assert!(app.queue_view().items[0].sending);

    app.update(ThreadEvent::QueueSubmissionFailed {
        queue_id,
        error: "server unavailable".into(),
    });
    assert!(!app.queue_view().items[0].sending);
}

#[test]
fn empty_enter_does_not_bypass_queue_dispatch_order() {
    let mut app = App::new();
    app.update(ThreadEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("send this now");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None
    );
    assert_eq!(app.queue_view().items[0].text, "send this now");
}

#[test]
fn a_created_turn_does_not_claim_the_running_steer_action() {
    let mut app = App::new();
    app.update(ThreadEvent::TurnActivityChanged(TurnActivity::Starting));
    app.insert_text("after the queued turn");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, None);
    assert_eq!(app.queue_view().items[0].text, "after the queued turn");
    assert_eq!(app.status(), &Status::Working);
}

#[test]
fn rejected_steer_removes_only_its_pending_row_and_keeps_the_turn_working() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(ThreadEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("change direction");
    let Some(AppCommand::Thread(ThreadCommand::SteerTurn { steer_id, .. })) =
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL))
    else {
        panic!("expected active Turn steer");
    };

    app.update(ThreadEvent::SteerSubmissionFailed {
        source: SteerSource::Composer,
        steer_id,
        error: "sequence conflict".into(),
    });

    assert!(app.command_panel().is_none());
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

    app.update(ThreadEvent::TurnCompleted);

    assert_eq!(app.messages().len(), 1);
    assert_eq!(app.messages()[0].role, MessageRole::User);
    assert_eq!(app.messages()[0].text, "hello");
    assert_eq!(app.status(), &Status::Ready);
}

#[test]
fn client_error_is_visible_in_history_and_status() {
    let mut app = App::new();

    app.update(ThreadEvent::FailureReported("provider unavailable".into()));

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

    app.update(ThreadEvent::TurnInterrupted);

    assert_eq!(app.status(), &Status::Ready);
    assert_eq!(app.messages().last().unwrap().role, MessageRole::Notice);
    assert_eq!(app.messages().last().unwrap().text, "turn interrupted");
}

fn assert_text_submission(action: Option<AppCommand>, expected: &str) {
    let Some(AppCommand::Thread(ThreadCommand::SubmitTurn { submission })) = action else {
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
    assert_eq!(
        action,
        Some(AppCommand::Thread(ThreadCommand::CycleNextApprovalMode))
    );
    assert_eq!(app.approval_mode(), ApprovalMode::AskPermissions);

    app.set_next_approval_mode(ApprovalMode::AutoReview);
    let action = app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(
        action,
        Some(AppCommand::Thread(ThreadCommand::CycleNextApprovalMode))
    );
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
            app.update(ThreadEvent::FileSearchSnapshotReceived(snapshot));
        }
        if matches!(
            app.completion(),
            Some(CompletionView::Mention(popup)) if !popup.matches.is_empty()
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

fn manager_state_session(id: &str) -> Session {
    Session {
        session_id: SessionId::new(id).unwrap(),
        title: id.into(),
        status: SessionStatus::Active,
        manager: Default::default(),
        threads: Vec::new(),
    }
}
