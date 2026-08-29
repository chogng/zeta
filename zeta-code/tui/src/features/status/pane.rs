use crate::components::pane::PaneViewModel;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;

pub(crate) struct StatusViewData<'a> {
    pub(crate) model: &'a str,
    pub(crate) full_context_window: Option<u64>,
    pub(crate) available_context_window: Option<u64>,
    pub(crate) remaining_context_window: RemainingContextWindow,
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

pub(crate) fn status_view(data: StatusViewData<'_>) -> PaneViewModel<SelectionViewModel> {
    PaneViewModel::new(
        SelectionViewModel::new(
            "Status",
            vec![SelectionTab::new(
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
                    detail("Session ID", data.session_id),
                    detail("Thread ID", data.thread_id),
                    detail("Thread version", data.thread_sequence.to_string()),
                ],
            )],
        )
        .without_tab_bar()
        .without_selection(),
        "Esc back",
    )
}

fn detail(label: &str, value: impl Into<String>) -> SelectionItem {
    SelectionItem::detail(label, value)
}

fn format_optional_tokens(tokens: Option<u64>) -> String {
    tokens.map_or_else(|| "unknown".into(), format_tokens)
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
#[path = "view_tests.rs"]
mod tests;
