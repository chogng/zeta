use super::{PaneBinding, PaneInput, PaneInputKind};
use crate::terminal_session::TerminalSessionKey;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

fn session(value: &str) -> SessionId {
    SessionId::new(value).expect("test session ID is non-empty")
}

fn thread(value: &str) -> ThreadId {
    ThreadId::new(value).expect("test thread ID is non-empty")
}

#[test]
fn pane_binding_keeps_input_separate_from_optional_runtime() {
    let input = PaneInput::agent(session("session-1"), thread("thread-1"));
    let binding = PaneBinding::new(input.clone());

    assert_eq!(binding.input(), &input);
    assert_eq!(binding.terminal_key(), None);

    let mut terminal = PaneBinding::terminal(session("session-1"), TerminalSessionKey::new(7));
    assert_eq!(terminal.input().kind(), PaneInputKind::Terminal);
    assert_eq!(
        terminal.input().terminal_session_id(),
        Some(&session("session-1"))
    );
    assert_eq!(terminal.terminal_key(), Some(TerminalSessionKey::new(7)));
    assert!(!terminal.bind_terminal(&session("session-2"), TerminalSessionKey::new(8)));
    assert!(terminal.bind_terminal(&session("session-1"), TerminalSessionKey::new(8)));
    assert_eq!(terminal.terminal_key(), Some(TerminalSessionKey::new(8)));
}

#[test]
fn non_terminal_binding_rejects_terminal_runtime() {
    let mut binding = PaneBinding::new(PaneInput::settings());

    assert!(!binding.bind_terminal(&session("session-1"), TerminalSessionKey::new(1)));
    assert_eq!(binding.terminal_key(), None);
}
