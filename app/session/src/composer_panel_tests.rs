use super::draw_info_bar;
use crate::ComposerRoute;
use crate::SessionPaneStyle;
use crate::interaction::COMPOSER_INFO_BAR;
use zui::ui::{Color, Rect};
use zui::ui::{InteractionFrame, UiDispatch, UiFrame};

const STYLE: SessionPaneStyle = SessionPaneStyle::new(
    Color::WHITE,
    Color::rgb(246, 246, 247),
    Color::rgb(248, 248, 249),
    Color::rgb(222, 222, 224),
    Color::rgb(38, 38, 41),
    Color::rgb(126, 126, 132),
    Color::rgb(15, 110, 96),
    Color::rgb(16, 124, 16),
    Color::rgb(154, 103, 0),
    Color::rgb(180, 38, 38),
    Color::rgb(235, 235, 237),
    zeta_ui_components::ScrollViewStyle::new(zeta_ui_components::ScrollbarStyle::new(
        Color::TRANSPARENT,
        Color::rgba(100, 100, 100, 51),
    )),
);

#[test]
fn info_bar_paints_agent_and_shell_triggers_as_keycaps() {
    let bounds = Rect::from_xywh(10.0, 20.0, 400.0, 24.0);

    for (route, keys, label, accessibility_label) in [
        (
            ComposerRoute::Agent,
            &["/"][..],
            "for commands",
            "/ for commands",
        ),
        (
            ComposerRoute::Shell,
            &["↑", "↓"][..],
            "for command history",
            "Up and Down for command history",
        ),
    ] {
        let mut frame = UiFrame::<InteractionFrame>::new(Color::WHITE);

        frame.with_context(|context| draw_info_bar(context, bounds, route, STYLE));
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
            .target_at(zui::ui::Point::new(
                bounds.origin.x + 100.0,
                bounds.origin.y + 12.0,
            ))
            .expect("the full info bar should be inspectable outside its keycap");
        assert_eq!(inspected_label.name(), "ComposerInfoBar");
    }
}
