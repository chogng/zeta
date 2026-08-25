use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;
use zeta_ui::{
    Border, ButtonBackgrounds, ButtonStyle, Color, CornerRadii, Edges, InputBoxStateColors,
    InputBoxStyle, Rect, Size, SwitchColors, SwitchStateColors, SwitchStyle, TextStyle,
};
use zui::ui::ElementId;

use crate::language_server_settings::{
    LANGUAGE_SERVER_BASH, LANGUAGE_SERVER_JSON, LANGUAGE_SERVER_MODE_AUTOMATIC,
    LANGUAGE_SERVER_MODE_ENABLED, LANGUAGE_SERVER_RUST, LanguageServerSettingsTarget,
};
use crate::shell_style::ShellPalette;

pub(super) const CONTENT_INSET: f32 = 24.0;
const MODE_GAP: f32 = 8.0;
const CONTROL_HEIGHT: f32 = 34.0;

pub(super) fn server_controls() -> [(ElementId, LanguageServerSettingsTarget, &'static str); 3] {
    [
        (
            LANGUAGE_SERVER_RUST,
            LanguageServerSettingsTarget::RustAnalyzer,
            LanguageServerSettingsTarget::RustAnalyzer.label(),
        ),
        (
            LANGUAGE_SERVER_JSON,
            LanguageServerSettingsTarget::Json,
            LanguageServerSettingsTarget::Json.label(),
        ),
        (
            LANGUAGE_SERVER_BASH,
            LanguageServerSettingsTarget::Bash,
            LanguageServerSettingsTarget::Bash.label(),
        ),
    ]
}

pub(super) fn server_bounds(panel: Rect, target: LanguageServerSettingsTarget) -> Rect {
    let index = match target {
        LanguageServerSettingsTarget::RustAnalyzer => 0.0,
        LanguageServerSettingsTarget::Json => 1.0,
        LanguageServerSettingsTarget::Bash => 2.0,
    };
    let total_width = panel.size.width - CONTENT_INSET * 2.0;
    let width = (total_width - MODE_GAP * 2.0) / 3.0;
    Rect::from_xywh(
        panel.origin.x + CONTENT_INSET + index * (width + MODE_GAP),
        panel.origin.y + 58.0,
        width,
        CONTROL_HEIGHT,
    )
}

pub(super) fn mode_controls() -> [(ElementId, LanguageServerModeDto, &'static str); 2] {
    [
        (
            LANGUAGE_SERVER_MODE_AUTOMATIC,
            LanguageServerModeDto::Automatic,
            "Automatic",
        ),
        (
            LANGUAGE_SERVER_MODE_ENABLED,
            LanguageServerModeDto::Enabled,
            "Enabled",
        ),
    ]
}

pub(super) fn mode_bounds(panel: Rect, mode: LanguageServerModeDto) -> Rect {
    let index = match mode {
        LanguageServerModeDto::Disabled => 0.0,
        LanguageServerModeDto::Automatic => 0.0,
        LanguageServerModeDto::Enabled => 1.0,
    };
    let total_width = panel.size.width - CONTENT_INSET * 2.0;
    let width = (total_width - MODE_GAP) / 2.0;
    Rect::from_xywh(
        panel.origin.x + CONTENT_INSET + index * (width + MODE_GAP),
        panel.origin.y + 208.0,
        width,
        CONTROL_HEIGHT,
    )
}

pub(super) fn switch_bounds(panel: Rect) -> Rect {
    Rect::from_xywh(
        panel.right() - CONTENT_INSET - 48.0,
        panel.origin.y + 128.0,
        48.0,
        CONTROL_HEIGHT,
    )
}

pub(super) fn executable_bounds(panel: Rect) -> Rect {
    Rect::from_xywh(
        panel.origin.x + CONTENT_INSET,
        panel.origin.y + 280.0,
        panel.size.width - CONTENT_INSET * 2.0,
        CONTROL_HEIGHT,
    )
}

pub(super) fn close_bounds(panel: Rect) -> Rect {
    Rect::from_xywh(panel.right() - 48.0, panel.origin.y + 14.0, 30.0, 30.0)
}

pub(super) fn reset_bounds(panel: Rect) -> Rect {
    Rect::from_xywh(
        panel.origin.x + CONTENT_INSET,
        panel.bottom() - 54.0,
        130.0,
        CONTROL_HEIGHT,
    )
}

pub(super) fn save_bounds(panel: Rect) -> Rect {
    Rect::from_xywh(
        panel.right() - CONTENT_INSET - 82.0,
        panel.bottom() - 54.0,
        82.0,
        CONTROL_HEIGHT,
    )
}

pub(super) fn input_style(palette: ShellPalette) -> InputBoxStyle {
    InputBoxStyle::new(
        InputBoxStateColors::new(
            palette.surface_raised,
            palette.surface_raised,
            palette.surface_raised,
        ),
        InputBoxStateColors::new(palette.border, palette.text_muted, palette.accent),
        TextStyle::new(13.0, palette.text).with_line_height(18.0),
        TextStyle::new(13.0, palette.text_muted).with_line_height(18.0),
    )
    .with_border_width(1.0)
    .with_corner_radii(CornerRadii::uniform(4.0))
    .with_padding(Edges::new(7.0, 9.0, 7.0, 9.0))
    .with_selection_color(palette.terminal_selection)
    .with_caret_color(palette.accent)
    .with_preedit_underline_color(palette.accent)
}

pub(super) fn mode_button_style(palette: ShellPalette) -> ButtonStyle {
    ButtonStyle::new(
        ButtonBackgrounds::new(palette.surface_raised)
            .with_hovered(palette.surface_hovered)
            .with_focused(palette.surface_hovered)
            .with_pressed(palette.border),
        TextStyle::new(13.0, palette.text).with_line_height(18.0),
    )
    .with_selected_backgrounds(ButtonBackgrounds::new(palette.session_tab_highlight))
    .with_border(Border::uniform(1.0, palette.border))
    .with_corner_radii(CornerRadii::uniform(4.0))
}

pub(super) fn switch_style(palette: ShellPalette) -> SwitchStyle {
    let off_colors = SwitchColors::new(palette.border, Color::WHITE);
    let on_colors = SwitchColors::new(palette.success, Color::WHITE);
    let off = SwitchStateColors::new(off_colors)
        .with_hovered(off_colors)
        .with_focused(off_colors)
        .with_pressed(off_colors)
        .with_disabled(off_colors);
    let on = SwitchStateColors::new(on_colors)
        .with_hovered(on_colors)
        .with_focused(on_colors)
        .with_pressed(on_colors)
        .with_disabled(on_colors);
    SwitchStyle::new(off, on)
        .with_track_size(Size::new(36.0, 20.0))
        .with_thumb_diameter(14.0)
        .with_thumb_inset(3.0)
        .with_track_border(Border::uniform(0.0, Color::TRANSPARENT))
        .with_thumb_border(Border::uniform(0.0, Color::TRANSPARENT))
        .with_track_corner_radii(CornerRadii::uniform(10.0))
        .with_thumb_corner_radii(CornerRadii::uniform(8.0))
}

pub(super) fn quiet_button_style(palette: ShellPalette) -> ButtonStyle {
    ButtonStyle::new(
        ButtonBackgrounds::new(Color::TRANSPARENT)
            .with_hovered(palette.surface_hovered)
            .with_focused(palette.surface_hovered)
            .with_pressed(palette.border)
            .with_disabled(Color::TRANSPARENT),
        TextStyle::new(13.0, palette.text).with_line_height(18.0),
    )
    .with_disabled_text_style(TextStyle::new(13.0, palette.text_muted).with_line_height(18.0))
    .with_corner_radii(CornerRadii::uniform(4.0))
}

pub(super) fn primary_button_style(palette: ShellPalette) -> ButtonStyle {
    ButtonStyle::new(
        ButtonBackgrounds::new(palette.accent)
            .with_hovered(palette.accent)
            .with_focused(palette.accent)
            .with_pressed(palette.border)
            .with_disabled(palette.border),
        TextStyle::new(13.0, Color::WHITE).with_line_height(18.0),
    )
    .with_disabled_text_style(TextStyle::new(13.0, palette.text_muted).with_line_height(18.0))
    .with_corner_radii(CornerRadii::uniform(4.0))
}
