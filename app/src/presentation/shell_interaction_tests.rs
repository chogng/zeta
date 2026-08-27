use super::{
    ADD_SESSION, AGENT_CHANGES, AGENT_FILE_SEARCH_INPUT, AGENT_FILES, CONTEXT_DIFF,
    CONTEXT_GIT_BRANCH, CONTEXT_LOCATION, CONTEXT_WORKING_DIRECTORY, ContextAction,
    FIRST_TAB_CONTAINER_SESSION_TAB, FIRST_TITLEBAR_SESSION_TAB, SESSION_CONTEXT_MENU,
    SESSION_HEADER, SESSION_SEARCH_INPUT, SessionContextMenuAction, TAB_CONTAINER_ACTION_BAR,
    TAB_CONTAINER_LIST, TAB_CONTAINER_TOOLBAR, TITLEBAR_TAB_LIST, WorkspacePaneSelection,
    session_tab_id, session_tab_index, titlebar_session_tab_id,
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
fn session_tab_identities_are_unique_and_round_trip_to_their_indices() {
    let body_ids = (0..4).map(session_tab_id).collect::<Vec<_>>();
    let titlebar_ids = (0..4).map(titlebar_session_tab_id).collect::<Vec<_>>();

    assert_eq!(
        body_ids
            .iter()
            .chain(&titlebar_ids)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        8
    );
    assert_eq!(
        body_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
    for (index, id) in body_ids.into_iter().enumerate() {
        assert_eq!(session_tab_index(id, 0..4), Some(index));
    }
    for (index, id) in titlebar_ids.into_iter().enumerate() {
        assert_eq!(session_tab_index(id, 0..4), Some(index));
    }
}

#[test]
fn session_identity_namespace_does_not_overlap_workspace_pane_elements() {
    let session_ids = [
        TAB_CONTAINER_LIST,
        FIRST_TAB_CONTAINER_SESSION_TAB,
        TITLEBAR_TAB_LIST,
        FIRST_TITLEBAR_SESSION_TAB,
        SESSION_HEADER,
        session_tab_id(1),
    ];
    let workspace_pane_ids = [AGENT_FILE_SEARCH_INPUT, super::AGENT_FILES_TOOLBAR];

    assert!(
        session_ids
            .into_iter()
            .all(|session| !workspace_pane_ids.contains(&session))
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
