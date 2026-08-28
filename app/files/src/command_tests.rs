use zeta_commands::AppCommandId;

use super::FILES_REFRESH;
use super::FILES_SEARCH;
use super::command_request_for_element;

#[test]
fn files_elements_resolve_to_their_stable_commands() {
    let expected = [
        (FILES_REFRESH, AppCommandId::RefreshAgentFiles),
        (FILES_SEARCH, AppCommandId::ToggleAgentFileSearch),
    ];

    for (element, command) in expected {
        assert_eq!(
            command_request_for_element(element).map(|request| request.command_id()),
            Some(command)
        );
    }
}
