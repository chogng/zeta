use super::{ComposerPanelLayout, draw_info_bar};
use crate::agent_composer::ComposerMode;
use crate::shell_interaction::COMPOSER_INFO_BAR;
use crate::shell_style::SHELL_PALETTE;
use zeta_ui::{Color, Rect};
use zui::{InteractionFrame, UiDispatch, UiFrame};

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

#[test]
fn info_bar_paints_agent_and_shell_triggers_as_keycaps() {
    let bounds = Rect::from_xywh(10.0, 20.0, 400.0, 24.0);

    for (mode, keys, label, accessibility_label) in [
        (
            ComposerMode::Agent,
            &["/"][..],
            "for commands",
            "/ for commands",
        ),
        (
            ComposerMode::Shell,
            &["↑", "↓"][..],
            "for command history",
            "Up and Down for command history",
        ),
    ] {
        let mut frame = UiFrame::<InteractionFrame>::new(Color::WHITE);

        frame.with_context(|context| draw_info_bar(context, bounds, mode, SHELL_PALETTE));
        let scene = frame.scene();

        assert_eq!(scene.rects().len(), keys.len());
        for keycap in scene.rects() {
            assert_eq!(keycap.bounds().size.width, 16.0);
            assert_eq!(keycap.bounds().size.height, 16.0);
            assert_eq!(keycap.fill(), Color::rgb(96, 97, 102));
        }
        for key in keys {
            let keycap_label = scene
                .text_blocks()
                .iter()
                .find(|text| text.text() == *key)
                .unwrap();
            assert_eq!(keycap_label.style().color(), Color::WHITE);
        }
        assert!(scene.text_blocks().iter().any(|text| text.text() == label));
        assert!(
            !scene
                .text_blocks()
                .iter()
                .any(|text| text.text() == accessibility_label)
        );
        let accessibility_nodes = frame
            .interaction()
            .accessibility_nodes(&UiDispatch::default());
        assert_eq!(
            accessibility_nodes
                .iter()
                .find(|node| node.id == COMPOSER_INFO_BAR)
                .unwrap()
                .label,
            accessibility_label
        );
        let inspected_label = scene
            .inspection()
            .target_at(zeta_ui::Point::new(
                bounds.origin.x + 100.0,
                bounds.origin.y + 12.0,
            ))
            .expect("the full info bar should be inspectable outside its keycap");
        assert_eq!(inspected_label.name(), "ComposerInfoBar");
    }
}
