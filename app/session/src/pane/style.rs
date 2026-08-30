//! Host-resolved Session Pane colors.

use zeta_ui_components::ScrollViewStyle;
use zeta_ui_theme::TypographyStyle;
use zeta_ui_theme::UiTheme;
use zui::ui::Color;

#[derive(Clone, Copy)]
pub struct SessionPaneStyle {
    pub surface: Color,
    pub surface_raised: Color,
    pub surface_hovered: Color,
    pub border: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub selected: Color,
    pub scroll_view: ScrollViewStyle,
    pub compact_action_label: TypographyStyle,
}

impl SessionPaneStyle {
    pub fn from_theme(theme: UiTheme) -> Self {
        Self {
            surface: theme.content_background,
            surface_raised: theme.side_bar_background,
            surface_hovered: theme.list_hover_background,
            border: theme.border,
            text: theme.foreground,
            text_muted: theme.muted_foreground,
            accent: theme.accent,
            success: theme.success,
            warning: theme.warning,
            error: theme.error,
            selected: theme.list_active_background,
            scroll_view: theme.file_list_scroll_view_style(),
            compact_action_label: theme.compact_action_label,
        }
    }
}
