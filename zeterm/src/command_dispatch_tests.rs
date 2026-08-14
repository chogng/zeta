use zeta_commands::ZetermCommandId;

use super::builtin_command_registry;
use super::command_request_for_element;
use crate::shell_interaction::{
    ADD_SESSION, AGENT_FILES, AGENT_FILES_REFRESH, CONTEXT_DIFF, CONTEXT_LOCATION,
    CONTEXT_WORKING_DIRECTORY, SESSION_SIDEBAR_TOGGLE,
};

#[test]
fn element_entry_points_map_to_stable_product_commands() {
    assert_eq!(
        command_request_for_element(SESSION_SIDEBAR_TOGGLE).map(|request| request.command_id()),
        Some(ZetermCommandId::ToggleSessionSidebar)
    );
    assert_eq!(
        command_request_for_element(ADD_SESSION).map(|request| request.command_id()),
        Some(ZetermCommandId::AddSession)
    );
    assert_eq!(
        command_request_for_element(AGENT_FILES).map(|request| request.command_id()),
        Some(ZetermCommandId::ShowAgentFiles)
    );
    assert_eq!(
        command_request_for_element(AGENT_FILES_REFRESH)
            .expect("refresh command")
            .command_id()
            .id(),
        "workbench.action.refreshAgentFiles"
    );
    assert_eq!(
        command_request_for_element(CONTEXT_DIFF)
            .expect("workspace diff command")
            .command_id()
            .id(),
        "workbench.action.showWorkspaceDiff"
    );
    assert_eq!(
        command_request_for_element(CONTEXT_LOCATION)
            .expect("Remote connection picker command")
            .command_id(),
        ZetermCommandId::PickExecutionLocation
    );
    assert_eq!(
        command_request_for_element(CONTEXT_WORKING_DIRECTORY)
            .expect("workspace picker command")
            .command_id()
            .id(),
        "workbench.action.pickWorkingDirectory"
    );
}

#[test]
fn every_catalog_command_has_a_native_handler() {
    let registry = builtin_command_registry();

    assert_eq!(registry.len(), ZetermCommandId::ALL.len());
    for command in ZetermCommandId::ALL {
        assert!(
            registry.contains(command),
            "missing handler for {}",
            command.id()
        );
    }
}

#[test]
fn only_commands_with_current_execution_are_user_bindable() {
    assert_eq!(
        ZetermCommandId::bindable_from_id("workbench.action.toggleSideBar"),
        Some(ZetermCommandId::ToggleSessionSidebar)
    );
    assert_eq!(
        ZetermCommandId::bindable_from_id("workbench.action.newSession"),
        None
    );
}
