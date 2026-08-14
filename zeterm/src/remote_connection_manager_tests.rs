use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;
use zeta_remote_connections::RemoteConnectionEntry;
use zeta_remote_connections::RemoteConnectionName;
use zeta_ui::TextInputCommand;

use super::RemoteConnectionManagerField;
use super::RemoteConnectionManagerState;
use super::RemoteConnectionSaveRequest;

#[test]
fn create_edit_connect_and_delete_requests_preserve_saved_identity() {
    let mut state = RemoteConnectionManagerState::default();
    state.open(Vec::new(), None);
    assert!(state.is_open());
    assert!(state.selected_name().is_none());

    insert(&mut state, RemoteConnectionManagerField::Name, "BUILD-01");
    insert(
        &mut state,
        RemoteConnectionManagerField::Host,
        "build.example",
    );
    insert(
        &mut state,
        RemoteConnectionManagerField::Workspace,
        "/srv/project",
    );
    let RemoteConnectionSaveRequest::Create(created) = state.save_request().unwrap() else {
        panic!("new drafts create connections");
    };
    assert_eq!(created.name().as_str(), "build-01");
    state.save_succeeded(created.clone());
    assert_eq!(state.connect_request(), Some(name("build-01")));

    state.apply(
        RemoteConnectionManagerField::Name,
        TextInputCommand::SelectAll,
    );
    insert(&mut state, RemoteConnectionManagerField::Name, "STAGING");
    let RemoteConnectionSaveRequest::Update { original, entry } = state.save_request().unwrap()
    else {
        panic!("saved drafts update their original identity");
    };
    assert_eq!(original, name("build-01"));
    assert_eq!(entry.name(), &name("staging"));
    assert!(state.connect_request().is_none());
    state.save_succeeded(entry);
    assert_eq!(state.connect_request(), Some(name("staging")));

    assert!(state.delete_request().is_none());
    assert_eq!(state.delete_label(), "Confirm Delete");
    assert_eq!(state.delete_request(), Some(name("staging")));
    state.delete_succeeded(&name("staging"));
    assert!(state.connections().is_empty());
    assert!(state.selected_name().is_none());
}

#[test]
fn selection_is_sorted_and_refuses_to_discard_dirty_drafts() {
    let mut state = RemoteConnectionManagerState::default();
    state.open(
        vec![
            entry("staging", "staging.example", "/srv/staging"),
            entry("build", "build.example", "/srv/build"),
        ],
        None,
    );
    assert_eq!(state.selected_name(), Some(&name("build")));
    assert!(state.select(1));
    assert_eq!(state.selected_name(), Some(&name("staging")));

    insert(
        &mut state,
        RemoteConnectionManagerField::Workspace,
        "/changed",
    );
    assert!(!state.select(0));
    assert_eq!(state.selected_name(), Some(&name("staging")));
    assert!(state.status().unwrap().0.contains("Save or close"));
    assert!(!state.start_new());
}

#[test]
fn invalid_drafts_report_the_canonical_field_error() {
    let mut state = RemoteConnectionManagerState::default();
    state.open(Vec::new(), None);
    insert(&mut state, RemoteConnectionManagerField::Name, "bad name");
    insert(&mut state, RemoteConnectionManagerField::Host, "host");
    insert(
        &mut state,
        RemoteConnectionManagerField::Workspace,
        "relative",
    );

    assert!(state.save_request().is_none());
    let (message, error) = state.status().unwrap();
    assert!(error);
    assert!(message.contains("name must contain"));
}

#[test]
fn child_launch_progress_locks_mutation_and_failure_is_retryable() {
    let mut state = RemoteConnectionManagerState::default();
    state.open(vec![entry("build", "build.example", "/srv/project")], None);
    state.launch_started(name("build"));
    assert!(state.is_launching());
    assert!(!state.can_mutate());
    assert!(!state.can_connect());
    assert!(!state.can_delete());

    let original_host = state
        .input(RemoteConnectionManagerField::Host)
        .text()
        .to_owned();
    state.apply(
        RemoteConnectionManagerField::Host,
        TextInputCommand::Insert("ignored".into()),
    );
    assert_eq!(
        state.input(RemoteConnectionManagerField::Host).text(),
        original_host
    );

    state.launch_progress("Uploading Remote runtime… 50%");
    assert_eq!(state.status().unwrap().0, "Uploading Remote runtime… 50%");
    state.launch_failed("server unavailable");
    assert!(!state.is_launching());
    assert!(state.can_connect());
    assert_eq!(state.status(), Some(("server unavailable", true)));
}

fn insert(
    state: &mut RemoteConnectionManagerState,
    field: RemoteConnectionManagerField,
    value: &str,
) {
    state.apply(field, TextInputCommand::Insert(value.into()));
}

fn name(value: &str) -> RemoteConnectionName {
    RemoteConnectionName::parse(value).unwrap()
}

fn entry(name_value: &str, host: &str, workspace: &str) -> RemoteConnectionEntry {
    RemoteConnectionEntry::new(
        name(name_value),
        SshTarget::new(
            SshHost::parse(host).unwrap(),
            RemoteWorkspacePath::parse(workspace).unwrap(),
        ),
    )
}
