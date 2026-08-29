use super::AdditionalDirectorySelectionAction;
use super::list_selection;
use crate::components::list_selection::ListSelectionState;
use std::path::PathBuf;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryPermissionDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustStateDto;

#[test]
fn additional_directory_view_maps_exact_roots_to_remove_actions() {
    let view = list_selection(WorkspaceAdditionalDirectoryListResult {
        revision: 3,
        directories: vec![WorkspaceAdditionalDirectoryDto {
            contributions: Default::default(),
            root: PathBuf::from("/workspace/shared"),
            trust: WorkspaceTrustStateDto::Restricted,
            permissions: vec![WorkspaceAdditionalDirectoryPermissionDto::ReadFiles],
        }],
    });

    assert_eq!(
        ListSelectionState::new(view.model.into_body()).title(),
        "Additional directories"
    );
    assert!(matches!(
        view.actions.values().next(),
        Some(AdditionalDirectorySelectionAction::Remove { root })
            if root == &PathBuf::from("/workspace/shared")
    ));
}
