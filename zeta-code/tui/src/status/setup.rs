use std::collections::BTreeMap;

use crate::widgets::list_selection::ListSelectionActivationMode;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionItemId;
use crate::widgets::list_selection::ListSelectionModel;

use super::StatusLineEdit;
use super::StatusLineItem;
use super::StatusLineSettings;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StatusLineSelectionAction {
    SetEnabled(StatusLineEdit),
}

pub(crate) struct StatusLineChoices {
    pub(crate) model: ListSelectionModel,
    pub(crate) actions: BTreeMap<ListSelectionItemId, StatusLineSelectionAction>,
}

pub(crate) fn list_selection(settings: &StatusLineSettings, revision: u64) -> StatusLineChoices {
    let mut actions = BTreeMap::new();
    let items = StatusLineItem::ALL
        .into_iter()
        .map(|item| {
            let enabled = settings.enabled(item);
            let id = ListSelectionItemId::new(item.id());
            actions.insert(
                id.clone(),
                StatusLineSelectionAction::SetEnabled(StatusLineEdit {
                    expected_revision: revision,
                    item,
                    enabled: !enabled,
                }),
            );
            ListSelectionItem::new(item.label())
                .with_id(id)
                .with_columns(item.label(), item.description(), checkbox(enabled))
        })
        .collect();
    let model = ListSelectionModel::new(
        "Status line",
        vec![ListSelectionGroup::new("Status line", items)],
    )
    .without_tab_bar()
    .with_activation_mode(ListSelectionActivationMode::EnterOrSpace)
    .with_activation_label("toggle");
    StatusLineChoices { model, actions }
}

const fn checkbox(checked: bool) -> &'static str {
    if checked { "[ ✔ ]" } else { "[   ]" }
}

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
