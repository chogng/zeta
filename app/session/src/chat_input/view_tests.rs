use super::draw_info_bar;
use crate::SessionPaneStyle;
use crate::interaction::COMPOSER_INFO_BAR;
use zeta_ui_theme::DEFAULT_UI_THEME;
use zui::ui::{Color, Rect};
use zui::ui::{InteractionFrame, UiDispatch, UiFrame};

use super::super::ComposerRoute;

#[test]
fn chat_input_info_bar_paints_agent_and_shell_triggers_as_keycaps() {
    let style = SessionPaneStyle::from_theme(DEFAULT_UI_THEME);
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

        frame.with_context(|context| draw_info_bar(context, bounds, route, style));
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
