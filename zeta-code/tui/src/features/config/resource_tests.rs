use super::ConfigResource;
use super::TerminalSettingsEdit;

#[test]
fn mouse_interaction_edit_is_persisted_and_reloaded() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("terminal.json");
    let mut resource = ConfigResource::new(path.clone());
    assert!(resource.refresh().unwrap().mouse_interactions());

    let (settings, _) = resource
        .apply_edit(&TerminalSettingsEdit {
            expected_revision: 1,
            mouse_interactions: false,
        })
        .unwrap();
    assert!(!settings.mouse_interactions());

    let mut reloaded = ConfigResource::new(path);
    assert!(!reloaded.refresh().unwrap().mouse_interactions());
}
