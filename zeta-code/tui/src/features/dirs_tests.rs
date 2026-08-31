use super::DirSelectionAction;
use super::choices;
use crate::components::list_selection::ListSelectionState;
use std::path::PathBuf;
use zeta_app_server_protocol::protocol::environment::PermissionDto;
use zeta_app_server_protocol::protocol::environment::SessionDirDto;
use zeta_app_server_protocol::protocol::environment::SessionDirListResult;

#[test]
fn dir_view_maps_exact_paths_to_remove_actions() {
    let view = choices(SessionDirListResult {
        revision: 3,
        dirs: vec![SessionDirDto {
            contributions: Default::default(),
            path: PathBuf::from("/dir/shared"),
            permissions: vec![PermissionDto::ReadFiles],
        }],
    });

    assert_eq!(ListSelectionState::new(view.model).title(), "Directories");
    assert!(matches!(
        view.actions.values().next(),
        Some(DirSelectionAction::Remove { path })
            if path == &PathBuf::from("/dir/shared")
    ));
}
