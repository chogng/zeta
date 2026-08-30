use super::QuickViewState;
use crate::components::detail_list::DetailList;
use crate::components::detail_list::DetailListRow;
use crate::components::pane::PaneSpec;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn detail_scroll_is_bounded_and_supports_first_and_last_shortcuts() {
    let detail = DetailList::new(
        "Output",
        vec![DetailListRow::new("stdout", "one\ntwo\nthree")],
    );
    let mut quick_view = QuickViewState::new(PaneSpec::new(detail, "Esc close"));

    quick_view.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL));
    assert_eq!(quick_view.scroll, quick_view.max_scroll());
    quick_view.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    assert_eq!(quick_view.scroll, quick_view.max_scroll());
    quick_view.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL));
    assert_eq!(quick_view.scroll, 0);
}
