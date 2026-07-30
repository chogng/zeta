use super::{MultiDiffEditor, MultiDiffEditorItem, MultiDiffEditorStyle};
use crate::{DiffEditorLabels, DiffEditorState};
use zeta_diff::DiffDocument;
use zeta_ui::{
    Color, Component, Rect, ScrollAxis, ScrollCommand, ScrollDelta, ScrollState, UiScene,
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
