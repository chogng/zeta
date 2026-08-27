use crate::Color;
use crate::Icon;
use crate::SearchBoxStyle;

/// Host-resolved colors, icons, and input styling for Workbench chrome.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkbenchUiStyle {
    pub(super) surface: Color,
    pub(super) surface_raised: Color,
    pub(super) surface_hovered: Color,
    pub(super) border: Color,
    pub(super) text: Color,
    pub(super) text_muted: Color,
    pub(super) selected: Color,
    pub(super) search: SearchBoxStyle,
    pub(super) settings_icon: Icon,
    pub(super) add_icon: Icon,
    pub(super) tabs_expanded_icon: Icon,
    pub(super) tabs_collapsed_icon: Icon,
    pub(super) workspace_visible_icon: Icon,
    pub(super) workspace_hidden_icon: Icon,
}

impl WorkbenchUiStyle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        surface: Color,
        surface_raised: Color,
        surface_hovered: Color,
        border: Color,
        text: Color,
        text_muted: Color,
        selected: Color,
        search: SearchBoxStyle,
        settings_icon: Icon,
        add_icon: Icon,
        tabs_expanded_icon: Icon,
        tabs_collapsed_icon: Icon,
        workspace_visible_icon: Icon,
        workspace_hidden_icon: Icon,
    ) -> Self {
        Self {
            surface,
            surface_raised,
            surface_hovered,
            border,
            text,
            text_muted,
            selected,
            search,
            settings_icon,
            add_icon,
            tabs_expanded_icon,
            tabs_collapsed_icon,
            workspace_visible_icon,
            workspace_hidden_icon,
        }
    }
}
