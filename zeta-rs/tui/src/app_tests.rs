use super::Action;
use super::App;
use super::MessageRole;
use super::Status;
use crate::toppane::ComposerInput;
use crate::toppane::ComposerSubmission;
use crate::toppane::DynamicSlashCommand;
use crate::toppane::SlashCommand;
use crate::toppane::SlashCommandArgumentMode;
use crate::toppane::SlashCommandItem;
use crate::toppane::SlashCommandRegistry;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use std::fs;
use std::time::Duration;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

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
    let Some(Action::Submit(submission)) = action else {
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

    assert_eq!(action, Some(Action::PasteImage));
    assert_eq!(app.status(), &Status::Ready);
}

#[test]
fn clipboard_png_submits_through_the_existing_attachment_path() {
    let mut app = App::new();
    app.attach_image_bytes(b"\x89PNG\r\n\x1a\npayload".to_vec());

    assert_eq!(app.input(), "[Image #1] ");
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let Some(Action::Submit(submission)) = action else {
        panic!("expected image submission");
    };
    assert_eq!(submission.display_text, "[Image #1]");
    assert!(matches!(
        &submission.input[0],
        ComposerInput::Image { url } if url.starts_with("data:image/png;base64,")
    ));
}

#[test]
fn active_turn_ignores_the_clipboard_image_shortcut() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));

    assert_eq!(action, None);
    assert_eq!(app.input(), "");
}

#[test]
fn control_c_requests_exit() {
    let mut app = App::new();

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(action, Some(Action::Quit));
}

#[test]
fn quit_slash_command_requests_exit_without_starting_a_turn() {
    let mut app = App::new();
    app.insert_text("/quit");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, Some(Action::Quit));
    assert_eq!(app.status(), &Status::Ready);
    assert!(app.messages().is_empty());
}

#[test]
fn product_command_is_delegated_to_the_typed_dispatcher() {
    let mut app = App::new();
    app.insert_text("/status");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let Some(Action::Command(invocation)) = action else {
        panic!("expected product command action");
    };
    assert_eq!(
        invocation.command,
        SlashCommandItem::Builtin(SlashCommand::Status)
    );
    assert!(invocation.arguments.is_empty());
    assert_eq!(app.status(), &Status::Ready);
    assert!(app.messages().is_empty());
}

#[test]
fn inline_product_arguments_reach_the_typed_dispatcher() {
    let mut app = App::new();
    app.insert_text("/model provider/model");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let Some(Action::Command(invocation)) = action else {
        panic!("expected product command action");
    };
    assert_eq!(
        invocation.command,
        SlashCommandItem::Builtin(SlashCommand::Model)
    );
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
    let registry = SlashCommandRegistry::with_dynamic_commands([DynamicSlashCommand {
        name: "diagnose".into(),
        description: "inspect the current workspace".into(),
        argument_mode: SlashCommandArgumentMode::Optional,
    }])
    .unwrap();
    let mut app = App::for_workspace_with_slash_commands(&workspace, registry);
    app.insert_text("/diag logs");
    app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.input(), "/diagnose logs");
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        action,
        Some(Action::Submit(ComposerSubmission {
            display_text: "/diagnose logs".into(),
            input: vec![ComposerInput::Text("/diagnose logs".into())],
        }))
    );
    assert_eq!(app.status(), &Status::Working);
    assert_eq!(app.messages()[0].role, MessageRole::User);
    assert_eq!(app.messages()[0].text, "/diagnose logs");
    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn activating_a_slash_command_by_index_uses_the_command_dispatch_path() {
    let mut app = App::new();
    app.insert_text("/q");

    let action = app.activate_slash_command(0);

    assert_eq!(action, Some(Action::Quit));
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

    assert_eq!(action, Some(Action::Quit));
    assert!(app.input().is_empty());
    assert!(app.messages().is_empty());
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
    wait_for_mention_results(&mut app);

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
fn escape_dismisses_an_at_file_popup_before_requesting_exit() {
    let workspace = temporary_workspace("mention-dismiss");
    fs::write(workspace.join("notes.md"), "notes").unwrap();
    let mut app = App::for_workspace(&workspace);
    app.insert_text("@notes");

    let dismissed = app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(dismissed, None);
    assert_eq!(app.mention_popup(), None);
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        Some(Action::Quit)
    );
    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn control_c_interrupts_a_working_turn_without_exiting() {
    let mut app = App::new();
    app.insert_text("hello");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(action, Some(Action::Interrupt));
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
    app.wait_for_user_input();

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(action, Some(Action::Interrupt));
    assert_eq!(app.status(), &Status::Cancelling);
}

#[test]
fn working_turn_does_not_accept_a_second_prompt_or_paste() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.insert_text("second");
    app.handle_paste("third".into());

    assert_eq!(action, None);
    assert_eq!(app.input(), "");
    assert_eq!(app.messages().len(), 1);
}

#[test]
fn response_returns_the_app_to_ready() {
    let mut app = App::new();
    app.insert_text("hello");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    app.record_response("hi".into());

    assert_eq!(app.messages().len(), 2);
    assert_eq!(app.messages()[1].role, MessageRole::Agent);
    assert_eq!(app.messages()[1].text, "hi");
    assert_eq!(app.status(), &Status::Ready);
}

#[test]
fn client_error_is_visible_in_history_and_status() {
    let mut app = App::new();

    app.record_error("provider unavailable".into());

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

    app.record_interrupted();

    assert_eq!(app.status(), &Status::Ready);
    assert_eq!(app.messages().last().unwrap().role, MessageRole::Notice);
    assert_eq!(app.messages().last().unwrap().text, "turn interrupted");
}

fn assert_text_submission(action: Option<Action>, expected: &str) {
    let Some(Action::Submit(submission)) = action else {
        panic!("expected text submission");
    };
    assert_eq!(submission.display_text, expected);
    assert_eq!(
        submission.input,
        vec![ComposerInput::Text(expected.to_owned())]
    );
}

fn wait_for_mention_results(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.poll_background_events();
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
