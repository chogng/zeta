use zeta_commands::AppCommandId;
use zeta_ui_components::ScrollAxis;
use zeta_ui_components::ScrollCommand;
use zeta_ui_components::ScrollViewStyle;
use zeta_ui_components::ScrollbarStyle;

use super::SettingsActivation;
use super::SettingsState;
use crate::SETTINGS_CLOSE;
use crate::SETTINGS_NAV_APPEARANCE;
use crate::SETTINGS_NAV_GENERAL;
use crate::SETTINGS_NAV_REMOTE;
use crate::SettingsKeybindingsViewport;
use crate::SettingsPageSection;
use crate::keyboard_shortcut_row_element;
use std::time::Instant;
use zui::ui::Color;
use zui::ui::Point;
use zui::ui::Rect;

fn keybindings_viewport() -> SettingsKeybindingsViewport {
    SettingsKeybindingsViewport::new(
        Rect::from_xywh(0.0, 0.0, 600.0, 500.0),
        AppCommandId::BINDABLE.len(),
        0,
        ScrollViewStyle::new(ScrollbarStyle::new(
            Color::TRANSPARENT,
            Color::rgb(126, 126, 132),
        )),
    )
}

#[test]
fn activation_selects_the_remote_section() {
    let mut settings = SettingsState::default();

    assert_eq!(
        settings.activate(SETTINGS_NAV_REMOTE),
        SettingsActivation::OpenRemote
    );
    assert_eq!(settings.section(), SettingsPageSection::Remote);
}

#[test]
fn activation_changes_owned_state_and_returns_host_effects() {
    let mut settings = SettingsState::default();
    assert_eq!(settings.section(), SettingsPageSection::General);
    assert_eq!(
        settings.activate(SETTINGS_NAV_APPEARANCE),
        SettingsActivation::Changed
    );
    assert_eq!(settings.section(), SettingsPageSection::Appearance);
    assert_eq!(
        settings.activate(SETTINGS_NAV_GENERAL),
        SettingsActivation::Changed
    );
    assert_eq!(settings.section(), SettingsPageSection::General);
}

#[test]
fn close_button_returns_the_host_close_effect() {
    let mut settings = SettingsState::default();

    assert_eq!(settings.activate(SETTINGS_CLOSE), SettingsActivation::Close);
}

#[test]
fn shortcut_rows_are_inactive_outside_the_keybindings_section() {
    let mut settings = SettingsState::default();

    assert_eq!(
        settings.activate(keyboard_shortcut_row_element(AppCommandId::Copy)),
        SettingsActivation::Ignored
    );
}

#[test]
fn keybindings_scroll_clamps_to_list_content_and_survives_section_changes() {
    let mut settings = SettingsState::default();
    assert_eq!(
        settings.activate(crate::SETTINGS_NAV_KEYBINDINGS),
        SettingsActivation::Changed
    );

    assert!(settings.scroll_keybindings(
        ScrollCommand::ToEnd(ScrollAxis::Vertical),
        keybindings_viewport(),
        Instant::now(),
    ));
    assert_eq!(settings.keybindings_scroll_state().vertical_offset(), 148.0);

    assert_eq!(
        settings.activate(SETTINGS_NAV_GENERAL),
        SettingsActivation::Changed
    );
    assert_eq!(
        settings.activate(crate::SETTINGS_NAV_KEYBINDINGS),
        SettingsActivation::Changed
    );
    assert_eq!(settings.keybindings_scroll_state().vertical_offset(), 148.0);
}

#[test]
fn keybindings_scrollbar_thumb_drag_and_track_click_share_the_list_state() {
    let mut settings = SettingsState::default();
    let viewport = keybindings_viewport();
    let view = viewport.list(
        settings.keybindings_scroll_state(),
        settings.keybindings_scrollbar_presentation(),
    );
    let scrollbar = view
        .scroll_view()
        .vertical_scrollbar()
        .expect("keybindings should overflow");
    let thumb = scrollbar.thumb_bounds();
    let now = Instant::now();

    assert!(
        settings
            .press_keybindings_scrollbar(
                Point::new(thumb.origin.x + 2.0, thumb.origin.y + 2.0),
                viewport,
                now,
            )
            .handled
    );
    assert!(
        settings
            .keybindings_scrollbar_pointer_moved(
                Point::new(
                    thumb.origin.x + 2.0,
                    scrollbar.track_bounds().bottom() - 2.0
                ),
                viewport,
                now,
            )
            .handled
    );
    assert_eq!(settings.keybindings_scroll_state().vertical_offset(), 148.0);
    assert!(
        settings
            .release_keybindings_scrollbar(Point::new(-1.0, -1.0), viewport, now)
            .handled
    );

    let track_point = Point::new(
        scrollbar.track_bounds().origin.x + 2.0,
        scrollbar.track_bounds().origin.y + 1.0,
    );
    assert!(
        settings
            .press_keybindings_scrollbar(track_point, viewport, now)
            .handled
    );
    assert_eq!(settings.keybindings_scroll_state().vertical_offset(), 0.0);
}

#[test]
fn focused_keybinding_is_scrolled_into_the_list_viewport() {
    let mut settings = SettingsState::default();
    let viewport = keybindings_viewport();
    settings.scroll_keybindings(
        ScrollCommand::ToEnd(ScrollAxis::Vertical),
        viewport,
        Instant::now(),
    );

    assert!(settings.ensure_keybinding_visible(
        keyboard_shortcut_row_element(AppCommandId::Copy),
        viewport,
        Instant::now(),
    ));
    assert_eq!(settings.keybindings_scroll_state().vertical_offset(), 0.0);
}
