use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionActivationMode;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use std::collections::BTreeMap;
use zeta_protocol::Session;
use zeta_protocol::SessionStatus;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SessionSelectionAction {
    Resume { session_id: String },
}

pub(crate) struct SessionSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, SessionSelectionAction>,
}

pub(crate) fn session_selection_view(
    sessions: &[Session],
    active_session_id: &str,
) -> SessionSelectionView {
    let mut actions = BTreeMap::new();
    let mut selected = 0;
    let all = sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            if session.session_id.as_str() == active_session_id {
                selected = index;
            }
            session_item(index, session, active_session_id, &mut actions)
        })
        .collect::<Vec<_>>();
    let active = filtered_items(&all, sessions, SessionStatus::Active);
    let completed = filtered_items(&all, sessions, SessionStatus::Completed);
    let archived = filtered_items(&all, sessions, SessionStatus::Archived);
    let active_count = active.len();
    let completed_count = completed.len();
    let archived_count = archived.len();

    SessionSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                "Resume session",
                vec![
                    SelectionTab::new(format!("All ({})", all.len()), all),
                    SelectionTab::new(format!("Active ({active_count})"), active),
                    SelectionTab::new(format!("Completed ({completed_count})"), completed),
                    SelectionTab::new(format!("Archived ({archived_count})"), archived),
                ],
            )
            .with_activation_mode(SelectionActivationMode::Enter)
            .with_initial_selected(selected)
            .with_search(SearchBoxModel::new("Search saved sessions"))
            .with_empty_message("No matching sessions"),
            "Space search  ·  ←/→ tabs  ·  ↑/↓ select  ·  Enter resume  ·  Esc back",
        ),
        actions,
    }
}

fn session_item(
    index: usize,
    session: &Session,
    active_session_id: &str,
    actions: &mut BTreeMap<SelectionItemId, SessionSelectionAction>,
) -> SelectionItem {
    let item_id = SelectionItemId::new(format!("session-{index}"));
    actions.insert(
        item_id.clone(),
        SessionSelectionAction::Resume {
            session_id: session.session_id.to_string(),
        },
    );
    SelectionItem::new(format!(
        "{}{}",
        session.title,
        if session.session_id.as_str() == active_session_id {
            " ✓"
        } else {
            ""
        }
    ))
    .with_id(item_id)
    .with_description(format!(
        "{}  ·  {}  ·  {} threads",
        session.session_id,
        status_label(session.status),
        session.threads.len()
    ))
}

fn filtered_items(
    items: &[SelectionItem],
    sessions: &[Session],
    status: SessionStatus,
) -> Vec<SelectionItem> {
    items
        .iter()
        .zip(sessions)
        .filter(|(_, session)| session.status == status)
        .map(|(item, _)| item.clone())
        .collect()
}

fn status_label(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Active => "active",
        SessionStatus::Completed => "completed",
        SessionStatus::Archived => "archived",
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
