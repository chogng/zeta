use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{
    GitBranchContextMenu, GitBranchContextMenuState, GitBranchMenuAction, GitBranchMenuActivation,
};
use crate::shell_interaction::COMPOSER;
use crate::shell_style::SHELL_PALETTE;
use crate::workspace_context::WorkspaceContext;
use zeta_ui::{CaretVisibility, Rect, TextInputCommand, TextInputLayoutEngine};
use zeta_ui_dispatch::{AccessibilityRole, InteractionFrame, UiDispatch};

static NEXT_MENU_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn branch_menu_places_the_current_branch_first_and_marks_it() {
    let root = repository_fixture();
    run_git(&root, &["branch", "zulu"]);
    run_git(&root, &["branch", "alpha"]);
    let mut context = WorkspaceContext::capture_current();
    context.switch_working_directory(root.clone()).unwrap();
    let mut state = GitBranchContextMenuState::default();

    state.open(
        Rect::from_xywh(240.0, 640.0, 90.0, 24.0),
        context.local_branches().unwrap(),
        Some(COMPOSER),
    );

    let items = state.items();
    assert_eq!(items[0].label, "✓ main");
    assert_eq!(items[1].label, "alpha");
    assert_eq!(items[2].label, "zulu");
    assert!(matches!(
        items[0].action,
        Some(GitBranchMenuAction::Select(_))
    ));
    assert_eq!(state.dismiss(), Some(COMPOSER));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn branch_menu_reuses_context_menu_geometry_and_modal_semantics() {
    let root = repository_fixture();
    run_git(&root, &["branch", "topic"]);
    let mut context = WorkspaceContext::capture_current();
    context.switch_working_directory(root.clone()).unwrap();
    let anchor = Rect::from_xywh(240.0, 640.0, 90.0, 24.0);
    let mut state = GitBranchContextMenuState::default();
    state.open(anchor, context.local_branches().unwrap(), None);
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let menu = GitBranchContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &state,
        CaretVisibility::Visible,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    )
    .unwrap();
    let mut frame = InteractionFrame::default();

    menu.register_interactions(&mut frame);

    assert!(menu.bounds().bottom() <= anchor.origin.y);
    let nodes = frame.accessibility_nodes(&dispatch);
    assert_eq!(nodes[0].role, AccessibilityRole::Menu);
    assert_eq!(nodes[1].role, AccessibilityRole::TextInput);
    assert_eq!(nodes[2].role, AccessibilityRole::MenuItem);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn search_row_filters_branches_and_resets_to_the_first_page() {
    let root = repository_fixture();
    run_git(&root, &["branch", "feature/search"]);
    run_git(&root, &["branch", "topic"]);
    let mut context = WorkspaceContext::capture_current();
    context.switch_working_directory(root.clone()).unwrap();
    let mut state = GitBranchContextMenuState::default();
    state.open(
        Rect::from_xywh(240.0, 640.0, 90.0, 24.0),
        context.local_branches().unwrap(),
        None,
    );

    state.apply_search(TextInputCommand::Insert("search".to_string()));

    assert_eq!(state.search_input().text(), "search");
    assert_eq!(state.items().len(), 1);
    assert_eq!(state.items()[0].label, "feature/search");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn branch_menu_pages_large_branch_lists_and_surfaces_switch_errors() {
    let root = repository_fixture();
    for index in 0..12 {
        run_git(&root, &["branch", &format!("topic-{index:02}")]);
    }
    let mut context = WorkspaceContext::capture_current();
    context.switch_working_directory(root.clone()).unwrap();
    let mut state = GitBranchContextMenuState::default();
    state.open(
        Rect::from_xywh(240.0, 640.0, 90.0, 24.0),
        context.local_branches().unwrap(),
        None,
    );
    let more_index = state.items().len() - 1;

    assert_eq!(
        state.activate(more_index),
        Some(GitBranchMenuActivation::PageChanged)
    );
    assert!(
        state
            .items()
            .iter()
            .any(|item| item.label == "← Previous branches")
    );
    state.set_switch_error();
    assert_eq!(
        state.items()[0].label,
        "Switch failed · working tree unchanged"
    );
    assert!(state.items()[0].action.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

fn repository_fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "zeta-git-branch-menu-{}-{}",
        std::process::id(),
        NEXT_MENU_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    run_git(&root, &["init", "--initial-branch=main"]);
    run_git(&root, &["config", "user.name", "Zeta Test"]);
    run_git(&root, &["config", "user.email", "zeta@example.invalid"]);
    std::fs::write(root.join("tracked.txt"), "main\n").unwrap();
    run_git(&root, &["add", "tracked.txt"]);
    run_git(&root, &["commit", "-m", "initial"]);
    root
}

fn run_git(root: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
