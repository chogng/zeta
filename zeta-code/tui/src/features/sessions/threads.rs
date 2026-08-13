use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use std::collections::BTreeMap;
use zeta_protocol::Session;
use zeta_protocol::SessionThread;
use zeta_protocol::SessionThreadStatus;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadOrigin;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ThreadSelectionPurpose {
    Archive,
    Switch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ThreadSelectionAction {
    Archive { thread_id: ThreadId },
    Switch { thread_id: ThreadId },
}

pub(crate) struct ThreadSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, ThreadSelectionAction>,
}

pub(crate) fn thread_selection_view(
    session: &Session,
    current_thread_id: &ThreadId,
    purpose: ThreadSelectionPurpose,
) -> ThreadSelectionView {
    let mut actions = BTreeMap::new();
    let threads = session
        .threads
        .iter()
        .filter(|thread| {
            purpose == ThreadSelectionPurpose::Switch
                || thread.status != SessionThreadStatus::Archived
        })
        .collect::<Vec<_>>();
    let items = threads
        .iter()
        .enumerate()
        .map(|(index, thread)| thread_item(index, thread, current_thread_id, purpose, &mut actions))
        .collect::<Vec<_>>();
    let active = filtered_items(&items, &threads, SessionThreadStatus::Active);
    let archived = filtered_items(&items, &threads, SessionThreadStatus::Archived);
    let title = match purpose {
        ThreadSelectionPurpose::Archive => "Archive thread",
        ThreadSelectionPurpose::Switch => "Switch thread",
    };
    let action = match purpose {
        ThreadSelectionPurpose::Archive => "archive",
        ThreadSelectionPurpose::Switch => "open",
    };
    ThreadSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                title,
                vec![
                    SelectionTab::new(format!("All ({})", items.len()), items),
                    SelectionTab::new(format!("Active ({})", active.len()), active),
                    SelectionTab::new(format!("Archived ({})", archived.len()), archived),
                ],
            )
            .with_search(SearchBoxModel::new("Search thread IDs"))
            .with_empty_message("No matching threads"),
            format!("Space search  ·  ←/→ tabs  ·  ↑/↓ select  ·  Enter {action}  ·  Esc back"),
        ),
        actions,
    }
}

fn thread_item(
    index: usize,
    thread: &SessionThread,
    current_thread_id: &ThreadId,
    purpose: ThreadSelectionPurpose,
    actions: &mut BTreeMap<SelectionItemId, ThreadSelectionAction>,
) -> SelectionItem {
    let item_id = SelectionItemId::new(format!("thread-{index}"));
    let action = match purpose {
        ThreadSelectionPurpose::Archive => ThreadSelectionAction::Archive {
            thread_id: thread.thread_id.clone(),
        },
        ThreadSelectionPurpose::Switch => ThreadSelectionAction::Switch {
            thread_id: thread.thread_id.clone(),
        },
    };
    actions.insert(item_id.clone(), action);
    SelectionItem::new(format!(
        "{}{}",
        thread.thread_id,
        if thread.thread_id == *current_thread_id {
            " ✓"
        } else {
            ""
        }
    ))
    .with_id(item_id)
    .with_description(format!(
        "{}  ·  {}",
        status_label(thread.status),
        origin_label(&thread.origin)
    ))
}

fn filtered_items(
    items: &[SelectionItem],
    threads: &[&SessionThread],
    status: SessionThreadStatus,
) -> Vec<SelectionItem> {
    items
        .iter()
        .zip(threads)
        .filter(|(_, thread)| thread.status == status)
        .map(|(item, _)| item.clone())
        .collect()
}

fn status_label(status: SessionThreadStatus) -> &'static str {
    match status {
        SessionThreadStatus::Creating => "creating",
        SessionThreadStatus::Active => "active",
        SessionThreadStatus::Archived => "archived",
    }
}

fn origin_label(origin: &ThreadOrigin) -> String {
    match origin {
        ThreadOrigin::Root => "root".into(),
        ThreadOrigin::Fork {
            parent_thread_id, ..
        } => format!("fork of {parent_thread_id}"),
        ThreadOrigin::Rewind {
            parent_thread_id,
            before_turn_id,
            ..
        } => format!("rewind of {parent_thread_id} before {before_turn_id}"),
        ThreadOrigin::AgentSpawn {
            parent_thread_id,
            delegation_id,
            ..
        } => format!("agent spawned by {parent_thread_id} for {delegation_id}"),
    }
}

#[cfg(test)]
#[path = "threads_tests.rs"]
mod tests;
