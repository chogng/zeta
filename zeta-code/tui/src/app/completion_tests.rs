use super::report_turn_start_failure;
use crate::app::App;
use crate::app::Status;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_protocol::TurnId;

#[test]
fn turn_start_failure_preserves_an_active_turn_that_appeared_during_the_request() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.set_active_turn(TurnId::new("turn_1").unwrap());

    report_turn_start_failure(&mut app, "sequence conflict".into());

    assert_eq!(app.status(), &Status::Working);
    assert!(
        app.messages()
            .last()
            .unwrap()
            .text
            .contains("could not start the Turn: sequence conflict")
    );
}

#[test]
fn initial_turn_failure_enters_error_state() {
    let mut app = App::new();
    app.insert_text("first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    report_turn_start_failure(&mut app, "server unavailable".into());

    assert_eq!(app.status(), &Status::Error);
}
