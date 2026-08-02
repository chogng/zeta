use super::SearchBoxInputOutcome;
use super::SearchBoxModel;
use super::SearchBoxState;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn search_box() -> SearchBoxState {
    SearchBoxState::new(SearchBoxModel::new("Search available skills"))
}

#[test]
fn search_box_is_visible_but_not_input_active_by_default() {
    let search = search_box();

    assert_eq!(search.placeholder(), "Search available skills");
    assert_eq!(search.query(), "");
    assert!(!search.input_active());
}

#[test]
fn space_enters_input_mode_before_becoming_query_text() {
    let mut search = search_box();

    assert_eq!(
        search.handle_key(key(KeyCode::Char(' '))),
        SearchBoxInputOutcome::Consumed
    );
    assert!(search.input_active());
    assert_eq!(search.query(), "");
    assert_eq!(
        search.handle_key(key(KeyCode::Char(' '))),
        SearchBoxInputOutcome::QueryChanged
    );
    assert_eq!(search.query(), " ");
}

#[test]
fn text_and_paste_only_change_an_active_search() {
    let mut search = search_box();

    assert_eq!(
        search.handle_key(key(KeyCode::Char('s'))),
        SearchBoxInputOutcome::Ignored
    );
    assert_eq!(
        search.handle_paste("status".into()),
        SearchBoxInputOutcome::Ignored
    );
    search.handle_key(key(KeyCode::Char(' ')));
    search.handle_key(key(KeyCode::Char('s')));
    search.handle_paste("how\nstatus".into());

    assert_eq!(search.query(), "show status");
}

#[test]
fn escape_clears_and_exits_input_mode() {
    let mut search = search_box();
    search.handle_key(key(KeyCode::Char(' ')));
    search.handle_key(key(KeyCode::Char('s')));

    assert_eq!(
        search.handle_key(key(KeyCode::Esc)),
        SearchBoxInputOutcome::QueryChanged
    );
    assert_eq!(search.query(), "");
    assert!(!search.input_active());
    assert_eq!(
        search.handle_key(key(KeyCode::Esc)),
        SearchBoxInputOutcome::Ignored
    );
}

#[test]
fn repeated_escape_is_consumed_after_the_press_exits_search() {
    let mut search = search_box();
    search.handle_key(key(KeyCode::Char(' ')));

    assert_eq!(
        search.handle_key(key(KeyCode::Esc)),
        SearchBoxInputOutcome::QueryChanged
    );
    assert_eq!(
        search.handle_key(KeyEvent::new_with_kind(
            KeyCode::Esc,
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        )),
        SearchBoxInputOutcome::Consumed
    );
    assert!(!search.input_active());
}
