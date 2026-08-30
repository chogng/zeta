//! TabPart's model, chrome UI, presentation state, and stable interaction identities.

mod identity;
mod session_input;
mod session_search;
mod style;
mod tab_context_menu;
mod tab_dirs_preview;
mod tab_mount;
mod tabs;
mod tabs_state;
mod titlebar;
mod toolbar;

pub use identity::{
    ADD_SESSION, CHANGES_PANE_BUTTON, SESSION_SEARCH_INPUT, TAB_CONTAINER, TAB_CONTAINER_TOGGLE,
    WINDOW,
};
pub use session_input::session_tab_input;
pub use session_search::SessionSearchState;
#[cfg(test)]
use style::WorkbenchColors;
pub use style::WorkbenchUiStyle;
pub use tab_context_menu::{
    TAB_CONTEXT_MENU_MOVE_TO_NEW_GROUP, TAB_RENAME_INPUT, TabContextMenu, TabContextMenuAction,
    TabContextMenuActivation, TabContextMenuState, TabContextMenuStyle, tab_group_menu_element_id,
    update_tab_context_menu_pointer,
};
pub use tab_mount::TabIntent;
pub use tabs::{
    TabContainer, TabContainerPlacement, mounted_tab_element_id, tab_input_element_id,
    tab_intent_for_element, tab_key_for_element, workbench_tab_groups,
};
pub use tabs_state::TabContainerState;
pub use titlebar::{TITLEBAR_HEIGHT, Titlebar, TitlebarInsets};
pub use toolbar::TabContainerToolbar;

#[cfg(test)]
pub(crate) use identity::{
    FIRST_TAB_CONTAINER_SESSION_TAB, FIRST_TITLEBAR_SESSION_TAB, TAB_CONTAINER_ACTION_BAR,
    TAB_CONTAINER_LIST, TAB_CONTAINER_SETTINGS_ACTION, TAB_CONTAINER_SETTINGS_CLOSE,
    TAB_CONTAINER_SETTINGS_TAB, TAB_CONTAINER_TOOLBAR, TITLEBAR, TITLEBAR_TAB_LIST,
};
#[cfg(test)]
pub(crate) use tab_context_menu::TAB_CONTEXT_MENU;

#[cfg(test)]
fn test_style() -> WorkbenchUiStyle {
    use crate::{
        Color, Edges, InputBoxStateColors, InputBoxStyle, ScrollViewStyle, ScrollbarStyle,
        SearchBoxStyle, TextStyle,
    };
    use zeta_icons::icons;

    let input = InputBoxStyle::new(
        InputBoxStateColors::new(Color::TRANSPARENT, Color::TRANSPARENT, Color::TRANSPARENT),
        InputBoxStateColors::new(Color::TRANSPARENT, Color::TRANSPARENT, Color::TRANSPARENT),
        TextStyle::new(12.0, Color::rgb(38, 38, 41)),
        TextStyle::new(12.0, Color::rgb(126, 126, 132)),
    )
    .with_padding(Edges::uniform(4.0));
    WorkbenchUiStyle::new(
        WorkbenchColors {
            content_background: Color::WHITE,
            side_bar_background: Color::rgb(246, 246, 247),
            border: Color::rgb(222, 222, 224),
            foreground: Color::rgb(38, 38, 41),
            muted_foreground: Color::rgb(126, 126, 132),
            control_hover_background: Color::rgb(232, 232, 232),
            hover_foreground: Color::rgb(245, 245, 247),
            hover_background: Color::rgb(45, 46, 51),
            hover_border: Color::rgba(255, 255, 255, 24),
            hover_shadow: Color::rgba(0, 0, 0, 48),
            menu_background: Color::WHITE,
            menu_hover_background: Color::rgb(226, 226, 228),
            tab_hover_background: Color::rgb(226, 226, 228),
            tab_active_background: Color::rgb(235, 235, 237),
            action_bar_background: Color::rgb(245, 245, 246),
            title_bar_background: Color::WHITE,
            title_bar_action_foreground: Color::rgb(66, 66, 66),
            title_bar_hover_background: Color::rgb(229, 229, 229),
            accent: Color::rgb(15, 110, 96),
            success: Color::rgb(16, 124, 16),
            warning: Color::rgb(154, 103, 0),
            error: Color::rgb(180, 38, 38),
        },
        SearchBoxStyle::new(input, icons::SEARCH, Color::rgb(126, 126, 132)).with_icon_size(18.0),
        ScrollViewStyle::new(
            ScrollbarStyle::new(Color::TRANSPARENT, Color::rgb(126, 126, 132))
                .with_thickness(6.0)
                .with_hovered_colors(Color::TRANSPARENT, Color::rgb(90, 90, 96))
                .with_active_colors(Color::TRANSPARENT, Color::rgb(64, 64, 70)),
        ),
        icons::GEAR,
        icons::ADD,
        icons::CLOSE,
        icons::PINNED,
        icons::LAYOUT_SIDEBAR_LEFT,
        icons::LAYOUT_SIDEBAR_LEFT_OFF_EMPTY,
        icons::DIFF,
    )
}
