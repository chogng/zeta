use super::{
    ADD_SESSION, CONTEXT_DIFF, CONTEXT_GIT_BRANCH, CONTEXT_LOCATION, CONTEXT_WORKING_DIRECTORY,
    ContextAction, SESSION_CONTEXT_MENU, SESSION_SEARCH_INPUT, SESSION_SIDEBAR_ACTION_BAR,
    SESSION_SIDEBAR_TOOLBAR, SessionContextMenuAction,
};

#[test]
fn context_actions_have_stable_unique_element_identities() {
    let ids = ContextAction::ALL.map(ContextAction::element_id);

    assert_eq!(
        ids,
        [
            CONTEXT_LOCATION,
            CONTEXT_WORKING_DIRECTORY,
            CONTEXT_GIT_BRANCH,
            CONTEXT_DIFF,
        ]
    );
    assert_eq!(
        ContextAction::from_element_id(ids[2]),
        Some(ContextAction::GitBranch)
    );
    assert_eq!(
        ids.into_iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
}

#[test]
fn sessions_toolbar_elements_have_stable_unique_identities() {
    let ids = [
        SESSION_SIDEBAR_TOOLBAR,
        SESSION_SEARCH_INPUT,
        SESSION_SIDEBAR_ACTION_BAR,
        ADD_SESSION,
    ];

    assert_eq!(
        ids.into_iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
}

#[test]
fn session_context_menu_actions_have_stable_labels_and_identities() {
    let ids = SessionContextMenuAction::ALL.map(SessionContextMenuAction::element_id);
    let labels = SessionContextMenuAction::ALL.map(SessionContextMenuAction::label);

    assert_eq!(labels, ["Pin", "Close", "Rename", "Fork"]);
    assert_eq!(
        ids.into_iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
    assert_eq!(
        SessionContextMenuAction::from_element_id(ids[3]),
        Some(SessionContextMenuAction::Fork)
    );
    assert!(SessionContextMenuAction::is_menu_element(
        SESSION_CONTEXT_MENU
    ));
}
