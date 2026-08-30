//! Session Pane interaction identity tests.

use zeta_commands::AppCommandId;

use super::ContextAction;
use super::command_request_for_element;
use super::{CONTEXT_DIFF, CONTEXT_GIT_BRANCH, CONTEXT_LOCATION, CONTEXT_WORKING_DIRECTORY};

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
fn session_context_elements_resolve_to_their_stable_commands() {
    let expected = [
        (ContextAction::Location, AppCommandId::PickExecutionLocation),
        (
            ContextAction::WorkingDirectory,
            AppCommandId::PickWorkingDirectory,
        ),
        (ContextAction::GitBranch, AppCommandId::PickGitBranch),
        (ContextAction::Diff, AppCommandId::ShowGitDiff),
    ];

    for (action, command) in expected {
        assert_eq!(
            command_request_for_element(action.element_id()).map(|request| request.command_id()),
            Some(command)
        );
    }
}
