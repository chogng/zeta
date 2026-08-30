use super::draw;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionModel;
use crate::components::list_selection::ListSelectionState;
use crate::components::search_box::SearchBoxModel;
use crate::render::test_context;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

fn state() -> ListSelectionState {
    ListSelectionState::new(
        ListSelectionModel::new(
            "Skills",
            vec![ListSelectionGroup::new(
                "All (1)",
                vec![ListSelectionItem::new("skill-creator")],
            )],
        )
        .with_search(SearchBoxModel::new("Search available skills")),
    )
}

fn render(state: &ListSelectionState) -> Buffer {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| draw(frame, frame.area(), state, test_context()))
        .unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn tabs_search_and_items_share_the_same_state_column() {
    let state = state();
    let buffer = render(&state);

    assert_eq!(buffer[(2, 1)].bg, test_context().highlight());
    assert_eq!(buffer[(2, 2)].symbol(), "┌");
    assert_eq!(buffer[(0, 5)].symbol(), "❯");
    assert_eq!(buffer[(2, 5)].symbol(), "s");
}

#[test]
fn focus_marker_moves_from_items_through_search_to_tabs() {
    let mut state = state();

    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    let search = render(&state);
    assert_eq!(search[(0, 3)].symbol(), "❯");
    assert_eq!(search[(0, 5)].symbol(), " ");

    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    let tabs = render(&state);
    assert_eq!(tabs[(0, 1)].symbol(), "❯");
    assert_eq!(tabs[(0, 3)].symbol(), " ");
}
