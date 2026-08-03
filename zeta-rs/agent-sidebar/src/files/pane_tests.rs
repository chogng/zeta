use super::{EXPLORER_PANE, FilesPane, FilesPaneStyle};
use crate::{DirectoryEntry, FilesState};
use zeta_ui::{Color, Component, Rect, ScrollViewStyle, ScrollbarStyle, UiScene};
use zui::{AccessibilityRole, InteractionFrame, UiDispatch};

fn style() -> FilesPaneStyle {
    FilesPaneStyle {
        surface: Color::WHITE,
        selected_background: Color::rgb(232, 232, 232),
        hovered_background: Color::rgb(242, 242, 242),
        text: Color::rgb(38, 38, 41),
        text_muted: Color::rgb(126, 126, 132),
        scroll_view: ScrollViewStyle::new(ScrollbarStyle::new(
            Color::TRANSPARENT,
            Color::rgb(126, 126, 132),
        )),
    }
}

#[test]
fn large_file_tree_only_paints_and_registers_visible_rows() {
    let mut files = FilesState::default();
    files.refresh(
        (0..50)
            .map(|index| DirectoryEntry::file(format!("file-{index:03}.txt")))
            .collect(),
    );
    let dispatch = UiDispatch::default();
    let style = style();
    let pane = FilesPane::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 100.0),
        &files,
        zui::ElementId::scoped(1, 23),
        &style,
        &dispatch,
    );
    let mut frame = InteractionFrame::default();
    let mut scene = UiScene::new(Color::WHITE);

    pane.register_interactions(&mut frame);
    pane.paint(&mut scene);

    let nodes = frame.accessibility_nodes(&dispatch);
    let list = nodes.iter().find(|node| node.id == EXPLORER_PANE).unwrap();
    let items = nodes
        .iter()
        .filter(|node| node.role == AccessibilityRole::TreeItem)
        .collect::<Vec<_>>();
    assert_eq!(list.parent, Some(zui::ElementId::scoped(1, 23)));
    assert_eq!(list.role, AccessibilityRole::Tree);
    assert_eq!(items.len(), 5);
    assert!(items.iter().all(|item| item.parent == Some(EXPLORER_PANE)));
    assert!(
        scene
            .text_blocks()
            .iter()
            .any(|block| block.text() == "file-000.txt")
    );
    assert!(
        scene
            .text_blocks()
            .iter()
            .all(|block| block.text() != "file-049.txt")
    );
}

#[test]
fn expanded_directory_paints_an_indented_child_as_a_tree_item() {
    let mut files = FilesState::default();
    files.refresh(vec![DirectoryEntry::directory("src")]);
    let directory_id = files.tree_row(0).unwrap().entry().element_id();
    assert!(files.activate(directory_id).is_some());
    assert!(files.complete_directory_load(directory_id, vec![DirectoryEntry::file("lib.rs")]));

    let dispatch = UiDispatch::default();
    let style = style();
    let pane = FilesPane::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 100.0),
        &files,
        zui::ElementId::scoped(1, 23),
        &style,
        &dispatch,
    );
    let mut frame = InteractionFrame::default();
    let mut scene = UiScene::new(Color::WHITE);
    pane.register_interactions(&mut frame);
    pane.paint(&mut scene);
    let nodes = frame.accessibility_nodes(&dispatch);
    let child = nodes.iter().find(|node| node.label == "lib.rs").unwrap();

    assert_eq!(child.role, AccessibilityRole::TreeItem);
    assert_eq!(child.level, Some(2));
    assert!(
        scene
            .text_blocks()
            .iter()
            .any(|block| block.text() == "lib.rs")
    );
}
