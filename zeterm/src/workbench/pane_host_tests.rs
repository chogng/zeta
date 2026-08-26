use super::{PaneHost, PaneHostScope};
use crate::pane_group::{PaneGroup, PaneSplitDirection};
use crate::pane_input::{PaneBinding, PaneInput, PaneInputKind};
use crate::tab_input::TabInputKey;
use crate::terminal_session::TerminalSessionKey;
use zeta_protocol::{SessionId, ThreadId};

fn session(value: &str) -> SessionId {
    SessionId::new(value).expect("test session ID is non-empty")
}

fn thread(value: &str) -> ThreadId {
    ThreadId::new(value).expect("test thread ID is non-empty")
}

#[test]
fn host_mounts_heterogeneous_inputs_in_one_group() {
    let tab_key = TabInputKey::session(session("session-1"));
    let mut group = PaneGroup::new();
    let root = group.root_pane();
    let second = group.split_active(PaneSplitDirection::Horizontal);
    let mut host = PaneHost::new();

    host.insert(
        (PaneHostScope::Tab(tab_key.clone()), root),
        PaneBinding::new(PaneInput::agent(session("session-1"), thread("thread-1"))),
    );
    host.insert(
        (PaneHostScope::Tab(tab_key.clone()), second),
        PaneBinding::new(PaneInput::files("/workspace".into())),
    );

    let first_mount = host
        .mount(&PaneHostScope::Tab(tab_key.clone()), &group, root)
        .expect("root Pane should mount");
    let second_mount = host
        .mount(&PaneHostScope::Tab(tab_key), &group, second)
        .expect("split Pane should mount");
    assert_eq!(first_mount.kind(), PaneInputKind::Agent);
    assert_eq!(second_mount.kind(), PaneInputKind::Files);
    assert_eq!(first_mount.terminal_key(), None);
    assert_eq!(second_mount.terminal_key(), None);
}

#[test]
fn terminal_runtime_can_only_attach_to_matching_terminal_input() {
    let tab_key = TabInputKey::session(session("session-1"));
    let mut host = PaneHost::new();
    let key = (PaneHostScope::Tab(tab_key), PaneGroup::new().root_pane());

    assert!(host.ensure_terminal(
        key.clone(),
        &session("session-1"),
        TerminalSessionKey::new(1),
    ));
    assert_eq!(host.terminal_key(&key), Some(TerminalSessionKey::new(1)));
    assert!(!host.ensure_terminal(key, &session("session-2"), TerminalSessionKey::new(2),));
}
