use super::WORKSPACE_CHANGES;
use super::WORKSPACE_FILES;
use super::WorkspacePaneSelection;
use crate::PaneInputKind;

#[test]
fn workspace_pane_selection_is_the_navigation_form_of_pane_kind() {
    assert_eq!(
        WorkspacePaneSelection::ALL.map(WorkspacePaneSelection::element_id),
        [WORKSPACE_CHANGES, WORKSPACE_FILES]
    );
    assert_eq!(
        WorkspacePaneSelection::ALL.map(WorkspacePaneSelection::label),
        ["Changes", "Files"]
    );
    assert_eq!(
        WorkspacePaneSelection::from_pane_kind(PaneInputKind::Diff),
        Some(WorkspacePaneSelection::Changes)
    );
    assert_eq!(
        WorkspacePaneSelection::from_pane_kind(PaneInputKind::Files),
        Some(WorkspacePaneSelection::Files)
    );
}
