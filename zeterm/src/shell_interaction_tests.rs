use super::{
    ACTIVE_SESSION_TAB, ADD_SESSION, AGENT_CHANGES, AGENT_FILE_SEARCH_INPUT, AGENT_FILES,
    AgentSidebarPaneAction, CONTEXT_DIFF, CONTEXT_GIT_BRANCH, CONTEXT_LOCATION,
    CONTEXT_WORKING_DIRECTORY, ContextAction, SESSION_CONTEXT_MENU, SESSION_HEADER,
    SESSION_SEARCH_INPUT, SESSION_SIDEBAR_ACTION_BAR, SESSION_SIDEBAR_TOOLBAR, SESSION_TAB_LIST,
    SessionContextMenuAction, session_tab_id, session_tab_index,
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
fn sidebar_part_pane_actions_have_stable_labels_and_identities() {
    let ids = AgentSidebarPaneAction::ALL.map(AgentSidebarPaneAction::element_id);
    let labels = AgentSidebarPaneAction::ALL.map(AgentSidebarPaneAction::label);

    assert_eq!(ids, [AGENT_CHANGES, AGENT_FILES]);
    assert_eq!(labels, ["Changes", "Files"]);
    assert_eq!(
        AgentSidebarPaneAction::from_element_id(AGENT_FILES),
        Some(AgentSidebarPaneAction::Files)
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
fn session_tab_identities_are_unique_and_round_trip_to_their_indices() {
    let ids = (0..4).map(session_tab_id).collect::<Vec<_>>();

    assert_eq!(
        ids.iter().collect::<std::collections::HashSet<_>>().len(),
        4
    );
    for (index, id) in ids.into_iter().enumerate() {
        assert_eq!(session_tab_index(id, 0..4), Some(index));
    }
}

#[test]
fn session_identity_namespace_does_not_overlap_sidebar_part_elements() {
    let session_ids = [
        SESSION_TAB_LIST,
        ACTIVE_SESSION_TAB,
        SESSION_HEADER,
        session_tab_id(1),
    ];
    let sidebar_part_ids = [
        AGENT_FILE_SEARCH_INPUT,
        zeta_agent_sidebar::AGENT_FILES_TOOLBAR,
    ];

    assert!(
        session_ids
            .into_iter()
            .all(|session| !sidebar_part_ids.contains(&session))
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
