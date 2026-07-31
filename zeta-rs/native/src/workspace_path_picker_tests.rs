use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{
    WorkspacePathPicker, WorkspacePathPickerAction, WorkspacePathPickerActivation,
    WorkspacePathPickerState, workspace_path_item_id,
};
use crate::shell_interaction::COMPOSER;
use crate::shell_style::SHELL_PALETTE;
use zeta_ui::{CaretVisibility, Point, Rect, TextInputCommand, TextInputLayoutEngine};
use zeta_ui_dispatch::{AccessibilityRole, InteractionFrame, UiDispatch};

static NEXT_PICKER_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn picker_lists_current_parent_and_sorted_child_directories() {
    let root = picker_fixture();
    std::fs::create_dir_all(root.join("zulu")).unwrap();
    std::fs::create_dir_all(root.join("Alpha")).unwrap();
    std::fs::write(root.join("ignored.txt"), "file").unwrap();
    let mut state = WorkspacePathPickerState::default();

    state
        .open(
            Rect::from_xywh(40.0, 640.0, 180.0, 24.0),
            &root,
            Some(COMPOSER),
        )
        .unwrap();

    let items = state.items();
    assert!(items[0].label.starts_with("Use this folder · "));
    assert!(matches!(
        &items[0].action,
        Some(WorkspacePathPickerAction::SelectCurrent)
    ));
    assert!(items[1].label.starts_with("↑ Parent · "));
    assert_eq!(items[2].label, "› Alpha/");
    assert_eq!(items[3].label, "› zulu/");
    assert_eq!(state.dismiss(), Some(COMPOSER));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn picker_browses_directories_and_pages_all_children() {
    let root = picker_fixture();
    for index in 0..10 {
        std::fs::create_dir_all(root.join(format!("folder-{index:02}"))).unwrap();
    }
    let mut state = WorkspacePathPickerState::default();
    state
        .open(Rect::from_xywh(40.0, 640.0, 180.0, 24.0), &root, None)
        .unwrap();
    let more_index = state.items().len() - 1;

    assert_eq!(
        state.activate(more_index).unwrap(),
        Some(WorkspacePathPickerActivation::BrowseChanged)
    );
    assert!(
        state
            .items()
            .iter()
            .any(|item| item.label == "← Previous folders")
    );
    let child_index = state
        .items()
        .iter()
        .position(|item| item.label == "› folder-08/")
        .unwrap();
    assert_eq!(
        state.activate(child_index).unwrap(),
        Some(WorkspacePathPickerActivation::BrowseChanged)
    );
    assert_eq!(
        state.open.as_ref().unwrap().directory,
        root.join("folder-08").canonicalize().unwrap()
    );
    assert_eq!(
        state.activate(0).unwrap(),
        Some(WorkspacePathPickerActivation::SelectWorkspace(
            root.join("folder-08").canonicalize().unwrap()
        ))
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn picker_is_anchored_above_the_toolbar_and_registers_modal_menu_semantics() {
    let root = picker_fixture();
    std::fs::create_dir_all(root.join("child")).unwrap();
    let anchor = Rect::from_xywh(40.0, 640.0, 180.0, 24.0);
    let mut state = WorkspacePathPickerState::default();
    state.open(anchor, &root, None).unwrap();
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let picker = WorkspacePathPicker::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &state,
        CaretVisibility::Visible,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    )
    .unwrap();
    let mut frame = InteractionFrame::default();

    picker.register_interactions(&mut frame);

    assert!(picker.bounds().bottom() <= anchor.origin.y);
    let nodes = frame.accessibility_nodes(&dispatch);
    assert_eq!(nodes[0].role, AccessibilityRole::Menu);
    assert_eq!(nodes[1].role, AccessibilityRole::TextInput);
    assert_eq!(nodes[2].role, AccessibilityRole::MenuItem);
    assert_eq!(frame.target_at(Point::new(42.0, 642.0)), None);
    assert_eq!(state.first_action_id(), Some(workspace_path_item_id(0)));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn search_row_filters_child_directories_and_resets_after_browsing() {
    let root = picker_fixture();
    std::fs::create_dir_all(root.join("alpha")).unwrap();
    std::fs::create_dir_all(root.join("beta")).unwrap();
    let mut state = WorkspacePathPickerState::default();
    state
        .open(Rect::from_xywh(40.0, 640.0, 180.0, 24.0), &root, None)
        .unwrap();

    state.apply_search(TextInputCommand::Insert("bet".to_string()));

    let items = state.items();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].label, "› beta/");
    assert_eq!(
        state.activate(0).unwrap(),
        Some(WorkspacePathPickerActivation::BrowseChanged)
    );
    assert!(state.search_input().text().is_empty());
    assert!(
        state
            .items()
            .first()
            .is_some_and(|item| item.label.starts_with("Use this folder · "))
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn picker_fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "zeta-workspace-path-picker-{}-{}",
        std::process::id(),
        NEXT_PICKER_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}
