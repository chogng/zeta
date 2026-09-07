use super::DirSelectionAction;
use super::choices;
use crate::widgets::list_selection::ListSelectionInputOutcome;
use crate::widgets::list_selection::ListSelectionState;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use std::path::PathBuf;
use zeta_app_server_protocol::protocol::environment::PermissionDto;
use zeta_app_server_protocol::protocol::environment::SessionDirDto;
use zeta_app_server_protocol::protocol::environment::SessionDirListResult;

#[test]
fn empty_directories_open_and_dismiss_without_an_action() {
    let view = choices(
        &zeta_protocol::SessionId::new("session").unwrap(),
        SessionDirListResult {
            revision: 0,
            dirs: Vec::new(),
        },
    );
    assert!(view.actions.is_empty());
    let mut state = ListSelectionState::new(view.model);
    assert!(!state.show_tabs());
    assert!(state.visible_items().is_empty());
    assert_eq!(state.empty_message(), "No directories");
    assert_eq!(state.selected_visible_index(), None);
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ListSelectionInputOutcome::Consumed,
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        ListSelectionInputOutcome::Dismiss,
    );
}

#[test]
fn every_directory_is_reachable_and_removing_the_last_clears_selection() {
    let session = zeta_protocol::SessionId::new("session").unwrap();
    let view = choices(
        &session,
        SessionDirListResult {
            revision: 3,
            dirs: ["/dir/first", "/dir/second"]
                .map(|path| SessionDirDto {
                    contributions: Default::default(),
                    path: PathBuf::from(path),
                    permissions: Vec::new(),
                })
                .to_vec(),
        },
    );
    let mut state = ListSelectionState::new(view.model);
    assert!(state.show_tabs());
    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let ListSelectionInputOutcome::Activate(id) =
        state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("the second directory must be selectable")
    };
    assert_eq!(
        view.actions.get(&id),
        Some(&DirSelectionAction::Remove {
            path: PathBuf::from("/dir/second"),
        })
    );
    let empty = choices(
        &session,
        SessionDirListResult {
            revision: 4,
            dirs: Vec::new(),
        },
    );
    state.replace_model(empty.model);
    assert!(!state.show_tabs());
    assert!(state.visible_items().is_empty());
    assert_eq!(state.selected_visible_index(), None);
}

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
