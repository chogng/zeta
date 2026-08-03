use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionActivationMode;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use std::collections::BTreeMap;
use zeta_protocol::Thread;
use zeta_protocol::ThreadItem;
use zeta_protocol::TurnId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RewindSelectionAction {
    Rewind {
        before_turn_id: TurnId,
        checkpoint_label: String,
    },
}

pub(crate) struct RewindSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, RewindSelectionAction>,
}

pub(crate) fn rewind_selection_view(thread: &Thread) -> RewindSelectionView {
    let checkpoints = thread
        .turns
        .iter()
        .filter_map(|turn| checkpoint_text(&turn.items).map(|text| (turn, text)))
        .collect::<Vec<_>>();
    let total = checkpoints.len();
    let mut actions = BTreeMap::new();
    let items = checkpoints
        .iter()
        .enumerate()
        .map(|(index, (turn, text))| {
            let item_id = SelectionItemId::new(format!("rewind-{index}"));
            let checkpoint_label = compact_label(text);
            actions.insert(
                item_id.clone(),
                RewindSelectionAction::Rewind {
                    before_turn_id: turn.turn_id.clone(),
                    checkpoint_label: checkpoint_label.clone(),
                },
            );
            let removed = total.saturating_sub(index);
            SelectionItem::new(format!("{}. {checkpoint_label}", index + 1))
                .with_id(item_id)
                .with_description(format!(
                    "remove this checkpoint and {remaining} later turn{suffix}",
                    remaining = removed.saturating_sub(1),
                    suffix = if removed == 2 { "" } else { "s" }
                ))
        })
        .collect::<Vec<_>>();
    let selected = items.len().saturating_sub(1);

    RewindSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new("Rewind", vec![SelectionTab::new("Checkpoints", items)])
                .with_activation_mode(SelectionActivationMode::Enter)
                .without_tab_bar()
                .with_initial_selected(selected)
                .with_search(SearchBoxModel::new("Search message checkpoints"))
                .with_empty_message("No message checkpoints available"),
            "Space search  ·  ↑/↓ select  ·  Enter rewind  ·  Esc back",
        ),
        actions,
    }
}

fn checkpoint_text(items: &[ThreadItem]) -> Option<String> {
    let parts = items
        .iter()
        .filter_map(|item| match item {
            ThreadItem::UserMessage { text, .. } => Some(text.as_str()),
            ThreadItem::UserImage { .. } => Some("[Image]"),
            ThreadItem::AgentMessage { .. }
            | ThreadItem::Reasoning { .. }
            | ThreadItem::Plan { .. }
            | ThreadItem::ToolCall { .. }
            | ThreadItem::ToolResult { .. } => None,
        })
        .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join(" "))
}

fn compact_label(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut characters = normalized.chars();
    let label = characters.by_ref().take(96).collect::<String>();
    if characters.next().is_some() {
        format!("{label}…")
    } else {
        label
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
