use super::choices;
use crate::TuiOptions;
use crate::TuiRecoveryState;
use crate::widgets::list_selection::ListSelectionState;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

#[test]
fn startup_choices_show_the_effective_new_session_context() {
    let options = TuiOptions::new("TUI conversation")
        .with_dir_root("/workspace/zeta")
        .with_profile_root("/profile");
    let model = choices(&options.startup_context());
    let key_hints = model.key_hints().text().to_owned();
    let mut state = ListSelectionState::new(model);

    assert_eq!(state.title(), "Startup");
    assert!(!state.show_tabs());
    assert_eq!(key_hints, "↑↓/jk to choose  ·  Esc to close");
    assert_eq!(
        state
            .visible_items()
            .iter()
            .map(|item| (item.label(), item.description()))
            .collect::<Vec<_>>(),
        vec![
            ("Mode", Some("New")),
            ("Workspace", Some("/workspace/zeta")),
            ("Profile", Some("/profile")),
            ("Connection", Some("Local App Server")),
        ]
    );
    assert!(state.activate_visible_item(0).is_none());
}

#[test]
fn startup_choices_show_resume_identity_for_a_recovered_session() {
    let options = TuiOptions::new("TUI conversation")
        .with_remote_dir("/srv/project")
        .with_recovery(TuiRecoveryState::new(
            SessionId::new("session-1").unwrap(),
            ThreadId::new("thread-1").unwrap(),
        ));
    let state = ListSelectionState::new(choices(&options.startup_context()));

    assert_eq!(
        state
            .visible_items()
            .iter()
            .map(|item| (item.label(), item.description()))
            .collect::<Vec<_>>(),
        vec![
            ("Mode", Some("Resume")),
            ("Workspace", Some("/srv/project")),
            ("Profile", Some("default")),
            ("Connection", Some("Remote App Server")),
            ("Session", Some("session-1")),
            ("Thread", Some("thread-1")),
        ]
    );
}
