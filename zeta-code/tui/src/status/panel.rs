use super::model::format_cache_hit_rate;
use super::model::format_reference_cost;
use crate::render::RenderContext;
use crate::widgets::detail_list;
use crate::widgets::detail_list::DetailList;
use crate::widgets::detail_list::DetailListRow;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Rect;
use zeta_protocol::ModelReferenceCostSummary;
use zeta_protocol::ModelUsageSummary;
use zeta_protocol::ModelUsageTotal;

const PAGE_ROWS: u16 = 5;

pub(crate) struct StatusViewData<'a> {
    pub(crate) model: &'a str,
    pub(crate) full_context_window: Option<u64>,
    pub(crate) available_context_window: Option<u64>,
    pub(crate) remaining_context_window: RemainingContextWindow,
    pub(crate) usage: &'a ModelUsageSummary,
    pub(crate) reference_cost: &'a ModelReferenceCostSummary,
    pub(crate) session_id: &'a str,
    pub(crate) thread_id: &'a str,
    pub(crate) thread_sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RemainingContextWindow {
    Exact {
        remaining_tokens: u64,
        available_tokens: u64,
    },
    Estimated {
        remaining_tokens: u64,
        available_tokens: u64,
    },
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatusPanelOutcome {
    Consumed,
    Dismiss,
}

#[derive(Debug)]
pub(crate) struct StatusPanel {
    detail: DetailList,
    scroll: u16,
}

impl StatusPanel {
    pub(crate) fn desired_height(&self, width: u16) -> u16 {
        self.detail
            .desired_height_for_width(width.saturating_sub(4))
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> StatusPanelOutcome {
        if key.kind != KeyEventKind::Press || key.modifiers != KeyModifiers::NONE {
            return StatusPanelOutcome::Consumed;
        }

        match key.code {
            KeyCode::Esc => return StatusPanelOutcome::Dismiss,
            KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(PAGE_ROWS),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(PAGE_ROWS),
            KeyCode::Home => self.scroll = 0,
            KeyCode::End => self.scroll = u16::MAX,
            _ => {}
        }
        StatusPanelOutcome::Consumed
    }

    pub(crate) fn draw(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
        let visible_rows = area.height.saturating_sub(1);
        let content_height =
            u16::try_from(self.detail.content_height(area.width.saturating_sub(4)))
                .unwrap_or(u16::MAX);
        let allocated_max_scroll = content_height.saturating_sub(visible_rows);
        detail_list::draw_scrolled(
            frame,
            area,
            &self.detail,
            self.scroll.min(allocated_max_scroll),
            context,
        );
    }

    pub(crate) const fn key_hints(&self) -> &'static str {
        "↑/↓ scroll · PgUp/PgDn page · Home/End jump · Esc close"
    }
}

pub(crate) fn status_panel(data: StatusViewData<'_>) -> StatusPanel {
    StatusPanel {
        detail: DetailList::new(
            "Status",
            vec![
                detail("Model", data.model),
                detail(
                    "Full context window",
                    format_optional_tokens(data.full_context_window),
                ),
                detail(
                    "Available context window",
                    format_optional_tokens(data.available_context_window),
                ),
                detail(
                    "Remaining context window",
                    format_remaining_context(data.remaining_context_window),
                ),
                detail("Model calls", data.usage.model_invocations.to_string()),
                detail("Input tokens", format_usage_total(&data.usage.input_tokens)),
                detail(
                    "Cached input",
                    format_usage_total(&data.usage.cached_input_tokens),
                ),
                detail(
                    "Cached input share",
                    format_cache_hit_rate(data.usage).unwrap_or_else(|| "unknown".into()),
                ),
                detail(
                    "Cache writes",
                    format_usage_total(&data.usage.cache_write_input_tokens),
                ),
                detail(
                    "Output tokens",
                    format_usage_total(&data.usage.output_tokens),
                ),
                detail(
                    "Reasoning output",
                    format_usage_total(&data.usage.reasoning_tokens),
                ),
                detail(
                    "Reference cost",
                    format_reference_cost(data.usage.model_invocations, data.reference_cost)
                        .unwrap_or_else(|| "unknown".into()),
                ),
                detail("Session ID", data.session_id),
                detail("Thread ID", data.thread_id),
                detail("Thread version", data.thread_sequence.to_string()),
            ],
        ),
        scroll: 0,
    }
}

fn detail(label: &str, value: impl Into<String>) -> DetailListRow {
    DetailListRow::new(label, value)
}

fn format_optional_tokens(tokens: Option<u64>) -> String {
    tokens.map_or_else(|| "unknown".into(), format_tokens)
}

fn format_usage_total(total: &ModelUsageTotal) -> String {
    if total.complete {
        format_tokens(total.reported)
    } else if total.reported > 0 {
        format!(">={}", format_tokens(total.reported))
    } else {
        "unknown".into()
    }
}

fn format_remaining_context(remaining: RemainingContextWindow) -> String {
    match remaining {
        RemainingContextWindow::Exact {
            remaining_tokens,
            available_tokens,
        } => format_remaining_tokens(remaining_tokens, available_tokens),
        RemainingContextWindow::Estimated {
            remaining_tokens,
            available_tokens,
        } => format!(
            "~{}",
            format_remaining_tokens(remaining_tokens, available_tokens)
        ),
        RemainingContextWindow::Unknown => "unknown".into(),
    }
}

fn format_remaining_tokens(remaining_tokens: u64, available_tokens: u64) -> String {
    let percentage_tenths = if available_tokens == 0 {
        0
    } else {
        (u128::from(remaining_tokens) * 1_000 / u128::from(available_tokens)).min(1_000) as u64
    };
    format!(
        "{} ({}.{:01}%)",
        format_tokens(remaining_tokens),
        percentage_tenths / 10,
        percentage_tenths % 10
    )
}

fn format_tokens(tokens: u64) -> String {
    let digits = tokens.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3 + 7);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(character);
    }
    formatted.push_str(" tokens");
    formatted
}

#[cfg(test)]
#[path = "panel_tests.rs"]
mod tests;
