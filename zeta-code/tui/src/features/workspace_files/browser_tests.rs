use super::FileSelectionAction;
use super::directory_view;
use super::file_preview;
use crate::components::selection::SelectionViewState;
use std::path::PathBuf;
use zeta_app_server_protocol::protocol::fs::FsFileType;
use zeta_app_server_protocol::protocol::fs::FsReadDirectoryEntry;

#[test]
fn directory_view_sorts_directories_first_and_preserves_typed_actions() {
    let view = directory_view(
        PathBuf::from("src"),
        &[
            FsReadDirectoryEntry {
                name: "lib.rs".into(),
                file_type: FsFileType::File,
            },
            FsReadDirectoryEntry {
                name: "app".into(),
                file_type: FsFileType::Directory,
            },
        ],
    );
    let state = SelectionViewState::new(view.model.into_body());

    assert_eq!(state.visible_items()[0].label(), "../");
    assert_eq!(state.visible_items()[1].label(), "app/");
    assert!(view.actions.values().any(|action| matches!(
        action,
        FileSelectionAction::PreviewFile { path } if path == &PathBuf::from("src/lib.rs")
    )));
}

#[test]
fn file_preview_is_read_only_and_bounded() {
    let model = file_preview(
        PathBuf::from("src/lib.rs"),
        &"line\n".repeat(250),
        "1234567890abcdef",
    );
    let state = SelectionViewState::new(model.into_body());

    assert_eq!(state.title(), "File · src/lib.rs");
    assert_eq!(state.selected_visible_index(), None);
    let preview = state.visible_items()[0].preview().unwrap();
    assert_eq!(
        preview.lines().last().unwrap().to_string(),
        "… preview truncated …"
    );
}
