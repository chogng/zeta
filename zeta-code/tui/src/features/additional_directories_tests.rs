use super::AdditionalDirectorySelectionAction;
use super::selection_view;
use crate::components::selection::SelectionViewState;
use std::path::PathBuf;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryPermissionDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustStateDto;

#[test]
fn additional_directory_view_maps_exact_roots_to_remove_actions() {
    let view = selection_view(WorkspaceAdditionalDirectoryListResult {
        revision: 3,
        directories: vec![WorkspaceAdditionalDirectoryDto {
            contributions: Default::default(),
            root: PathBuf::from("/workspace/shared"),
            trust: WorkspaceTrustStateDto::Restricted,
            permissions: vec![WorkspaceAdditionalDirectoryPermissionDto::ReadFiles],
        }],
    });

    assert_eq!(
        SelectionViewState::new(view.model.into_body()).title(),
        "Additional directories"
    );
    assert!(matches!(
        view.actions.values().next(),
        Some(AdditionalDirectorySelectionAction::Remove { root })
            if root == &PathBuf::from("/workspace/shared")
    ));
}
