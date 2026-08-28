use super::RemainingContextWindow;
use super::StatusViewData;
use super::status_view;
use crate::components::selection::SelectionViewState;

#[test]
fn status_pane_exposes_model_context_and_conversation_identity_without_search() {
    let state = SelectionViewState::new(
        status_view(StatusViewData {
            model: "openai/gpt",
            full_context_window: Some(1_000_000),
            available_context_window: Some(894_880),
            remaining_context_window: RemainingContextWindow::Estimated(771_424),
            session_id: "session-1",
            thread_id: "thread-2",
            thread_sequence: 3,
        })
        .into_body(),
    );

    assert_eq!(state.title(), "Status");
    assert!(state.search().is_none());
    assert_eq!(
        state
            .visible_items()
            .iter()
            .map(|item| (item.label(), item.description().unwrap()))
            .collect::<Vec<_>>(),
        vec![
            ("Model", "openai/gpt"),
            ("Full context window", "1,000,000 tokens"),
            ("Available context window", "894,880 tokens"),
            ("Remaining context window", "~771,424 tokens"),
            ("Session ID", "session-1"),
            ("Thread ID", "thread-2"),
            ("Thread sequence", "3"),
        ]
    );
    assert_eq!(state.selected_visible_index(), None);
}
