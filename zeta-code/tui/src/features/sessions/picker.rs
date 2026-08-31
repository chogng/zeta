use crate::components::list_selection::ListSelectionActivationMode;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::search_box::SearchBoxModel;
use crate::features::sessions::branch_count_label;
use crate::features::sessions::session_size_label;
use std::collections::BTreeMap;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_protocol::Session;
use zeta_protocol::SessionStatus;
use zeta_utils_elapsed::format_compact_duration;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SessionSelectionAction {
    Resume { session_id: String },
}

pub(crate) struct SessionChoices {
    pub(crate) model: ListSelectionModel,
    pub(crate) actions: BTreeMap<ListSelectionItemId, SessionSelectionAction>,
}

pub(crate) fn session_choices(
    sessions: &[Session],
    active_session_id: &str,
) -> SessionChoices {
    session_choices_at(sessions, active_session_id, current_unix_millis())
}

fn session_choices_at(
    sessions: &[Session],
    active_session_id: &str,
    now_unix_ms: u64,
) -> SessionChoices {
    let mut actions = BTreeMap::new();
    let mut selected = 0;
    let all = sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            if session.session_id.as_str() == active_session_id {
                selected = index;
            }
            session_item(session, active_session_id, now_unix_ms, &mut actions)
        })
        .collect::<Vec<_>>();
    let active = filtered_items(&all, sessions, SessionStatus::Active);
    let archived = filtered_items(&all, sessions, SessionStatus::Archived);
    let active_count = active.len();
    let archived_count = archived.len();

    SessionChoices {
        model: ListSelectionModel::new(
            "Resume session",
            vec![
                ListSelectionGroup::new(format!("All ({})", all.len()), all),
                ListSelectionGroup::new(format!("Active ({active_count})"), active),
                ListSelectionGroup::new(format!("Archived ({archived_count})"), archived),
            ],
        )
        .with_activation_mode(ListSelectionActivationMode::Enter)
        .with_activation_label("resume")
        .with_initial_selected(selected)
        .with_search(SearchBoxModel::new("Search saved sessions"))
        .with_empty_message("No matching sessions"),
        actions,
    }
}

fn session_item(
    session: &Session,
    active_session_id: &str,
    now_unix_ms: u64,
    actions: &mut BTreeMap<ListSelectionItemId, SessionSelectionAction>,
) -> ListSelectionItem {
    let item_id = ListSelectionItemId::new(format!("session:{}", session.session_id));
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
        "{}  ·  {}  ·  {}  ·  {}  ·  {}",
        session_time(session, now_unix_ms),
        branch_count_label(session),
        session_size_label(session),
        status_label(session.status),
        session.session_id,
    ))
}

fn session_time(session: &Session, now_unix_ms: u64) -> String {
    let changed_at = session.manager.status_changed_at_unix_ms;
    if changed_at == 0 {
        return "time unknown".into();
    }
    format!(
        "{} ago",
        format_compact_duration(Duration::from_millis(
            now_unix_ms.saturating_sub(changed_at),
        ))
    )
}

fn current_unix_millis() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_millis();
    u64::try_from(millis).expect("Unix millisecond timestamp must fit u64")
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
        SessionStatus::Archived => "archived",
    }
}

#[cfg(test)]
#[path = "picker_tests.rs"]
mod tests;
