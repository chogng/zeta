use super::DirSelectionAction;
use super::choices;
use crate::widgets::list_selection::ListSelectionState;
use std::path::PathBuf;
use zeta_app_server_protocol::protocol::environment::PermissionDto;
use zeta_app_server_protocol::protocol::environment::SessionDirDto;
use zeta_app_server_protocol::protocol::environment::SessionDirListResult;

#[test]
fn dir_view_maps_exact_paths_to_remove_actions() {
    let view = choices(
        &zeta_protocol::SessionId::new("session").unwrap(),
        SessionDirListResult {
            revision: 3,
            dirs: vec![SessionDirDto {
                contributions: Default::default(),
                path: PathBuf::from("/dir/shared"),
                permissions: vec![PermissionDto::ReadFiles],
            }],
        },
    );

    let state = ListSelectionState::new(view.model);
    assert_eq!(state.title(), "Directories");
    assert!(matches!(
        view.actions
            .get(state.visible_items()[0].id().unwrap()),
        Some(DirSelectionAction::Remove { path })
            if path == &PathBuf::from("/dir/shared")
    ));
    assert!(matches!(
        view.actions
            .get(state.visible_items()[1].id().unwrap()),
        Some(DirSelectionAction::SetPermissions(params))
            if params.session_id.as_str() == "session"
                && params.expected_revision == 3
                && params.permissions.is_empty()
    ));
}
