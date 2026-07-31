use std::sync::atomic::{AtomicU64, Ordering};

use super::ExplorerPane;
use crate::shell_interaction::{AGENT_EXPLORER_PANE, AGENT_SIDEBAR};
use crate::shell_style::SHELL_PALETTE;
use crate::{agent_sidebar_workspace::AgentSidebarWorkspace, workspace_context::WorkspaceContext};
use zeta_ui::{Color, Component, Rect, UiScene};
use zeta_ui_dispatch::{AccessibilityRole, InteractionFrame, UiDispatch};

static NEXT_WORKSPACE_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn large_file_tree_only_paints_and_registers_visible_rows() {
    let fixture = std::env::temp_dir().join(format!(
        "zeta-virtual-file-list-{}-{}",
        std::process::id(),
        NEXT_WORKSPACE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&fixture).unwrap();
    for index in 0..50 {
        std::fs::write(
            fixture.join(format!("file-{index:03}.txt")),
            index.to_string(),
        )
        .unwrap();
    }
    let mut context = WorkspaceContext::capture_current();
    context.switch_working_directory(fixture.clone()).unwrap();
    let workspace = AgentSidebarWorkspace::new(&context);
    let pane = ExplorerPane::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 100.0),
        &workspace,
        SHELL_PALETTE,
    );
    let mut frame = InteractionFrame::default();
    let mut scene = UiScene::new(Color::WHITE);

    pane.register_interactions(&mut frame);
    pane.paint(&mut scene);

    let nodes = frame.accessibility_nodes(&UiDispatch::default());
    let list = nodes
        .iter()
        .find(|node| node.id == AGENT_EXPLORER_PANE)
        .unwrap();
    let items = nodes
        .iter()
        .filter(|node| node.role == AccessibilityRole::TreeItem)
        .collect::<Vec<_>>();
    assert_eq!(list.parent, Some(AGENT_SIDEBAR));
    assert_eq!(list.role, AccessibilityRole::Tree);
    assert_eq!(items.len(), 5);
    assert!(
        items
            .iter()
            .all(|item| item.parent == Some(AGENT_EXPLORER_PANE))
    );
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

    drop(workspace);
    std::fs::remove_dir_all(fixture).unwrap();
}

#[test]
fn expanded_directory_paints_an_indented_child_as_a_tree_item() {
    let fixture = std::env::temp_dir().join(format!(
        "zeta-file-tree-expand-{}-{}",
        std::process::id(),
        NEXT_WORKSPACE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let source = fixture.join("src");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("lib.rs"), "lib").unwrap();
    let mut context = WorkspaceContext::capture_current();
    context.switch_working_directory(fixture.clone()).unwrap();
    let mut workspace = AgentSidebarWorkspace::new(&context);
    let directory_id = workspace.root_entries()[0].element_id();

    assert!(workspace.activate_file_tree_element(directory_id));

    let pane = ExplorerPane::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 100.0),
        &workspace,
        SHELL_PALETTE,
    );
    let mut frame = InteractionFrame::default();
    let mut scene = UiScene::new(Color::WHITE);
    pane.register_interactions(&mut frame);
    pane.paint(&mut scene);
    let nodes = frame.accessibility_nodes(&UiDispatch::default());
    let child = nodes.iter().find(|node| node.label == "lib.rs").unwrap();

    assert_eq!(child.role, AccessibilityRole::TreeItem);
    assert_eq!(child.level, Some(2));
    assert!(
        scene
            .text_blocks()
            .iter()
            .any(|block| block.text() == "lib.rs")
    );

    std::fs::remove_dir_all(fixture).unwrap();
}
