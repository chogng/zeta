use zeta_editor::{CodeEditorDiagnostic, CodeEditorDiagnosticSeverity};
use zeta_ui::{Color, Component, Point, Rect, UiScene};

use super::*;
use crate::shell_style::SHELL_PALETTE;

#[test]
fn tooltip_clamps_to_the_editor_and_labels_source_and_code() {
    let diagnostic = CodeEditorDiagnostic::new(
        4..8,
        CodeEditorDiagnosticSeverity::Error,
        "cannot find value",
    )
    .with_source("rustc")
    .with_code("E0425");
    let editor = Rect::from_xywh(10.0, 20.0, 400.0, 200.0);
    let tooltip = FileEditorDiagnosticTooltip::new(
        editor,
        Point::new(405.0, 215.0),
        &diagnostic,
        SHELL_PALETTE,
    );
    let mut scene = UiScene::new(Color::WHITE);

    tooltip.paint(&mut scene);

    assert!(tooltip.bounds().origin.x >= editor.origin.x);
    assert!(tooltip.bounds().right() <= editor.right());
    assert!(tooltip.bounds().origin.y >= editor.origin.y);
    assert!(tooltip.bounds().bottom() <= editor.bottom());
    assert_eq!(
        scene.text_blocks()[0].text(),
        "rustc(E0425): cannot find value"
    );
}
