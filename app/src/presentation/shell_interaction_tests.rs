use super::{
    ADD_SESSION, AGENT_CHANGES, AGENT_FILE_SEARCH_INPUT, AGENT_FILES, CONTEXT_DIFF,
    CONTEXT_GIT_BRANCH, CONTEXT_LOCATION, CONTEXT_WORKING_DIRECTORY, ContextAction,
    FIRST_TAB_CONTAINER_SESSION_TAB, FIRST_TITLEBAR_SESSION_TAB, SESSION_HEADER,
    SESSION_SEARCH_INPUT, TAB_CONTAINER_ACTION_BAR, TAB_CONTAINER_LIST, TAB_CONTAINER_TOOLBAR,
    TAB_CONTEXT_MENU, TITLEBAR_TAB_LIST, TabContextMenuAction, WorkspacePaneSelection,
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
fn workspace_pane_selections_have_stable_labels_and_identities() {
    let ids = WorkspacePaneSelection::ALL.map(WorkspacePaneSelection::element_id);
    let labels = WorkspacePaneSelection::ALL.map(WorkspacePaneSelection::label);

    assert_eq!(ids, [AGENT_CHANGES, AGENT_FILES]);
    assert_eq!(labels, ["Changes", "Files"]);
    assert_eq!(
        WorkspacePaneSelection::from_element_id(AGENT_FILES),
        Some(WorkspacePaneSelection::Files)
    );
}

#[test]
fn sessions_toolbar_elements_have_stable_unique_identities() {
    let ids = [
        TAB_CONTAINER_TOOLBAR,
        SESSION_SEARCH_INPUT,
        TAB_CONTAINER_ACTION_BAR,
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
fn session_identity_namespace_does_not_overlap_workspace_pane_elements() {
    let session_ids = [
        TAB_CONTAINER_LIST,
        FIRST_TAB_CONTAINER_SESSION_TAB,
        TITLEBAR_TAB_LIST,
        FIRST_TITLEBAR_SESSION_TAB,
        SESSION_HEADER,
    ];
    let workspace_pane_ids = [AGENT_FILE_SEARCH_INPUT, super::AGENT_FILES_TOOLBAR];

    assert!(
        session_ids
            .into_iter()
            .all(|session| !workspace_pane_ids.contains(&session))
    );
}

#[test]
fn tab_context_menu_actions_have_stable_labels_and_identities() {
    let ids = TabContextMenuAction::ALL.map(TabContextMenuAction::element_id);
    let labels = TabContextMenuAction::ALL.map(|action| action.label(false));

    assert_eq!(labels, ["Pin", "Close", "Move to new group"]);
    assert_eq!(
        ids.into_iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3
    );
    assert_eq!(
        TabContextMenuAction::from_element_id(ids[2]),
        Some(TabContextMenuAction::MoveToNewGroup)
    );
    assert!(TabContextMenuAction::is_menu_element(TAB_CONTEXT_MENU));
}
