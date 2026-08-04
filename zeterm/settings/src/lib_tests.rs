use zeta_icons::icons;
use zeta_ui::ActionBarStyle;
use zeta_ui::ButtonBackgrounds;
use zeta_ui::ButtonStyle;
use zeta_ui::CaretVisibility;
use zeta_ui::Color;
use zeta_ui::InputBoxStateColors;
use zeta_ui::InputBoxStyle;
use zeta_ui::SearchBoxStyle;
use zeta_ui::TextInputLayoutEngine;
use zeta_ui::TextStyle;
use zui::AccessibilityRole;
use zui::InteractionFrame;
use zui::UiDispatch;
use zui::UiFrame;

use super::{
    SETTINGS_CLOSE, SETTINGS_NAV_BACK, SETTINGS_NAV_LANGUAGE_SERVERS, SETTINGS_PAGE,
    SETTINGS_RESET, SETTINGS_SAVE, SETTINGS_SEARCH_INPUT, SettingsPage,
    SettingsPageActionAvailability, SettingsPageLayout, SettingsPageStyle,
};

fn style() -> SettingsPageStyle {
    let border = Color::rgb(222, 222, 224);
    let text = Color::rgb(38, 38, 41);
    let text_muted = Color::rgb(126, 126, 132);
    let input = InputBoxStyle::new(
        InputBoxStateColors::new(Color::WHITE, Color::WHITE, Color::WHITE),
        InputBoxStateColors::new(border, border, Color::rgb(15, 110, 96)),
        TextStyle::new(13.0, text).with_line_height(18.0),
        TextStyle::new(13.0, text_muted).with_line_height(18.0),
    );
    let nav = ButtonStyle::new(
        ButtonBackgrounds::new(Color::TRANSPARENT),
        TextStyle::new(13.0, text).with_line_height(18.0),
    );
    let close = ButtonStyle::new(
        ButtonBackgrounds::new(Color::TRANSPARENT),
        TextStyle::new(13.0, text).with_line_height(18.0),
    );
    let action = ButtonStyle::new(
        ButtonBackgrounds::new(Color::TRANSPARENT),
        TextStyle::new(13.0, text).with_line_height(18.0),
    );
    SettingsPageStyle::new(
        Color::rgb(252, 252, 253),
        Color::rgb(246, 246, 247),
        Color::WHITE,
        Color::rgb(246, 246, 247),
        border,
        text,
        Color::rgb(15, 110, 96),
        SearchBoxStyle::new(input, icons::SEARCH, text_muted),
        nav,
        close,
        ActionBarStyle::new(action, zeta_ui::Size::new(100.0, 32.0)),
    )
}

#[test]
fn layout_keeps_rail_header_action_bar_and_content_separate() {
    let layout =
        SettingsPageLayout::for_viewport(zeta_ui::Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0));
    assert!(layout.rail().right() <= layout.content().origin.x);
    assert_eq!(layout.header().bottom(), layout.action_bar().origin.y);
    assert_eq!(layout.header().size.height, 32.0);
    assert_eq!(layout.action_bar().bottom(), layout.content().origin.y);
    assert!(layout.search().origin.x >= layout.header().origin.x);
    assert!(layout.close().right() <= layout.header().right());
}

#[test]
fn page_registers_host_boundary_and_page_actions() {
    let dispatch = UiDispatch::default();
    let input = zeta_ui::TextInput::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let actions = SettingsPageActionAvailability::none()
        .with_reset_enabled(true)
        .with_save_enabled(true);
    let page = SettingsPage::new(
        zeta_ui::Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &input,
        CaretVisibility::Visible,
        style(),
        actions,
        &dispatch,
        &mut text_layout,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(zeta_ui::Color::WHITE);
    let parent = zui::ElementId::scoped(1, 1);
    let page = page.with_parent(parent);
    frame.draw_component(&page);
    let nodes = frame.interaction().accessibility_nodes(&dispatch);
    assert!(nodes.iter().any(|node| node.id == SETTINGS_PAGE));
    assert!(nodes.iter().any(|node| node.id == SETTINGS_SEARCH_INPUT));
    assert!(nodes.iter().any(|node| node.id == SETTINGS_CLOSE));
    assert!(nodes.iter().any(|node| node.id == SETTINGS_NAV_BACK));
    assert!(
        nodes
            .iter()
            .any(|node| node.id == SETTINGS_NAV_LANGUAGE_SERVERS)
    );
    assert!(nodes.iter().any(|node| node.id == SETTINGS_RESET));
    assert!(nodes.iter().any(|node| node.id == SETTINGS_SAVE));
    assert!(
        nodes
            .iter()
            .all(|node| { node.id != SETTINGS_PAGE || node.role == AccessibilityRole::Group })
    );
}
