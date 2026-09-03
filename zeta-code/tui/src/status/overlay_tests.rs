use super::RemainingContextWindow;
use super::StatusViewData;
use super::status_overlay;
use crate::render::test_context;
use crate::widgets::detail_list;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;

#[test]
fn status_overlay_exposes_model_context_and_conversation_identity_without_search() {
    let state = status_overlay(StatusViewData {
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
    });

    assert_eq!(state.title(), "Status");
    assert_eq!(
        state
            .rows()
            .iter()
            .map(|row| (row.label(), row.value()))
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
}

#[test]
fn status_overlay_renders_bold_labels_with_colons_and_plain_values() {
    let state = status_overlay(StatusViewData {
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
    });
    let backend = TestBackend::new(100, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| detail_list::draw_scrolled(frame, frame.area(), &state, 0, test_context()))
        .unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(2, 1)].symbol(), "M");
    assert!(buffer[(2, 1)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(7, 1)].symbol(), ":");
    assert!(buffer[(7, 1)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(9, 1)].symbol(), "o");
    assert!(!buffer[(9, 1)].modifier.contains(Modifier::BOLD));
}
