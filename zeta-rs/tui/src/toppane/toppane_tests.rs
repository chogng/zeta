use super::TopPane;
use super::TopPaneOutcome;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn pane_routes_submission_from_the_composer() {
    let mut pane = TopPane::new();
    pane.insert_text("hello");

    let outcome = pane.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(outcome, TopPaneOutcome::Submit("hello".into()));
    assert_eq!(pane.text(), "");
}
