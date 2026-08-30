use crate::components::list_selection::ListSelectionActivationMode;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
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

pub(crate) struct SessionPaneSpec {
    pub(crate) model: PaneSpec<ListSelectionModel>,
    pub(crate) actions: BTreeMap<ListSelectionItemId, SessionSelectionAction>,
}

pub(crate) fn session_pane_spec(sessions: &[Session], active_session_id: &str) -> SessionPaneSpec {
    session_pane_spec_at(sessions, active_session_id, current_unix_millis())
}

fn session_pane_spec_at(
    sessions: &[Session],
    active_session_id: &str,
    now_unix_ms: u64,
) -> SessionPaneSpec {
    let mut actions = BTreeMap::new();
    let mut selected = 0;
    let all = sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            if session.session_id.as_str() == active_session_id {
                selected = index;
            }
            session_item(index, session, active_session_id, now_unix_ms, &mut actions)
        })
        .collect::<Vec<_>>();
    let active = filtered_items(&all, sessions, SessionStatus::Active);
    let archived = filtered_items(&all, sessions, SessionStatus::Archived);
    let active_count = active.len();
    let archived_count = archived.len();

    SessionPaneSpec {
        model: PaneSpec::new(
            ListSelectionModel::new(
                "Resume session",
                vec![
                    ListSelectionGroup::new(format!("All ({})", all.len()), all),
                    ListSelectionGroup::new(format!("Active ({active_count})"), active),
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
    now_unix_ms: u64,
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
        "{}  ·  {}  ·  {}  ·  {}  ·  {}",
        session_time(session, now_unix_ms),
        branch_count(session),
        session_size(session),
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

fn branch_count(session: &Session) -> String {
    let count = session.threads.len();
    format!("{count} {}", if count == 1 { "branch" } else { "branches" })
}

fn session_size(session: &Session) -> String {
    let mut tokens = 0u64;
    let mut complete = true;
    for thread in &session.threads {
        tokens = tokens
            .saturating_add(thread.usage.input_tokens.reported)
            .saturating_add(thread.usage.output_tokens.reported);
        complete &= thread.usage.input_tokens.complete && thread.usage.output_tokens.complete;
    }
    let prefix = if complete { "" } else { "≥" };
    format!("{prefix}{} tokens", compact_count(tokens))
}

fn compact_count(count: u64) -> String {
    if count < 1_000 {
        return count.to_string();
    }
    if count < 1_000_000 {
        return format!("{:.1}K", count as f64 / 1_000.0);
    }
    format!("{:.1}M", count as f64 / 1_000_000.0)
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
#[path = "pane_tests.rs"]
mod tests;
