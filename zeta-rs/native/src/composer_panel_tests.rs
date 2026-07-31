use super::ComposerPanelLayout;
use zeta_ui::Rect;

#[test]
fn interaction_expands_panel_upward_and_preserves_fixed_composer_rows() {
    let main = Rect::from_xywh(0.0, 0.0, 800.0, 600.0);
    let closed = ComposerPanelLayout::for_main(main, 44.0, 0.0);
    let open = ComposerPanelLayout::for_main(main, 44.0, 200.0);

    assert_eq!(closed.panel().bottom(), main.bottom());
    assert_eq!(open.panel().bottom(), main.bottom());
    assert!(open.panel().origin.y < closed.panel().origin.y);
    assert_eq!(open.info_bar(), closed.info_bar());
    assert_eq!(open.editor(), closed.editor());
    assert_eq!(open.toolbar(), closed.toolbar());
    assert!(open.output().size.height < closed.output().size.height);
}

#[test]
fn fixed_rows_place_info_above_editor_and_toolbar_at_the_bottom() {
    let main = Rect::from_xywh(0.0, 0.0, 800.0, 600.0);
    let layout = ComposerPanelLayout::for_main(main, 44.0, 0.0);

    assert!(layout.info_bar().bottom() < layout.editor().origin.y);
    assert_eq!(
        layout.info_editor_separator().bottom(),
        layout.editor().origin.y
    );
    assert_eq!(
        layout.info_editor_separator().size.width,
        layout.panel().size.width
    );
    assert!(layout.editor().bottom() < layout.toolbar().origin.y);
    assert!(layout.toolbar().bottom() < layout.panel().bottom());
}

#[test]
fn interaction_height_keeps_a_minimum_output_surface() {
    let main = Rect::from_xywh(0.0, 0.0, 800.0, 180.0);
    let layout = ComposerPanelLayout::for_main(main, 44.0, 500.0);

    assert!(layout.output().size.height >= 40.0);
    assert!(layout.interaction().is_some());
}
