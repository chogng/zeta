use crate::components::list_selection::ListSelectionActivationMode;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
use std::collections::BTreeMap;
use zeta_protocol::Session;
use zeta_protocol::SessionStatus;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SessionSelectionAction {
    Resume { session_id: String },
}

pub(crate) struct SessionPaneSpec {
    pub(crate) model: PaneSpec<ListSelectionModel>,
    pub(crate) actions: BTreeMap<ListSelectionItemId, SessionSelectionAction>,
}

pub(crate) fn session_pane_spec(sessions: &[Session], active_session_id: &str) -> SessionPaneSpec {
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

    SessionPaneSpec {
        model: PaneSpec::new(
            ListSelectionModel::new(
                "Resume session",
                vec![
                    ListSelectionGroup::new(format!("All ({})", all.len()), all),
                    ListSelectionGroup::new(format!("Active ({active_count})"), active),
                    ListSelectionGroup::new(format!("Completed ({completed_count})"), completed),
                    ListSelectionGroup::new(format!("Archived ({archived_count})"), archived),
                ],
            )
            .with_activation_mode(ListSelectionActivationMode::Enter)
            .with_initial_selected(selected)
            .with_search(SearchBoxModel::new("Search saved sessions"))
            .with_empty_message("No matching sessions"),
            "Space search  ·  Tab/Shift-Tab tabs  ·  ↑/↓ select  ·  Enter resume  ·  Esc back",
        ),
        actions,
    }
}

fn session_item(
    index: usize,
    session: &Session,
    active_session_id: &str,
    actions: &mut BTreeMap<ListSelectionItemId, SessionSelectionAction>,
) -> ListSelectionItem {
    let item_id = ListSelectionItemId::new(format!("session-{index}"));
    actions.insert(
        item_id.clone(),
        SessionSelectionAction::Resume {
            session_id: session.session_id.to_string(),
        },
    );
    ListSelectionItem::new(format!(
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
    items: &[ListSelectionItem],
    sessions: &[Session],
    status: SessionStatus,
) -> Vec<ListSelectionItem> {
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
#[path = "pane_tests.rs"]
mod tests;
