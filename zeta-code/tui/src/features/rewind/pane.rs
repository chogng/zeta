use crate::components::list_selection::ListSelectionActivationMode;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
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

pub(crate) struct RewindPaneSpec {
    pub(crate) model: PaneSpec<ListSelectionModel>,
    pub(crate) actions: BTreeMap<ListSelectionItemId, RewindSelectionAction>,
}

pub(crate) fn rewind_pane_spec(thread: &Thread) -> RewindPaneSpec {
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
            let item_id = ListSelectionItemId::new(format!("rewind-{index}"));
            let checkpoint_label = compact_label(text);
            actions.insert(
                item_id.clone(),
                RewindSelectionAction::Rewind {
                    before_turn_id: turn.turn_id.clone(),
                    checkpoint_label: checkpoint_label.clone(),
                },
            );
            let removed = total.saturating_sub(index);
            ListSelectionItem::new(format!("{}. {checkpoint_label}", index + 1))
                .with_id(item_id)
                .with_description(format!(
                    "remove this checkpoint and {remaining} later turn{suffix}",
                    remaining = removed.saturating_sub(1),
                    suffix = if removed == 2 { "" } else { "s" }
                ))
        })
        .collect::<Vec<_>>();
    let selected = items.len().saturating_sub(1);

    RewindPaneSpec {
        model: PaneSpec::new(
            ListSelectionModel::new(
                "Rewind",
                vec![ListSelectionGroup::new("Checkpoints", items)],
            )
            .with_activation_mode(ListSelectionActivationMode::Enter)
            .with_activation_label("rewind")
            .without_tab_bar()
            .with_initial_selected(selected)
            .with_search(SearchBoxModel::new("Search message checkpoints"))
            .with_empty_message("No message checkpoints available"),
        ),
        actions,
    }
}

fn checkpoint_text(items: &[ThreadItem]) -> Option<String> {
    let parts = items
        .iter()
        .filter_map(|item| match item {
            ThreadItem::UserMessage { text, .. } => Some(text.as_str()),
            ThreadItem::UserContext { content, .. } => Some(content.as_str()),
            ThreadItem::UserImage { .. } | ThreadItem::UserImageAttachment { .. } => {
                Some("[Image]")
            }
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
#[path = "pane_tests.rs"]
mod tests;
