use super::StatusLineEdit;
use super::StatusLineResource;
use crate::features::status_line::StatusLineItem;

#[test]
fn edit_is_persisted_and_reloaded() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("statusline.json");
    let mut resource = StatusLineResource::new(path.clone());
    let settings = resource.refresh().unwrap();
    assert!(settings.enabled(StatusLineItem::Model));

    let (settings, _) = resource
        .apply_edit(&StatusLineEdit {
            expected_revision: 1,
            item: StatusLineItem::Model,
            enabled: false,
        })
        .unwrap();
    assert!(!settings.enabled(StatusLineItem::Model));

    let mut reloaded = StatusLineResource::new(path);
    assert!(!reloaded.refresh().unwrap().enabled(StatusLineItem::Model));
}
