use super::ConfigResource;
use super::TerminalSettingsEdit;
use crate::features::config::FollowUpMode;
use crate::features::config::TerminalSettings;
use std::fs;
use zeta_app_server_protocol::protocol::environment::PermissionDto;

#[test]
fn mouse_interaction_edit_is_persisted_and_reloaded() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("terminal.json");
    let mut resource = ConfigResource::new(path.clone());
    assert!(resource.refresh().unwrap().mouse_interactions());

    let (settings, _) = resource
        .apply_edit(&TerminalSettingsEdit {
            expected_revision: 1,
            settings: {
                let mut settings = TerminalSettings::default();
                settings.set_mouse_interactions(false);
                settings
            },
        })
        .unwrap();
    assert!(!settings.mouse_interactions());

    let mut reloaded = ConfigResource::new(path);
    assert!(!reloaded.refresh().unwrap().mouse_interactions());
}

#[test]
fn follow_up_mode_defaults_to_queue_and_is_persisted() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("terminal.json");
    let mut resource = ConfigResource::new(path.clone());
    assert_eq!(
        resource.refresh().unwrap().follow_up_mode(),
        FollowUpMode::Queue
    );

    let mut settings = TerminalSettings::default();
    settings.set_follow_up_mode(FollowUpMode::Steer);
    resource
        .apply_edit(&TerminalSettingsEdit {
            expected_revision: 1,
            settings,
        })
        .unwrap();

    let mut reloaded = ConfigResource::new(path);
    assert_eq!(
        reloaded.refresh().unwrap().follow_up_mode(),
        FollowUpMode::Steer
    );
}

#[test]
fn existing_terminal_settings_without_follow_up_mode_load_as_queue() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("terminal.json");
    fs::write(
        &path,
        r#"{
  "mouseInteractions": false,
  "dirPermissions": {}
}"#,
    )
    .unwrap();

    let mut resource = ConfigResource::new(path);
    let settings = resource.refresh().unwrap();

    assert!(!settings.mouse_interactions());
    assert_eq!(settings.follow_up_mode(), FollowUpMode::Queue);
}

#[test]
fn dir_defaults_are_persisted_and_reloaded() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("terminal.json");
    let mut resource = ConfigResource::new(path.clone());
    resource.refresh().unwrap();
    let permissions = vec![PermissionDto::ReadFiles, PermissionDto::SearchFiles];
    let mut settings = TerminalSettings::default();
    settings.set_dir_permissions(&permissions);

    resource
        .apply_edit(&TerminalSettingsEdit {
            expected_revision: 1,
            settings,
        })
        .unwrap();

    let mut reloaded = ConfigResource::new(path);
    assert_eq!(reloaded.refresh().unwrap().dir_permissions(), permissions);
}
