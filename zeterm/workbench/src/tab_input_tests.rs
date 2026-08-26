//! Tab input model behavior tests.

use super::TabInputChange;
use super::TabInputKey;
use super::TabInputModel;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;

fn session(id: &str, title: &str) -> Session {
    Session {
        session_id: SessionId::new(id).unwrap(),
        title: title.to_owned(),
        status: SessionStatus::Active,
        model: None,
        workspace: None,
        sequence: 1,
        threads: Vec::new(),
    }
}

#[test]
fn session_upsert_creates_inputs_and_selects_the_newest() {
    let first = session("session-1", "First terminal");
    let second = session("session-2", "Second terminal");
    let first_key = TabInputKey::session(first.session_id.clone());
    let second_key = TabInputKey::session(second.session_id.clone());
    let mut model = TabInputModel::default();

    assert_eq!(
        model.upsert_session(&first, "~/first"),
        TabInputChange::Added(first_key)
    );
    assert_eq!(
        model.upsert_session(&second, "~/second"),
        TabInputChange::Added(second_key.clone())
    );
    assert_eq!(model.session_count(), 2);
    assert_eq!(model.active_key(), Some(&second_key));
    assert_eq!(model.inputs()[0].session_id(), Some(&first.session_id));
    assert_eq!(model.inputs()[1].session_id(), Some(&second.session_id));
    assert!(model.inputs()[2].is_settings());
}

#[test]
fn session_upsert_updates_an_existing_input_without_replacing_its_identity() {
    let first = session("session-1", "First terminal");
    let mut model = TabInputModel::default();
    model.upsert_session(&first, "~/first");

    let mut renamed = first.clone();
    renamed.title = "First terminal renamed".to_owned();
    let key = TabInputKey::session(first.session_id.clone());

    assert_eq!(
        model.upsert_session(&renamed, "~/first"),
        TabInputChange::Updated(key.clone())
    );
    assert_eq!(model.session_count(), 1);
    assert_eq!(model.active_key(), Some(&key));
    assert_eq!(model.inputs()[0].title(), "First terminal renamed");
}

#[test]
fn catalog_upsert_does_not_change_the_active_input() {
    let active = session("session-active", "Active");
    let saved = session("session-saved", "Saved");
    let mut model = TabInputModel::default();
    model.upsert_session(&active, "~/zeta");
    let active_before_catalog = model.active_key().cloned();

    assert_eq!(
        model.upsert_catalog_session(&saved, "~/zeta"),
        TabInputChange::Added(TabInputKey::session(saved.session_id.clone()))
    );
    assert_eq!(model.active_key().cloned(), active_before_catalog);
}

#[test]
fn activation_rejects_unknown_inputs_and_updates_known_inputs() {
    let known = session("session-known", "Known");
    let unknown = SessionId::new("session-unknown").unwrap();
    let mut model = TabInputModel::default();
    model.upsert_session(&known, "~/known");

    assert!(!model.activate_session(&unknown));
    assert_eq!(model.selected_session(), Some(&known.session_id));
    assert!(model.activate_session(&known.session_id));
    assert_eq!(
        model.active_key(),
        Some(&TabInputKey::session(known.session_id.clone()))
    );
}

#[test]
fn status_updates_are_scoped_to_the_logical_input() {
    let first = session("session-1", "First");
    let second = session("session-2", "Second");
    let mut model = TabInputModel::default();
    model.upsert_session(&first, "~/first");
    model.upsert_session(&second, "~/second");

    model.update_status(&first.session_id, "Exited");

    assert_eq!(model.inputs()[0].status_label(), "Exited");
    assert_eq!(model.inputs()[1].status_label(), "Active");
}

#[test]
fn settings_is_a_first_class_input_and_preserves_the_last_session() {
    let first = session("session-1", "First");
    let mut model = TabInputModel::default();

    assert_eq!(model.inputs().len(), 1);
    assert!(model.inputs()[0].is_settings());
    assert!(model.activate_settings());
    model.upsert_session(&first, "~/first");

    assert!(model.is_settings());
    assert_eq!(model.active_key(), Some(&TabInputKey::Settings));
    assert_eq!(model.selected_session(), Some(&first.session_id));

    assert!(model.activate_last_session());
    assert_eq!(
        model.active_key(),
        Some(&TabInputKey::session(first.session_id.clone()))
    );
    assert!(!model.is_settings());
}
