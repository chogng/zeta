use super::RemainingContextWindow;
use super::StatusPanelOutcome;
use super::StatusViewData;
use super::status_panel;
use crate::render::test_context;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;
use zeta_protocol::ModelMoneyAmount;
use zeta_protocol::ModelReferenceCostSummary;
use zeta_protocol::ModelUsageSummary;
use zeta_protocol::ModelUsageTotal;

#[test]
fn status_panel_exposes_model_accounting_context_and_conversation_identity() {
    let usage = usage();
    let reference_cost = reference_cost();
    let panel = panel(&usage, &reference_cost);

    assert_eq!(panel.detail.title(), "Status");
    assert_eq!(
        panel
            .detail
            .rows()
            .iter()
            .map(|row| (row.label(), row.value()))
            .collect::<Vec<_>>(),
        vec![
            ("Model", "openai/gpt"),
            ("Full context window", "1,000,000 tokens"),
            ("Available context window", "894,880 tokens"),
            ("Remaining context window", "~771,424 tokens (86.2%)"),
            ("Model calls", "4"),
            ("Input tokens", "10,000 tokens"),
            ("Cached input", "7,500 tokens"),
            ("Cached input share", "75.0%"),
            ("Cache writes", "500 tokens"),
            ("Output tokens", ">=1,200 tokens"),
            ("Reasoning output", "unknown"),
            ("Reference cost", "$0.01008"),
            ("Session ID", "session-1"),
            ("Thread ID", "thread-2"),
            ("Thread version", "3"),
        ]
    );
}

#[test]
fn status_panel_requests_full_content_height_and_renders_bold_labels() {
    let usage = usage();
    let reference_cost = reference_cost();
    let panel = panel(&usage, &reference_cost);
    assert_eq!(panel.desired_height(100), 18);
    assert!(panel.desired_height(24) > panel.desired_height(100));
    let backend = TestBackend::new(100, panel.desired_height(100));
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| panel.draw(frame, frame.area(), test_context()))
        .unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(2, 1)].symbol(), "M");
    assert!(buffer[(2, 1)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(7, 1)].symbol(), ":");
    assert!(buffer[(7, 1)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(9, 1)].symbol(), "o");
    assert!(!buffer[(9, 1)].modifier.contains(Modifier::BOLD));
}

#[test]
fn status_panel_scrolls_when_allocated_height_is_shorter_than_content() {
    let usage = usage();
    let reference_cost = reference_cost();
    let mut panel = panel(&usage, &reference_cost);
    let backend = TestBackend::new(80, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    assert_eq!(
        panel.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
        StatusPanelOutcome::Consumed
    );
    terminal
        .draw(|frame| panel.draw(frame, frame.area(), test_context()))
        .unwrap();

    let rendered = terminal.backend().to_string();
    assert!(rendered.contains("Thread version: 3"));
    assert!(!rendered.contains("Full context window"));
    assert_eq!(
        panel.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        StatusPanelOutcome::Consumed
    );
    assert_eq!(
        panel.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        StatusPanelOutcome::Dismiss
    );
}

#[test]
fn status_panel_does_not_invent_a_cache_share_or_exact_cost() {
    let mut usage = usage();
    usage.cached_input_tokens.complete = false;
    let reference_cost = ModelReferenceCostSummary {
        known_amounts: vec![ModelMoneyAmount {
            currency: "USD".into(),
            pico_units: "1000000000".into(),
        }],
        complete: false,
    };
    let panel = panel(&usage, &reference_cost);
    let rows = panel.detail.rows();

    assert_eq!(row_value(rows, "Cached input"), ">=7,500 tokens");
    assert_eq!(row_value(rows, "Cached input share"), "unknown");
    assert_eq!(row_value(rows, "Reference cost"), "≥$0.001");
}

fn panel<'a>(
    usage: &'a ModelUsageSummary,
    reference_cost: &'a ModelReferenceCostSummary,
) -> super::StatusPanel {
    status_panel(StatusViewData {
        model: "openai/gpt",
        full_context_window: Some(1_000_000),
        available_context_window: Some(894_880),
        remaining_context_window: RemainingContextWindow::Estimated {
            remaining_tokens: 771_424,
            available_tokens: 894_880,
        },
        usage,
        reference_cost,
        session_id: "session-1",
        thread_id: "thread-2",
        thread_sequence: 3,
    })
}

fn row_value<'a>(rows: &'a [crate::widgets::detail_list::DetailListRow], label: &str) -> &'a str {
    rows.iter()
        .find(|row| row.label() == label)
        .unwrap()
        .value()
}

fn reference_cost() -> ModelReferenceCostSummary {
    ModelReferenceCostSummary {
        known_amounts: vec![ModelMoneyAmount {
            currency: "USD".into(),
            pico_units: "10080000000".into(),
        }],
        complete: true,
    }
}

fn usage() -> ModelUsageSummary {
    ModelUsageSummary {
        model_invocations: 4,
        input_tokens: ModelUsageTotal {
            reported: 10_000,
            complete: true,
        },
        output_tokens: ModelUsageTotal {
            reported: 1_200,
            complete: false,
        },
        cached_input_tokens: ModelUsageTotal {
            reported: 7_500,
            complete: true,
        },
        cache_write_input_tokens: ModelUsageTotal {
            reported: 500,
            complete: true,
        },
        reasoning_tokens: ModelUsageTotal {
            reported: 0,
            complete: false,
        },
    }
}
