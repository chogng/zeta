use super::status_view;
use crate::components::selection::SelectionViewState;

#[test]
fn status_pane_exposes_conversation_identity_without_search() {
    let state =
        SelectionViewState::new(status_view("session-1", "thread-2", 3, "openai/gpt").into_body());

    assert_eq!(state.title(), "Status");
    assert!(state.search().is_none());
    assert_eq!(
        state
            .visible_items()
            .iter()
            .map(|item| (item.label(), item.description().unwrap()))
            .collect::<Vec<_>>(),
        vec![
            ("Session", "session-1"),
            ("Thread", "thread-2"),
            ("Thread sequence", "3"),
            ("Model", "openai/gpt"),
        ]
    );
    assert_eq!(state.selected_visible_index(), None);
}
