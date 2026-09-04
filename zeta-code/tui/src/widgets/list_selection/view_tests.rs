use super::draw;
use super::draw_with_pointer;
use crate::render::test_context;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionModel;
use crate::widgets::list_selection::ListSelectionState;
use crate::widgets::search_box::SearchBoxModel;
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

fn render_with_item_hover(state: &ListSelectionState, hovered_item: usize) -> Buffer {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            draw_with_pointer(
                frame,
                frame.area(),
                state,
                None,
                None,
                false,
                false,
                Some(hovered_item),
                None,
                test_context(),
            )
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn tabs_search_and_items_share_the_same_state_column() {
    let state = state();
    let buffer = render(&state);

    assert_eq!(
        buffer[(2, 0)].bg,
        test_context().accent_surface_background()
    );
    assert_eq!(buffer[(2, 1)].symbol(), "┌");
    assert_eq!(buffer[(0, 4)].symbol(), ">");
    assert_eq!(buffer[(2, 4)].symbol(), "s");
}

#[test]
fn keyboard_focus_does_not_add_markers_to_search_or_tabs() {
    let mut state = state();

    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    let search = render(&state);
    assert_eq!(search[(0, 2)].symbol(), " ");
    assert_eq!(search[(0, 4)].symbol(), ">");

    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    let tabs = render(&state);
    assert_eq!(tabs[(0, 0)].symbol(), " ");
    assert_eq!(tabs[(0, 4)].symbol(), ">");
}

#[test]
fn keyboard_selection_and_a_different_hovered_row_remain_visible_together() {
    let state = ListSelectionState::new(
        ListSelectionModel::new(
            "Items",
            vec![ListSelectionGroup::new(
                "All",
                vec![
                    ListSelectionItem::new("First"),
                    ListSelectionItem::new("Second"),
                ],
            )],
        )
        .without_tab_bar(),
    );

    let buffer = render_with_item_hover(&state, 1);

    assert_eq!(buffer[(0, 0)].symbol(), ">");
    assert_eq!(buffer[(2, 0)].fg, test_context().foreground());
    assert_eq!(buffer[(2, 0)].bg, ratatui::style::Color::Reset);
    assert_eq!(buffer[(0, 1)].symbol(), " ");
    assert_eq!(buffer[(2, 1)].fg, test_context().foreground());
    assert_eq!(buffer[(2, 1)].bg, ratatui::style::Color::Reset);
}
