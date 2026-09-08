use super::draw_body_with_pointer;
use super::draw_tabs;
use crate::render::horizontal_margin;
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

#[test]
fn mouse_scroll_clamps_and_keyboard_navigation_reveals_the_current_item() {
    use crate::widgets::navigation::Navigation;
    use ratatui::layout::Position;
    use ratatui::layout::Rect;
    let mut view = ListSelectionState::new(
        ListSelectionModel::new(
            "Items",
            vec![ListSelectionGroup::new(
                "All",
                (0..30)
                    .map(|index| ListSelectionItem::new(format!("Item {index}")))
                    .collect(),
            )],
        )
        .without_tab_bar(),
    );
    let area = Rect::new(2, 1, 38, 8);
    for _ in 0..40 {
        view.scroll(area, Navigation::Next, Position::new(2, 2));
    }
    assert_eq!(view.selected_visible_index(), Some(0));
    assert_eq!(view.item_index_in(area, 2, area.bottom() - 1), Some(29));
    assert_eq!(view.item_index_in(Rect::new(2, 1, 38, 30), 2, 1), Some(0));
    view.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(view.selected_visible_index(), Some(1));
    assert_eq!(view.item_index_in(area, 2, 1), Some(0));
    for _ in 0..40 {
        view.scroll(area, Navigation::Previous, Position::new(2, 2));
    }
    assert_eq!(view.item_index_in(area, 2, 1), Some(0));
}

#[test]
fn overflowing_lists_keep_selection_visible_and_notices_out_of_hit_testing() {
    use ratatui::layout::Rect;
    let mut view = ListSelectionState::new(
        ListSelectionModel::new(
            "Items",
            vec![ListSelectionGroup::new(
                "All",
                (0..30)
                    .map(|index| {
                        ListSelectionItem::new(format!("Item {index}")).with_id(
                            crate::widgets::list_selection::ListSelectionItemId::new(
                                index.to_string(),
                            ),
                        )
                    })
                    .collect(),
            )],
        )
        .without_tab_bar(),
    );
    assert_eq!(view.body_rows(), 30);
    for height in 1..35 {
        for selected in 0..30 {
            assert!(view.select_visible_item(selected));
            let area = Rect::new(2, 0, 38, height);
            let viewport = super::ListViewport::new(area, 30, Some(selected));
            assert!(viewport.start <= selected && selected < viewport.end);
            assert!(viewport.below.bottom() <= area.bottom());
            let row = viewport.items.y + (selected - viewport.start) as u16;
            assert_eq!(view.item_index_in(area, 2, row), Some(selected));
            for notice in [viewport.above, viewport.below] {
                if notice.height > 0 {
                    assert_eq!(view.item_index_in(area, 2, notice.y), None);
                }
            }
        }
    }
    view.select_visible_item(14);
    let mut terminal = Terminal::new(TestBackend::new(40, 8)).unwrap();
    terminal
        .draw(|frame| {
            draw_body_with_pointer(
                frame,
                frame.area(),
                &view,
                false,
                false,
                None,
                None,
                crate::render::test_context(),
            )
        })
        .unwrap();
    let rendered = terminal.backend().to_string();
    assert!(rendered.contains("9 more above"));
    assert!(rendered.contains("15 more below"));
    let mut terminal = Terminal::new(TestBackend::new(40, 30)).unwrap();
    terminal
        .draw(|frame| {
            draw_body_with_pointer(
                frame,
                frame.area(),
                &view,
                false,
                false,
                None,
                None,
                crate::render::test_context(),
            )
        })
        .unwrap();
    assert!(!terminal.backend().to_string().contains("more"));
}

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
    render_with_pointer(state, None)
}

fn render_with_item_hover(state: &ListSelectionState, hovered_item: usize) -> Buffer {
    render_with_pointer(state, Some(hovered_item))
}

fn render_with_pointer(state: &ListSelectionState, hovered_item: Option<usize>) -> Buffer {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let content = horizontal_margin(frame.area(), 2);
            let tab_rows = state.tab_rows(content.width);
            let tabs = ratatui::layout::Rect::new(content.x, content.y, content.width, tab_rows);
            let body = ratatui::layout::Rect::new(
                content.x,
                content.y.saturating_add(tab_rows),
                content.width,
                content.height.saturating_sub(tab_rows),
            );
            draw_tabs(frame, tabs, state, None, None, test_context());
            draw_body_with_pointer(
                frame,
                body,
                state,
                false,
                false,
                hovered_item,
                None,
                test_context(),
            );
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
    assert_eq!(buffer[(2, 1)].symbol(), "╭");
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
