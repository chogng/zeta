use super::draw;
use crate::components::search_box::SearchBoxModel;
use crate::components::search_box::SearchBoxState;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Position;
use ratatui::style::Color;

#[test]
fn active_search_places_the_terminal_cursor_after_the_query() {
    let mut search = SearchBoxState::new(SearchBoxModel::new("Search"));
    search.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    search.handle_key(KeyEvent::new(KeyCode::Char('界'), KeyModifiers::NONE));
    search.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    let backend = TestBackend::new(30, 3);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| draw(frame, frame.area(), &search, Color::Blue))
        .unwrap();

    terminal
        .backend_mut()
        .assert_cursor_position(Position::new(4, 1));
}
