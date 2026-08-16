use super::ThreadSelectionAction;
use super::ThreadSelectionPurpose;
use super::thread_selection_view;
use crate::components::selection::SelectionViewState;
use zeta_protocol::DelegationId;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::SessionThreadStatus;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadOrigin;

#[test]
fn switch_view_marks_the_current_thread_and_keeps_archived_history_navigable() {
    let current = ThreadId::new("thread-1").unwrap();
    let session = session();
    let view = thread_selection_view(&session, &current, ThreadSelectionPurpose::Switch);
    let state = SelectionViewState::new(view.model.into_body());

    assert_eq!(state.title(), "Switch thread");
    assert_eq!(state.tabs()[0].label(), "All (2)");
    assert_eq!(state.visible_items()[0].label(), "thread-1 ✓");
    assert!(view.actions.values().any(|action| matches!(
        action,
        ThreadSelectionAction::Switch { thread_id } if thread_id.as_str() == "thread-2"
    )));
}

#[test]
fn archive_view_excludes_already_archived_threads() {
    let session = session();
    let view = thread_selection_view(
        &session,
        &ThreadId::new("thread-1").unwrap(),
        ThreadSelectionPurpose::Archive,
    );
    let state = SelectionViewState::new(view.model.into_body());

    assert_eq!(state.tabs()[0].label(), "All (1)");
    assert_eq!(state.visible_items()[0].label(), "thread-1 ✓");
}

#[test]
fn switch_view_describes_agent_threads_by_parent_and_delegation() {
    let parent_thread_id = ThreadId::new("thread-parent").unwrap();
    let child_thread_id = ThreadId::new("thread-agent").unwrap();
    let session = Session {
        session_id: SessionId::new("session-agent").unwrap(),
        title: "Agent session".into(),
        status: SessionStatus::Active,
        model: None,
        workspace: None,
        sequence: 1,
        threads: vec![SessionThread {
            thread_id: child_thread_id.clone(),
            origin: ThreadOrigin::AgentSpawn {
                parent_thread_id,
                parent_sequence: 1,
                delegation_id: DelegationId::new("delegation-1").unwrap(),
            },
            status: SessionThreadStatus::Active,
        }],
    };

    let view = thread_selection_view(&session, &child_thread_id, ThreadSelectionPurpose::Switch);
    let state = SelectionViewState::new(view.model.into_body());

    assert_eq!(
        state.visible_items()[0].description(),
        Some("active  ·  agent spawned by thread-parent for delegation-1")
    );
}

fn session() -> Session {
    Session {
        session_id: SessionId::new("session-1").unwrap(),
        title: "Session".into(),
        status: SessionStatus::Active,
        model: None,
        workspace: None,
        sequence: 3,
        threads: vec![
            SessionThread {
                thread_id: ThreadId::new("thread-1").unwrap(),
                origin: ThreadOrigin::Root,
                status: SessionThreadStatus::Active,
            },
            SessionThread {
                thread_id: ThreadId::new("thread-2").unwrap(),
                origin: ThreadOrigin::Fork {
                    parent_thread_id: ThreadId::new("thread-1").unwrap(),
                    parent_sequence: 2,
                },
                status: SessionThreadStatus::Archived,
            },
        ],
    }
}
