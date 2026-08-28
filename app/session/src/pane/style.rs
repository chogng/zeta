//! Host-resolved Session Pane colors.

use zeta_ui_components::ScrollViewStyle;
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
}

impl SessionPaneStyle {
    pub fn from_theme(theme: UiTheme) -> Self {
        Self::new(
            theme.content_background,
            theme.side_bar_background,
            theme.list_hover_background,
            theme.border,
            theme.foreground,
            theme.muted_foreground,
            theme.accent,
            theme.success,
            theme.warning,
            theme.error,
            theme.list_active_background,
            theme.file_list_scroll_view_style(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        surface: Color,
        surface_raised: Color,
        surface_hovered: Color,
        border: Color,
        text: Color,
        text_muted: Color,
        accent: Color,
        success: Color,
        warning: Color,
        error: Color,
        selected: Color,
        scroll_view: ScrollViewStyle,
    ) -> Self {
        Self {
            surface,
            surface_raised,
            surface_hovered,
            border,
            text,
            text_muted,
            accent,
            success,
            warning,
            error,
            selected,
            scroll_view,
        }
    }
}
