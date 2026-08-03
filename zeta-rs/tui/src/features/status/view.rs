use crate::components::pane::PaneViewModel;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;

pub(crate) fn status_view(
    session_id: &str,
    thread_id: &str,
    thread_sequence: u64,
    model: &str,
) -> PaneViewModel<SelectionViewModel> {
    PaneViewModel::new(
        SelectionViewModel::new(
            "Status",
            vec![SelectionTab::new(
                "Status",
                vec![
                    detail("Session", session_id),
                    detail("Thread", thread_id),
                    detail("Thread sequence", thread_sequence.to_string()),
                    detail("Model", model),
                ],
            )],
        )
        .without_tab_bar()
        .without_selection(),
        "Esc back",
    )
}

fn detail(label: &str, value: impl Into<String>) -> SelectionItem {
    SelectionItem::new(label).with_description(value)
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
