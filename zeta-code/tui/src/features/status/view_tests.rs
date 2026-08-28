use super::RemainingContextWindow;
use super::StatusViewData;
use super::status_view;
use crate::components::selection;
use crate::components::selection::SelectionViewState;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;

#[test]
fn status_pane_exposes_model_context_and_conversation_identity_without_search() {
    let state = SelectionViewState::new(
        status_view(StatusViewData {
            model: "openai/gpt",
            full_context_window: Some(1_000_000),
            available_context_window: Some(894_880),
            remaining_context_window: RemainingContextWindow::Estimated {
                remaining_tokens: 771_424,
                available_tokens: 894_880,
            },
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
            ("Remaining context window", "~771,424 tokens (86.2%)"),
            ("Session ID", "session-1"),
            ("Thread ID", "thread-2"),
            ("Thread version", "3"),
        ]
    );
    assert_eq!(state.selected_visible_index(), None);
}

#[test]
fn status_pane_renders_bold_labels_with_colons_and_plain_values() {
    let state = SelectionViewState::new(
        status_view(StatusViewData {
            model: "openai/gpt",
            full_context_window: Some(1_000_000),
            available_context_window: Some(894_880),
            remaining_context_window: RemainingContextWindow::Exact {
                remaining_tokens: 771_424,
                available_tokens: 894_880,
            },
            session_id: "session-1",
            thread_id: "thread-2",
            thread_sequence: 3,
        })
        .into_body(),
    );
    let backend = TestBackend::new(100, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| selection::draw(frame, frame.area(), &state))
        .unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(4, 2)].symbol(), "M");
    assert!(buffer[(4, 2)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(9, 2)].symbol(), ":");
    assert!(buffer[(9, 2)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(11, 2)].symbol(), "o");
    assert!(!buffer[(11, 2)].modifier.contains(Modifier::BOLD));
}
