use super::report_turn_start_failure;
use crate::app::App;
use crate::app::Status;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_protocol::TurnId;

#[test]
fn queued_follow_up_failure_preserves_the_running_turn() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let active_turn = Some(TurnId::new("turn_1").unwrap());

    report_turn_start_failure(&mut app, &active_turn, "sequence conflict".into());

    assert_eq!(app.status(), &Status::Working);
    assert!(
        app.messages()
            .last()
            .unwrap()
            .text
            .contains("could not queue the follow-up: sequence conflict")
    );
}

#[test]
fn initial_turn_failure_enters_error_state() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    report_turn_start_failure(&mut app, &None, "server unavailable".into());

    assert_eq!(app.status(), &Status::Error);
}
