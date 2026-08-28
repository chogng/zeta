use crate::Color;
use crate::Icon;
use crate::SearchBoxStyle;
use zeta_icons::icons;
use zeta_ui_theme::UiTheme;

/// Semantic colors consumed by Workbench chrome components.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkbenchColors {
    pub content_background: Color,
    pub side_bar_background: Color,
    pub border: Color,
    pub foreground: Color,
    pub muted_foreground: Color,
    pub control_hover_background: Color,
    pub menu_background: Color,
    pub menu_hover_background: Color,
    pub tab_hover_background: Color,
    pub tab_active_background: Color,
    pub action_bar_background: Color,
    pub title_bar_background: Color,
    pub title_bar_action_foreground: Color,
    pub title_bar_hover_background: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
}

/// Host-resolved colors, icons, and input styling for Workbench chrome.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkbenchUiStyle {
    pub(super) colors: WorkbenchColors,
    pub(super) search: SearchBoxStyle,
    pub(super) settings_icon: Icon,
    pub(super) add_icon: Icon,
    pub(super) close_icon: Icon,
    pub(super) pinned_icon: Icon,
    pub(super) tabs_expanded_icon: Icon,
    pub(super) tabs_collapsed_icon: Icon,
    pub(super) workspace_visible_icon: Icon,
    pub(super) workspace_hidden_icon: Icon,
}

impl WorkbenchUiStyle {
    pub fn from_theme(theme: UiTheme) -> Self {
        Self::new(
            WorkbenchColors {
                content_background: theme.content_background,
                side_bar_background: theme.side_bar_background,
                border: theme.border,
                foreground: theme.foreground,
                muted_foreground: theme.muted_foreground,
                control_hover_background: theme.list_hover_background,
                menu_background: theme.menu_background,
                menu_hover_background: theme.menu_hover_background,
                tab_hover_background: theme.tab_hover_background,
                tab_active_background: theme.tab_active_background,
                action_bar_background: theme.action_bar_background,
                title_bar_background: theme.title_bar_background,
                title_bar_action_foreground: theme.title_bar_action_foreground,
                title_bar_hover_background: theme.title_bar_hover_background,
                accent: theme.accent,
                success: theme.success,
                warning: theme.warning,
                error: theme.error,
            },
            theme.search_box_style(),
            icons::GEAR,
            icons::ADD,
            icons::CLOSE,
            icons::PINNED,
            icons::LAYOUT_SIDEBAR_LEFT,
            icons::LAYOUT_SIDEBAR_LEFT_OFF_EMPTY,
            icons::LAYOUT_SIDEBAR_RIGHT,
            icons::LAYOUT_SIDEBAR_RIGHT_OFF_EMPTY,
        )
    }

    pub fn new(
        colors: WorkbenchColors,
        search: SearchBoxStyle,
        settings_icon: Icon,
        add_icon: Icon,
        close_icon: Icon,
        pinned_icon: Icon,
        tabs_expanded_icon: Icon,
        tabs_collapsed_icon: Icon,
        workspace_visible_icon: Icon,
        workspace_hidden_icon: Icon,
    ) -> Self {
        Self {
            colors,
            search,
            settings_icon,
            add_icon,
            close_icon,
            pinned_icon,
            tabs_expanded_icon,
            tabs_collapsed_icon,
            workspace_visible_icon,
            workspace_hidden_icon,
        }
    }
}
