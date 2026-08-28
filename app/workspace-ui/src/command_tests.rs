use zeta_commands::AppCommandId;

use super::WorkspacePaneSelection;
use super::command_request_for_element;
use crate::interaction::AGENT_FILES_REFRESH;
use crate::interaction::AGENT_FILES_SEARCH;

#[test]
fn workspace_pane_selections_have_stable_labels_and_identities() {
    let ids = WorkspacePaneSelection::ALL.map(WorkspacePaneSelection::element_id);
    let labels = WorkspacePaneSelection::ALL.map(WorkspacePaneSelection::label);

    assert_eq!(
        ids,
        [
            WorkspacePaneSelection::Changes.element_id(),
            WorkspacePaneSelection::Files.element_id(),
        ]
    );
    assert_eq!(labels, ["Changes", "Files"]);
    assert_eq!(
        WorkspacePaneSelection::from_element_id(WorkspacePaneSelection::Files.element_id()),
        Some(WorkspacePaneSelection::Files)
    );
}

#[test]
fn workspace_elements_resolve_to_their_stable_commands() {
    let expected = [
        (
            WorkspacePaneSelection::Changes.element_id(),
            AppCommandId::ShowAgentChanges,
        ),
        (
            WorkspacePaneSelection::Files.element_id(),
            AppCommandId::ShowAgentFiles,
        ),
        (AGENT_FILES_REFRESH, AppCommandId::RefreshAgentFiles),
        (AGENT_FILES_SEARCH, AppCommandId::ToggleAgentFileSearch),
    ];

    for (element, command) in expected {
        assert_eq!(
            command_request_for_element(element).map(|request| request.command_id()),
            Some(command)
        );
    }
}
