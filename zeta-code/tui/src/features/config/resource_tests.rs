use super::ConfigResource;
use super::TerminalSettingsEdit;
use crate::features::config::TerminalSettings;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryPermissionDto;

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
fn additional_directory_defaults_are_persisted_and_reloaded() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("terminal.json");
    let mut resource = ConfigResource::new(path.clone());
    resource.refresh().unwrap();
    let permissions = vec![
        WorkspaceAdditionalDirectoryPermissionDto::ReadFiles,
        WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceSearch,
    ];
    let mut settings = TerminalSettings::default();
    settings.set_additional_directory_permissions(&permissions);

    resource
        .apply_edit(&TerminalSettingsEdit {
            expected_revision: 1,
            settings,
        })
        .unwrap();

    let mut reloaded = ConfigResource::new(path);
    assert_eq!(
        reloaded
            .refresh()
            .unwrap()
            .additional_directory_permissions(),
        permissions
    );
}
