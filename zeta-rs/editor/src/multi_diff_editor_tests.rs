use super::{MultiDiffEditor, MultiDiffEditorItem, MultiDiffEditorStyle};
use crate::{DiffEditorLabels, DiffEditorPresentation, DiffEditorState};
use zeta_diff::DiffDocument;
use zeta_ui::{
    Color, Component, CornerRadii, Rect, ScrollAxis, ScrollCommand, ScrollDelta, ScrollState,
    UiScene,
};

fn document(original: &str, modified: &str) -> DiffDocument {
    DiffDocument::from_text(original, modified).unwrap()
}

#[test]
fn paints_every_visible_file_as_its_own_diff_section() {
    let alpha = document("old alpha\n", "new alpha\n");
    let beta = document("old beta\n", "new beta\n");
    let items = [
        MultiDiffEditorItem::new(
            "alpha.rs",
            &alpha,
            DiffEditorState::default(),
            DiffEditorLabels::new("alpha base", "alpha working"),
        ),
        MultiDiffEditorItem::new(
            "beta.rs",
            &beta,
            DiffEditorState::default(),
            DiffEditorLabels::new("beta base", "beta working"),
        ),
    ];
    let editor = MultiDiffEditor::new(
        Rect::from_xywh(0.0, 0.0, 640.0, 568.0),
        &items,
        ScrollState::default(),
        MultiDiffEditorStyle::light(),
    );
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    assert_eq!(editor.content_height(), 176.0);
    let text = scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    assert!(text.contains(&"alpha.rs"));
    assert!(text.contains(&"alpha base"));
    assert!(text.contains(&"alpha working"));
    assert!(text.contains(&"beta.rs"));
    assert!(text.contains(&"beta base"));
    assert!(text.contains(&"beta working"));
}

#[test]
fn exposes_shared_scroll_metrics_for_the_multi_file_content() {
    let alpha = document("a", "b");
    let items = [MultiDiffEditorItem::new(
        "alpha.rs",
        &alpha,
        DiffEditorState::default(),
        DiffEditorLabels::new("base", "working"),
    )];
    let editor = MultiDiffEditor::new(
        Rect::from_xywh(0.0, 0.0, 640.0, 80.0),
        &items,
        ScrollState::default(),
        MultiDiffEditorStyle::light(),
    );

    assert_eq!(editor.scroll_metrics().viewport().height, 80.0);
    assert_eq!(editor.scroll_metrics().content().height, 84.0);
}

#[test]
fn unified_sections_use_one_readable_column_without_revision_headers() {
    let alpha = document("same\nold\n", "same\nnew\n");
    let items = [MultiDiffEditorItem::new(
        "alpha.rs",
        &alpha,
        DiffEditorState::default(),
        DiffEditorLabels::new("HEAD", "Working Tree"),
    )];
    let editor = MultiDiffEditor::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 120.0),
        &items,
        ScrollState::default(),
        MultiDiffEditorStyle::light_cards(),
    )
    .with_diff_presentation(DiffEditorPresentation::Unified);
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    let text = scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    assert_eq!(editor.content_height(), 110.0);
    assert!(text.contains(&"alpha.rs"));
    assert_eq!(text.iter().filter(|value| **value == "same").count(), 1);
    assert!(text.contains(&"old"));
    assert!(text.contains(&"new"));
    assert!(!text.contains(&"HEAD"));
    assert!(!text.contains(&"Working Tree"));
    assert!(scene.rects().iter().any(|rect| {
        rect.bounds() == Rect::from_xywh(8.0, 8.0, 304.0, 94.0)
            && rect.corner_radii() == CornerRadii::uniform(6.0)
    }));
}

