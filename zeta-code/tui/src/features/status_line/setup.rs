use std::collections::BTreeMap;

use crate::components::list_selection::ListSelectionActivationMode;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;

use super::StatusLineEdit;
use super::StatusLineItem;
use super::StatusLineSettings;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StatusLineSelectionAction {
    SetEnabled(StatusLineEdit),
}

pub(crate) struct StatusLinePaneSpec {
    pub(crate) model: PaneSpec<ListSelectionModel>,
    pub(crate) actions: BTreeMap<ListSelectionItemId, StatusLineSelectionAction>,
}

pub(crate) fn list_selection(settings: &StatusLineSettings, revision: u64) -> StatusLinePaneSpec {
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
    StatusLinePaneSpec {
        model: PaneSpec::new(model),
        actions,
    }
}

const fn checkbox(checked: bool) -> &'static str {
    if checked { "[ ✔ ]" } else { "[   ]" }
}

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
