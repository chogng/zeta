use super::Action;
use super::App;
use super::MessageRole;
use super::Status;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn enter_submits_trimmed_input_and_records_the_user_message() {
    let mut app = App::new();
    app.insert_text("  explain this  ");

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(action, Some(Action::Submit("explain this".into())));
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
fn control_c_requests_exit() {
    let mut app = App::new();

    let action = app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(action, Some(Action::Quit));
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
    assert_eq!(app.status(), &Status::Error("provider unavailable".into()));
}