#[test]
fn offscreen_sections_are_not_projected_into_the_scene() {
    let alpha = document("a", "b");
    let beta = document("c", "d");
    let items = [
        MultiDiffEditorItem::new(
            "alpha.rs",
            &alpha,
            DiffEditorState::default(),
            DiffEditorLabels::new("alpha base", "alpha working"),
        ),
        MultiDiffEditorItem::new(
            "beta.rs",
            &beta,
            DiffEditorState::default(),
            DiffEditorLabels::new("beta base", "beta working"),
        ),
    ];
    let mut state = ScrollState::default();
    let unscrolled = MultiDiffEditor::new(
        Rect::from_xywh(0.0, 0.0, 640.0, 80.0),
        &items,
        state,
        MultiDiffEditorStyle::light(),
    );
    state.apply(
        ScrollCommand::ByPixels(ScrollDelta::vertical(92.0)),
        unscrolled.scroll_metrics(),
        ScrollAxis::Vertical,
    );
    let editor = MultiDiffEditor::new(
        Rect::from_xywh(0.0, 0.0, 640.0, 80.0),
        &items,
        state,
        MultiDiffEditorStyle::light(),
    );
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    let text = scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    assert!(!text.contains(&"alpha.rs"));
    assert!(text.contains(&"beta.rs"));
}

#[test]
fn large_visible_section_only_projects_rows_inside_the_outer_viewport() {
    let original = (0..1_000)
        .map(|line| format!("old line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let document = document(&original, &original);
    let items = [MultiDiffEditorItem::new(
        "large.rs",
        &document,
        DiffEditorState::default(),
        DiffEditorLabels::new("base", "working"),
    )];
    let editor = MultiDiffEditor::new(
        Rect::from_xywh(0.0, 0.0, 640.0, 80.0),
        &items,
        ScrollState::default(),
        MultiDiffEditorStyle::light(),
    );
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    let text = scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    assert!(text.contains(&"large.rs"));
    assert!(!text.contains(&"old line 999"));
    assert!(text.len() < 20);
}

#[test]
fn exposes_visible_fold_controls_with_per_file_identity_and_card_geometry() {
    let original = (1..=20)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let modified = original.replace("line 11", "changed 11");
    let document = document(&original, &modified);
    let items = [MultiDiffEditorItem::new(
        "alpha.rs",
        &document,
        DiffEditorState::default(),
        DiffEditorLabels::new("base", "working"),
    )];
    let editor = MultiDiffEditor::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 300.0),
        &items,
        ScrollState::default(),
        MultiDiffEditorStyle::light_cards(),
    )
    .with_diff_presentation(DiffEditorPresentation::Unified);

    let controls = editor.fold_controls();

    assert_eq!(controls.len(), 2);
    assert_eq!(controls[0].item_index(), 0);
    assert_eq!(controls[0].region_index(), 0);
    assert_eq!(controls[0].line_count(), 7);
    assert_eq!(
        controls[0].bounds(),
        Rect::from_xywh(9.0, 41.0, 302.0, 20.0)
    );
}

#[test]
fn measured_layout_reuses_section_metrics_for_scrolling_and_paint() {
    let original = (1..=20)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let modified = original.replace("line 11", "changed 11");
    let document = document(&original, &modified);
    let items = [MultiDiffEditorItem::new(
        "alpha.rs",
        &document,
        DiffEditorState::default(),
        DiffEditorLabels::new("base", "working"),
    )];
    let bounds = Rect::from_xywh(0.0, 0.0, 320.0, 120.0);
    let measured = MultiDiffEditor::new(
        bounds,
        &items,
        ScrollState::default(),
        MultiDiffEditorStyle::light_cards(),
    )
    .with_diff_presentation(DiffEditorPresentation::Unified)
    .measure_layout();
    let reused = MultiDiffEditor::new(
        bounds,
        &items,
        ScrollState::default(),
        MultiDiffEditorStyle::light_cards(),
    )
    .with_diff_presentation(DiffEditorPresentation::Unified)
    .with_measured_layout(&measured);
    let mut scene = UiScene::new(Color::WHITE);

    reused.paint(&mut scene);

    assert_eq!(reused.content_height(), measured.content_height());
    assert_eq!(
        reused.scroll_metrics().content().height,
        measured.content_height()
    );
    assert_eq!(reused.fold_controls().len(), 1);
    assert!(
        scene
            .text_blocks()
            .iter()
            .any(|block| block.text() == "line 8")
    );
}
