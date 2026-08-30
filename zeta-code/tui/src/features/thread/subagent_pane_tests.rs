use super::SubagentPaneState;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

#[test]
fn completed_subagents_disappear_without_changing_stable_selection() {
    let mut session = session();
    let mut pane = SubagentPaneState::default();
    let selected = thread_id("child-b");
    pane.reconcile(Some(&session), Some(&selected));
    pane.focus();

    session.threads[1].status = ThreadStatus::Archived;
    pane.reconcile(Some(&session), Some(&selected));

    assert_eq!(pane.selected(), Some(&selected));
    assert_eq!(pane.view().rows.len(), 2);
}

#[test]
fn selection_drives_a_bounded_viewport() {
    let mut session = session();
    for index in 0..6 {
        session.threads.push(child(&format!("extra-{index}")));
    }
    let mut pane = SubagentPaneState::default();
    pane.reconcile(Some(&session), Some(&thread_id("root")));
    pane.focus();
    for _ in 0..6 {
        pane.select_next();
    }

    assert_eq!(pane.view().rows.len(), 4);
    assert!(
        pane.view()
            .rows
            .iter()
            .any(|row| Some(&row.thread_id) == pane.selected())
    );
}

fn session() -> Session {
    Session {
        session_id: session_id("root"),
        title: "Session".into(),
        status: SessionStatus::Active,
        threads: vec![root(), child("child-a"), child("child-b")],
    }
}

fn root() -> SessionThread {
    SessionThread {
        thread_id: thread_id("root"),
        parent_thread_id: None,
        forked_from_id: None,
        status: ThreadStatus::Active,
    }
}

fn child(value: &str) -> SessionThread {
    SessionThread {
        thread_id: thread_id(value),
        parent_thread_id: Some(thread_id("root")),
        forked_from_id: None,
        status: ThreadStatus::Active,
    }
}

fn session_id(value: &str) -> SessionId {
    SessionId::new(value).unwrap()
}

fn thread_id(value: &str) -> ThreadId {
    ThreadId::new(value).unwrap()
}
