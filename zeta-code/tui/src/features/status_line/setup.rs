use std::collections::BTreeMap;

use crate::components::pane::PaneViewModel;
use crate::components::selection::SelectionActivationMode;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;

use super::StatusLineEdit;
use super::StatusLineItem;
use super::StatusLineSettings;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StatusLineSelectionAction {
    SetEnabled(StatusLineEdit),
}

pub(crate) struct StatusLineSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, StatusLineSelectionAction>,
}

pub(super) fn selection_view(
    settings: StatusLineSettings,
    revision: u64,
) -> StatusLineSelectionView {
    let mut actions = BTreeMap::new();
    let items = StatusLineItem::ALL
        .into_iter()
        .map(|item| {
            let enabled = settings.enabled(item);
            let id = SelectionItemId::new(item.id());
            actions.insert(
                id.clone(),
                StatusLineSelectionAction::SetEnabled(StatusLineEdit {
                    expected_revision: revision,
                    item,
                    enabled: !enabled,
                }),
            );
            SelectionItem::new(item.label()).with_id(id).with_columns(
                item.label(),
                item.description(),
                enabled.to_string(),
            )
        })
        .collect();
    let model =
        SelectionViewModel::new("Status line", vec![SelectionTab::new("Status line", items)])
            .without_tab_bar()
            .with_activation_mode(SelectionActivationMode::EnterOrSpace);
    StatusLineSelectionView {
        model: PaneViewModel::new(model, "Enter/Space toggle  ·  ↑/↓ select  ·  Esc back"),
        actions,
    }
}

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
