use zeta_icons::icons;
use zeta_ui_components::ActionBarStyle;
use zeta_ui_components::ButtonBackgrounds;
use zeta_ui_components::ButtonStyle;
use zeta_ui_components::InputBoxStateColors;
use zeta_ui_components::InputBoxStyle;
use zeta_ui_components::InteractionRegion;
use zeta_ui_components::SearchBoxStyle;
use zui::ui::AccessibilityRole;
use zui::ui::AccessibilitySelection;
use zui::ui::CaretVisibility;
use zui::ui::Color;
use zui::ui::ElementId;
use zui::ui::FocusBehavior;
use zui::ui::InteractionFrame;
use zui::ui::NodeAction;
use zui::ui::TextInputLayoutEngine;
use zui::ui::TextStyle;
use zui::ui::UiDispatch;
use zui::ui::UiFrame;

use super::{
    SETTINGS_CLOSE, SETTINGS_NAV_APPEARANCE, SETTINGS_NAV_BACK, SETTINGS_NAV_GENERAL,
    SETTINGS_NAV_KEYBINDINGS, SETTINGS_NAV_LANGUAGE_SERVERS, SETTINGS_PAGE, SETTINGS_RESET,
    SETTINGS_SAVE, SETTINGS_SEARCH_INPUT, SettingsPage, SettingsPageActionAvailability,
    SettingsPageLayout, SettingsPageMode, SettingsPageSection, SettingsPageStyle,
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
        ActionBarStyle::new(action, zui::ui::Size::new(100.0, 32.0)),
    )
}

#[test]
fn layout_keeps_rail_header_action_bar_and_content_separate() {
    let layout =
        SettingsPageLayout::for_viewport(zui::ui::Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0));
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
    let input = zui::ui::TextInput::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let actions = SettingsPageActionAvailability::none()
        .with_reset_enabled(true)
        .with_save_enabled(true);
    let page = SettingsPage::new(
        zui::ui::Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &input,
        CaretVisibility::Visible,
        style(),
        actions,
        &dispatch,
        &mut text_layout,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(zui::ui::Color::WHITE);
    let parent = zui::ui::ElementId::scoped(1, 1);
    let page = page.with_parent(parent);
    frame.draw_component(&page);
    let nodes = frame.interaction().accessibility_nodes(&dispatch);
    assert!(nodes.iter().any(|node| node.id == SETTINGS_PAGE));
    assert!(nodes.iter().any(|node| node.id == SETTINGS_SEARCH_INPUT));
    assert!(nodes.iter().any(|node| node.id == SETTINGS_CLOSE));
    assert!(nodes.iter().any(|node| node.id == SETTINGS_NAV_BACK));
    for id in [
        SETTINGS_NAV_GENERAL,
        SETTINGS_NAV_LANGUAGE_SERVERS,
        SETTINGS_NAV_APPEARANCE,
        SETTINGS_NAV_KEYBINDINGS,
    ] {
        let node = frame.interaction().node(id).expect("section node");
        assert_eq!(node.action(), zui::ui::NodeAction::Activate);
    }
    assert!(nodes.iter().any(|node| node.id == SETTINGS_RESET));
    assert!(nodes.iter().any(|node| node.id == SETTINGS_SAVE));
    assert!(
        nodes
            .iter()
            .all(|node| { node.id != SETTINGS_PAGE || node.role == AccessibilityRole::Group })
    );
}

#[test]
fn selected_section_is_projected_to_the_navigation_semantics() {
    let dispatch = UiDispatch::default();
    let input = zui::ui::TextInput::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let page = SettingsPage::new_with_header_height_and_section(
        zui::ui::Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        32.0,
        &input,
        CaretVisibility::Visible,
        style(),
        SettingsPageActionAvailability::none(),
        SettingsPageSection::Appearance,
        &dispatch,
        &mut text_layout,
    );
    assert_eq!(page.section(), SettingsPageSection::Appearance);

    let mut frame = UiFrame::<InteractionFrame>::new(zui::ui::Color::WHITE);
    frame.draw_component(&page);
    let nodes = frame.interaction().accessibility_nodes(&dispatch);
    assert_eq!(
        nodes
            .iter()
            .find(|node| node.id == SETTINGS_NAV_APPEARANCE)
            .expect("appearance node")
            .selection,
        AccessibilitySelection::Selected
    );
    assert_eq!(
        nodes
            .iter()
            .find(|node| node.id == SETTINGS_NAV_GENERAL)
            .expect("general node")
            .selection,
        AccessibilitySelection::Unselected
    );
}

#[test]
fn surface_mode_leaves_the_workbench_interactive() {
    let dispatch = UiDispatch::default();
    let input = zui::ui::TextInput::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let host_id = ElementId::scoped(1, 1);
    let host = InteractionRegion::new(
        "SettingsHost",
        host_id,
        zui::ui::Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        AccessibilityRole::Group,
        "Main surface",
    )
    .with_focus(FocusBehavior::TabStop)
    .with_action(NodeAction::Activate);
    let page = SettingsPage::new(
        zui::ui::Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &input,
        CaretVisibility::Visible,
        style(),
        SettingsPageActionAvailability::none(),
        &dispatch,
        &mut text_layout,
    )
    .with_mode(SettingsPageMode::Surface);
    let mut frame = UiFrame::<InteractionFrame>::new(zui::ui::Color::WHITE);
    frame.draw_component(&host);
    frame.draw_component(&page);

    assert!(frame.interaction().focus_order().any(|id| id == host_id));
}
