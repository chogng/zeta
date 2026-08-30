use zeta_commands::AppCommandId;

use super::WorkbenchCommandDispatch;
use super::command_request_for_element;
use crate::ADD_SESSION;
use crate::FILES_PANE_TOGGLE;
use crate::TAB_CONTAINER_TOGGLE;
use crate::WorkbenchHost;
use zeta_files::FILES_REFRESH;
use zeta_session::interaction::CONTEXT_DIFF;

#[test]
fn workbench_elements_resolve_to_their_stable_commands() {
    assert_eq!(
        command_request_for_element(TAB_CONTAINER_TOGGLE).map(|request| request.command_id()),
        Some(AppCommandId::ToggleTabContainer)
    );
    assert_eq!(
        command_request_for_element(FILES_PANE_TOGGLE).map(|request| request.command_id()),
        Some(AppCommandId::ToggleFilesPane)
    );
    assert_eq!(
        command_request_for_element(ADD_SESSION).map(|request| request.command_id()),
        Some(AppCommandId::AddSession)
    );
    assert_eq!(
        command_request_for_element(FILES_REFRESH).map(|request| request.command_id()),
        Some(AppCommandId::RefreshAgentFiles)
    );
    assert_eq!(
        command_request_for_element(CONTEXT_DIFF).map(|request| request.command_id()),
        Some(AppCommandId::ShowGitDiff)
    );
}

#[test]
fn workbench_executes_its_commands_and_routes_capability_commands() {
    let mut workbench = WorkbenchHost::<()>::new();
    assert!(workbench.tab_container_state().is_expanded());

    assert_eq!(
        workbench.dispatch_command(AppCommandId::ToggleTabContainer.into()),
        WorkbenchCommandDispatch::Handled
    );
    assert!(!workbench.tab_container_state().is_expanded());
    assert_eq!(
        workbench.dispatch_command(AppCommandId::AddSession.into()),
        WorkbenchCommandDispatch::Capability(AppCommandId::AddSession.into())
    );
}
