use super::{PaneHost, PaneHostScope};
use crate::terminal_session::TerminalSessionKey;
use crate::workbench_host::pane_input::PaneBinding;
use crate::workbench_host::{PaneInput, PaneInputKind, PanePart, PaneSplitDirection, TabInputKey};
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
    let mut layout =
        PanePart::with_input(PaneInput::agent(session("session-1"), thread("thread-1")));
    let root = layout.root_pane();
    let (second, _) = layout.split_active_with_input(
        PaneSplitDirection::Horizontal,
        Some(PaneInput::files("/workspace".into())),
    );
    let mut host = PaneHost::new();

    host.insert(
        (PaneHostScope::Tab(tab_key.clone()), root),
        PaneBinding::new(),
    );
    host.insert(
        (PaneHostScope::Tab(tab_key.clone()), second),
        PaneBinding::new(),
    );

    let first_mount = host
        .mount(&PaneHostScope::Tab(tab_key.clone()), &layout, root)
        .expect("root Pane should mount");
    let second_mount = host
        .mount(&PaneHostScope::Tab(tab_key), &layout, second)
        .expect("split Pane should mount");
    assert_eq!(first_mount.kind(), PaneInputKind::Agent);
    assert_eq!(second_mount.kind(), PaneInputKind::Files);
    assert_eq!(first_mount.terminal_key(), None);
    assert_eq!(second_mount.terminal_key(), None);
}

#[test]
fn terminal_runtime_can_only_attach_to_matching_terminal_input() {
    let tab_key = TabInputKey::session(session("session-1"));
    let layout = PanePart::with_input(PaneInput::terminal(session("session-1")));
    let pane = layout.root_pane();
    let mut host = PaneHost::new();
    let key = (PaneHostScope::Tab(tab_key), pane);
    let input = layout.active_input(pane).expect("terminal input");

    assert!(host.ensure_terminal(
        key.clone(),
        input,
        &session("session-1"),
        TerminalSessionKey::new(1),
    ));
    assert_eq!(host.terminal_key(&key), Some(TerminalSessionKey::new(1)));
    assert!(!host.ensure_terminal(
        key,
        input,
        &session("session-2"),
        TerminalSessionKey::new(2),
    ));
}

#[test]
fn removing_a_tab_releases_all_of_its_runtime_bindings() {
    let closed_tab = TabInputKey::session(session("session-closed"));
    let kept_tab = TabInputKey::session(session("session-kept"));
    let mut host = PaneHost::new();
    let closed_root =
        PanePart::with_input(PaneInput::terminal(session("session-closed"))).root_pane();
    let kept_root = PanePart::with_input(PaneInput::terminal(session("session-kept"))).root_pane();
    let closed_key = (PaneHostScope::Tab(closed_tab.clone()), closed_root);
    let kept_key = (PaneHostScope::Tab(kept_tab.clone()), kept_root);

    host.insert(
        closed_key.clone(),
        PaneBinding::terminal(TerminalSessionKey::new(1)),
    );
    host.insert(
        kept_key.clone(),
        PaneBinding::terminal(TerminalSessionKey::new(2)),
    );

    let removed = host.remove_tab(&closed_tab);
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].terminal_key(), Some(TerminalSessionKey::new(1)));
    assert!(host.binding(&closed_key).is_none());
    assert!(host.binding(&kept_key).is_some());
}
