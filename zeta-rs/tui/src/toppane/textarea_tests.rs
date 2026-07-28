use super::TextArea;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn editing_uses_unicode_cursor_boundaries() {
    let mut textarea = TextArea::new();
    textarea.insert_text("你a");

    textarea.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    textarea.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    assert_eq!(textarea.text(), "a");
    assert_eq!(textarea.cursor_display_width(), 0);
}

#[test]
fn paste_is_inserted_at_the_cursor() {
    let mut textarea = TextArea::new();
    textarea.insert_text("ac");
    textarea.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));

    textarea.insert_text("b");

    assert_eq!(textarea.text(), "abc");
    assert_eq!(textarea.cursor_display_width(), 2);
}

#[test]
fn control_keys_are_left_for_parent_routing() {
    let mut textarea = TextArea::new();

    let outcome = textarea.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(outcome, super::TextAreaOutcome::Unhandled);
    assert_eq!(textarea.text(), "");
}
