use std::time::Instant;

use super::{EditorPane, EditorPaneState};
use crate::shell_interaction::{AGENT_EDITOR_PANE, MULTI_DIFF_EDITOR, MULTI_DIFF_SCROLLBAR};
use crate::workspace_panes::TEST_SCM_PANE_STYLE;
use zeta_diff::DiffDocument;
use zeta_ui::{Color, Component, Rect, UiScene};
use zui::ui::{AccessibilityRole, InteractionFrame, UiDispatch, UiFrame};

fn document(original: &str, modified: &str) -> DiffDocument {
    DiffDocument::from_text(original, modified).unwrap()
}

#[test]
fn changed_files_retain_independent_diff_viewports_in_one_multi_diff_state() {
    let mut state = EditorPaneState::default();
    state.open_diff(
        "alpha.rs",
        "alpha.rs (base)",
        "alpha.rs (working)",
        document("old alpha\n", "new alpha\n"),
    );
    let second_text = (0..24)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    state.open_diff(
        "beta.rs",
        "beta.rs (base)",
        "beta.rs (working)",
        document("", &second_text),
    );

    state
        .diff_mut(1)
        .unwrap()
        .editor_state_mut()
        .scroll_rows(8, 24, 6);
    assert!(state.scroll(120.0, zeta_ui::Size::new(320.0, 300.0), Instant::now(),));

    assert_eq!(state.diffs.len(), 2);
    assert_eq!(state.diffs[0].editor_state.first_visible_row(), 0);
    assert_eq!(state.diffs[1].editor_state.first_visible_row(), 8);
    assert_eq!(state.scroll_state.vertical_offset(), 120.0);
}

#[test]
fn changed_file_identity_and_viewport_state_survive_snapshot_reordering() {
    let mut state = EditorPaneState::default();
    state.replace_test_diffs(vec![
        (
            "alpha.rs".to_string(),
            document("old alpha\n", "new alpha\n"),
        ),
        ("beta.rs".to_string(), document("old beta\n", "new beta\n")),
    ]);
    let alpha_identity = state.diffs[0].identity;
    let beta_identity = state.diffs[1].identity;
    state
        .diff_mut(1)
        .unwrap()
        .editor_state_mut()
        .scroll_rows(8, 24, 6);

    state.replace_test_diffs(vec![
        (
            "beta.rs".to_string(),
            document("old beta refreshed\n", "new beta refreshed\n"),
        ),
        (
            "alpha.rs".to_string(),
            document("old alpha refreshed\n", "new alpha refreshed\n"),
        ),
    ]);

    assert_eq!(state.diffs[0].identity, beta_identity);
    assert_eq!(state.diffs[1].identity, alpha_identity);
    assert_eq!(state.diffs[0].editor_state.first_visible_row(), 8);
    assert_eq!(state.diffs[1].editor_state.first_visible_row(), 0);
}

#[test]
fn removed_changed_files_report_their_identity_for_animation_cleanup() {
    let mut state = EditorPaneState::default();
    state.replace_test_diffs(vec![
        (
            "alpha.rs".to_string(),
            document("old alpha\n", "new alpha\n"),
        ),
        ("beta.rs".to_string(), document("old beta\n", "new beta\n")),
    ]);
    let beta_identity = state.diffs[1].identity;

    let removed = state.replace_test_diffs(vec![(
        "alpha.rs".to_string(),
        document("old alpha refreshed\n", "new alpha refreshed\n"),
    )]);

    assert_eq!(removed, vec![beta_identity]);
    assert_eq!(state.diffs.len(), 1);
    assert_eq!(state.diffs[0].identity.slot(), 1);
}

