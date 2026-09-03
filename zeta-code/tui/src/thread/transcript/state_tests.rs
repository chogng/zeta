use super::ChatHistoryScroll;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn transcript_scroll_moves_relative_to_the_bottom_and_can_follow_latest() {
    let mut scroll = ChatHistoryScroll::default();
    assert_eq!(scroll.paragraph_offset(20), 20);

    assert!(scroll.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)));
    assert_eq!(scroll.paragraph_offset(20), 15);
    assert!(scroll.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL)));
    assert_eq!(scroll.paragraph_offset(20), 0);

    scroll.follow_latest();
    assert_eq!(scroll.paragraph_offset(20), 20);
}
