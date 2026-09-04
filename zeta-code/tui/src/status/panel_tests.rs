use super::AppServerResourcesView;
use super::ProcessMemoryCurrent;
use super::ProcessResourcesView;
use super::RemainingContextWindow;
use super::StatusPanelOutcome;
use super::StatusViewData;
use super::status_panel;
use crate::render::test_context;
use crate::status::ProcessCpuCurrent;
use crate::status::ProcessUsageView;
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

    assert_eq!(panel.title(), "Status");
    assert_eq!(
        panel
            .session
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
    assert_eq!(panel.desired_height(100), 19);
    assert!(panel.desired_height(24) > panel.desired_height(100));
    let backend = TestBackend::new(100, panel.desired_height(100));
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| panel.draw(frame, frame.area(), None, None, test_context()))
        .unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(3, 0)].symbol(), "S");
    assert_eq!(buffer[(2, 0)].symbol(), " ");
    assert_eq!(buffer[(10, 0)].symbol(), " ");
    assert_eq!(
        buffer[(3, 0)].fg,
        test_context().accent_surface_foreground()
    );
    assert_eq!(
        buffer[(3, 0)].bg,
        test_context().accent_surface_background()
    );
    assert_eq!(buffer[(2, 1)].symbol(), "M");
    assert!(buffer[(2, 1)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(7, 1)].symbol(), ":");
    assert!(buffer[(7, 1)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(28, 1)].symbol(), "o");
    assert!(!buffer[(28, 1)].modifier.contains(Modifier::BOLD));
}

#[test]
fn status_panel_updates_process_rows_without_resetting_each_tab_scroll() {
    let usage = usage();
    let reference_cost = reference_cost();
    let mut panel = panel(&usage, &reference_cost);
    panel.scroll = [7, 3];

    panel.apply_process_resources(ProcessResourcesView {
        local: ProcessUsageView {
            memory: ProcessMemoryCurrent::Available(240 * 1024 * 1024),
            cpu: ProcessCpuCurrent::Available(124),
        },
        tui: ProcessUsageView {
            memory: ProcessMemoryCurrent::Available(140 * 1024 * 1024),
            cpu: ProcessCpuCurrent::Available(84),
        },
        app_server: AppServerResourcesView::Local(ProcessUsageView {
            memory: ProcessMemoryCurrent::Available(100 * 1024 * 1024),
            cpu: ProcessCpuCurrent::Available(40),
        }),
        observed_peak_bytes: Some(180 * 1024 * 1024),
        one_minute_change_bytes: Some(3 * 1024 * 1024),
        five_minute_change_bytes: None,
    });

    assert_eq!(panel.scroll, [7, 3]);
    assert!(panel.select_tab(1));
    assert_eq!(
        row_value(panel.processes.rows(), "TUI resident memory"),
        "140.0 MiB"
    );
    assert_eq!(row_value(panel.processes.rows(), "Local CPU"), "12.4%");
    assert_eq!(
        row_value(panel.processes.rows(), "1 minute memory change"),
        "+3.0 MiB"
    );
    assert_eq!(
        row_value(panel.processes.rows(), "App Server resident memory"),
        "100.0 MiB"
    );
    assert_eq!(
        row_value(panel.processes.rows(), "5 minute memory change"),
        "collecting"
    );
}

#[test]
fn process_tab_renders_local_total_and_owned_process_details() {
    let usage = usage();
    let reference_cost = reference_cost();
    let mut panel = panel(&usage, &reference_cost);
    panel.apply_process_resources(ProcessResourcesView {
        local: ProcessUsageView {
            memory: ProcessMemoryCurrent::Available(240 * 1024 * 1024),
            cpu: ProcessCpuCurrent::Available(124),
        },
        tui: ProcessUsageView {
            memory: ProcessMemoryCurrent::Available(140 * 1024 * 1024),
            cpu: ProcessCpuCurrent::Available(84),
        },
        app_server: AppServerResourcesView::Local(ProcessUsageView {
            memory: ProcessMemoryCurrent::Available(100 * 1024 * 1024),
            cpu: ProcessCpuCurrent::Available(40),
        }),
        observed_peak_bytes: Some(260 * 1024 * 1024),
        one_minute_change_bytes: Some(3 * 1024 * 1024),
        five_minute_change_bytes: Some(-8 * i128::from(1024 * 1024)),
    });
    panel.select_tab(1);
    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| panel.draw(frame, frame.area(), None, None, test_context()))
        .unwrap();

    insta::assert_snapshot!("status_processes_tab", terminal.backend().to_string());
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
        .draw(|frame| panel.draw(frame, frame.area(), None, None, test_context()))
        .unwrap();

    let rendered = terminal.backend().to_string();
    assert!(rendered.contains("Thread version:"));
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
fn status_panel_switches_tabs_with_keyboard_and_exposes_mouse_targets() {
    let usage = usage();
    let reference_cost = reference_cost();
    let mut panel = panel(&usage, &reference_cost);
    let desired_height = panel.desired_height(80);

    assert_eq!(panel.tabs.active_index(), 0);
    assert_eq!(
        panel.tab_index_at(ratatui::layout::Rect::new(0, 0, 80, 20), 16, 0),
        Some(1)
    );
    assert_eq!(
        panel.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        StatusPanelOutcome::Consumed
    );
    assert_eq!(panel.tabs.active_index(), 1);
    assert_eq!(
        panel.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        StatusPanelOutcome::Consumed
    );
    assert_eq!(panel.tabs.active_index(), 0);
    assert_eq!(
        panel.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        StatusPanelOutcome::Consumed
    );
    assert_eq!(panel.tabs.active_index(), 1);
    assert_eq!(
        panel.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)),
        StatusPanelOutcome::Consumed
    );
    assert_eq!(panel.tabs.active_index(), 0);
    assert_eq!(panel.desired_height(80), desired_height);
    assert_eq!(panel.key_hints(), "Tab to switch · Esc to close");
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
    let rows = panel.session.rows();

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