#[test]
fn scrollbar_thumb_drag_and_track_click_update_the_shared_scroll_state() {
    let mut state = EditorPaneState::default();
    let changed = (0..80)
        .map(|line| format!("changed {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    state.open_diff(
        "long.rs",
        "long.rs (base)",
        "long.rs (working)",
        document("", &changed),
    );
    let bounds = Rect::from_xywh(0.0, 0.0, 320.0, 240.0);
    let scrollbar = state
        .scroll_view(bounds)
        .vertical_scrollbar()
        .expect("long diff should overflow");
    let thumb = scrollbar.thumb_bounds();
    let now = Instant::now();
    let press = state.press_scrollbar(
        zeta_ui::Point::new(thumb.origin.x + 2.0, thumb.origin.y + 2.0),
        bounds,
        now,
    );
    assert!(press.handled);

    let drag = state.scrollbar_pointer_moved(
        zeta_ui::Point::new(
            thumb.origin.x + 2.0,
            scrollbar.track_bounds().bottom() - 2.0,
        ),
        bounds,
        now,
    );
    assert!(drag.handled);
    assert!(state.scroll_state.vertical_offset() > 0.0);
    assert!(
        state
            .release_scrollbar(zeta_ui::Point::new(-1.0, -1.0), bounds, now)
            .handled
    );

    let previous = state.scroll_state.vertical_offset();
    let track_point = zeta_ui::Point::new(
        scrollbar.track_bounds().origin.x + 2.0,
        scrollbar.track_bounds().origin.y + 1.0,
    );
    assert!(state.press_scrollbar(track_point, bounds, now).handled);
    assert!(state.scroll_state.vertical_offset() < previous);
}

#[test]
fn overflowing_editor_registers_an_accessible_scrollbar_region() {
    let mut state = EditorPaneState::default();
    let changed = (0..80)
        .map(|line| format!("changed {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    state.open_diff("long.rs", "base", "working", document("", &changed));
    let pane = EditorPane::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 240.0),
        &state,
        TEST_SCM_PANE_STYLE,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(Color::WHITE);
    frame.draw_component(&pane);

    let nodes = frame
        .interaction()
        .accessibility_nodes(&UiDispatch::default());
    let scrollbar = nodes
        .iter()
        .find(|node| node.id == MULTI_DIFF_SCROLLBAR)
        .unwrap();
    assert_eq!(scrollbar.parent, Some(MULTI_DIFF_EDITOR));
    assert_eq!(scrollbar.role, AccessibilityRole::ScrollBar);
}

#[test]
fn editor_pane_paints_all_visible_file_diffs_as_unified_sections_without_tab_selection() {
    let mut state = EditorPaneState::default();
    state.open_diff(
        "alpha.rs",
        "alpha base",
        "alpha working",
        document("old alpha\n", "new alpha\n"),
    );
    state.open_diff(
        "beta.rs",
        "beta base",
        "beta working",
        document("old beta\n", "new beta\n"),
    );
    let pane = EditorPane::new(
        Rect::from_xywh(680.0, 212.0, 320.0, 488.0),
        &state,
        TEST_SCM_PANE_STYLE,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(Color::WHITE);
    frame.draw_component(&pane);
    let scene = frame.scene();

    let visible_text = scene
        .text_blocks()
        .iter()
        .map(|text| text.text())
        .collect::<Vec<_>>();
    assert!(visible_text.contains(&"alpha.rs"));
    assert!(visible_text.contains(&"beta.rs"));
    assert!(visible_text.contains(&"old alpha"));
    assert!(visible_text.contains(&"new alpha"));
    assert!(visible_text.contains(&"old beta"));
    assert!(visible_text.contains(&"new beta"));
    assert!(!visible_text.contains(&"alpha base"));
    assert!(!visible_text.contains(&"alpha working"));

    let dispatch = UiDispatch::default();
    let nodes = frame.interaction().accessibility_nodes(&dispatch);
    let multi_diff = nodes
        .iter()
        .find(|node| node.id == MULTI_DIFF_EDITOR)
        .unwrap();
    assert_eq!(multi_diff.parent, Some(AGENT_EDITOR_PANE));
    assert_eq!(multi_diff.role, AccessibilityRole::Group);
    assert_eq!(multi_diff.label, "Multiple file differences");
    assert!(
        nodes
            .iter()
            .all(|node| node.role != AccessibilityRole::TabList)
    );
}

#[test]
fn editor_pane_exposes_nested_component_inspection_nodes() {
    let original = (1..=80)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let modified = format!(
        "{}\n{}",
        original.replace("line 41", "changed 41"),
        (81..=120)
            .map(|line| format!("added {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let mut state = EditorPaneState::default();
    state.open_diff(
        "alpha.rs",
        "alpha base",
        "alpha working",
        document(&original, &modified),
    );
    let pane = EditorPane::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 300.0),
        &state,
        TEST_SCM_PANE_STYLE,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(Color::WHITE);
    frame.draw_component(&pane);

    let inspection = frame.scene().inspection();
    let names = inspection
        .nodes()
        .iter()
        .map(|node| node.name())
        .collect::<Vec<_>>();
    for name in [
        "EditorPane",
        "MultiDiffEditor",
        "ScrollView",
        "MultiDiffSection",
        "MultiDiffFileHeader",
        "DiffEditor",
        "CodeEditor",
        "MultiDiffFoldControl",
        "MultiDiffScrollbar",
    ] {
        assert!(
            names.contains(&name),
            "missing inspection node {name}; found {names:?}"
        );
    }

    let code_editor = inspection
        .nodes()
        .iter()
        .find(|node| node.name() == "CodeEditor")
        .expect("code editor inspection node");
    let ancestry = inspection
        .ancestry(code_editor.id())
        .into_iter()
        .map(|node| node.name())
        .collect::<Vec<_>>();
    assert_eq!(
        ancestry,
        vec![
            "EditorPane",
            "MultiDiffEditor",
            "ScrollView",
            "MultiDiffSection",
            "DiffEditor",
            "CodeEditor",
        ]
    );
}

#[test]
fn empty_editor_pane_exposes_an_honest_empty_state() {
    let state = EditorPaneState::default();
    let pane = EditorPane::new(
        Rect::from_xywh(0.0, 100.0, 320.0, 300.0),
        &state,
        TEST_SCM_PANE_STYLE,
    );
    let mut scene = UiScene::new(Color::WHITE);

    pane.paint(&mut scene);

    assert!(
        scene
            .text_blocks()
            .iter()
            .any(|text| text.text() == "No changed files")
    );
}

#[test]
fn unchanged_region_controls_are_accessible_and_toggle_retained_diff_state() {
    let original = (1..=20)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let modified = original.replace("line 11", "changed 11");
    let mut state = EditorPaneState::default();
    state.open_diff(
        "alpha.rs",
        "alpha base",
        "alpha working",
        document(&original, &modified),
    );
    let mut frame = UiFrame::<InteractionFrame>::new(Color::WHITE);
    let pane = EditorPane::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 300.0),
        &state,
        TEST_SCM_PANE_STYLE,
    );
    frame.draw_component(&pane);
    let nodes = frame
        .interaction()
        .accessibility_nodes(&UiDispatch::default());
    let fold = nodes
        .iter()
        .find(|node| node.label == "Show 7 unchanged lines in alpha.rs")
        .unwrap();
    let section = nodes
        .iter()
        .find(|node| node.label == "Changed file alpha.rs")
        .unwrap();

    assert_eq!(fold.parent, Some(section.id));
    assert_eq!(fold.role, AccessibilityRole::Button);
    assert!(state.toggle_fold_for_element(fold.id));
    assert!(state.diffs[0].editor_state.is_unchanged_region_expanded(0));

    let mut expanded_frame = UiFrame::<InteractionFrame>::new(Color::WHITE);
    let expanded_pane = EditorPane::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 480.0),
        &state,
        TEST_SCM_PANE_STYLE,
    );
    expanded_frame.draw_component(&expanded_pane);
    assert!(
        expanded_frame
            .interaction()
            .accessibility_nodes(&UiDispatch::default())
            .iter()
            .any(|node| node.label == "Hide 7 unchanged lines in alpha.rs")
    );
}
