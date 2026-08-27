//! TabPart's model, chrome UI, presentation state, and stable interaction identities.

mod identity;
mod session_input;
mod session_search;
mod style;
mod tab_context_menu;
mod tab_group;
mod tab_input;
mod tab_mount;
mod tab_part;
mod tab_status;
mod tabs;
mod tabs_state;
mod titlebar;
mod toolbar;

pub use session_input::session_tab_input;
pub use session_search::SessionSearchState;
pub use tab_context_menu::{
    TAB_CONTEXT_MENU, TabContextMenu, TabContextMenuAction, TabContextMenuState,
    TabContextMenuStyle, update_tab_context_menu_pointer,
};
pub use tab_group::{TabGroup, TabGroupId};
pub use tab_input::{TabInput, TabInputChange, TabInputKey, TabInputMetadata};
pub use tab_part::{TabId, TabPart};
pub use tab_status::{TabStatus, TabStatusKind};

pub use identity::{
    ADD_SESSION, FIRST_TAB_CONTAINER_SESSION_CLOSE, FIRST_TAB_CONTAINER_SESSION_TAB,
    FIRST_TITLEBAR_SESSION_TAB, SESSION_SEARCH_INPUT, TAB_CONTAINER, TAB_CONTAINER_ACTION_BAR,
    TAB_CONTAINER_LIST, TAB_CONTAINER_SETTINGS_CLOSE, TAB_CONTAINER_SETTINGS_TAB,
    TAB_CONTAINER_TOGGLE, TAB_CONTAINER_TOOLBAR, TITLEBAR, TITLEBAR_SETTINGS_CLOSE,
    TITLEBAR_SETTINGS_TAB, TITLEBAR_TAB_CONTAINER, TITLEBAR_TAB_LIST, WINDOW,
    WORKSPACE_PANE_TOGGLE, session_tab_close_id, session_tab_id, tab_group_list_id,
    titlebar_session_tab_close_id, titlebar_session_tab_id, titlebar_tab_group_list_id,
};
pub use style::WorkbenchUiStyle;
pub use tab_mount::TabIntent;
pub use tabs::{
    TabContainer, TabContainerPlacement, WorkbenchTab, WorkbenchTabGroup, tab_input_element_id,
    tab_intent_for_element, tab_key_for_element, workbench_tab_groups,
};
pub use tabs_state::TabContainerState;
pub use titlebar::{TITLEBAR_HEIGHT, Titlebar, TitlebarInsets};
pub use toolbar::TabContainerToolbar;

#[cfg(test)]
fn test_style() -> WorkbenchUiStyle {
    use crate::{Color, Edges, InputBoxStateColors, InputBoxStyle, SearchBoxStyle, TextStyle};
    use zeta_icons::icons;

    let input = InputBoxStyle::new(
        InputBoxStateColors::new(Color::TRANSPARENT, Color::TRANSPARENT, Color::TRANSPARENT),
        InputBoxStateColors::new(Color::TRANSPARENT, Color::TRANSPARENT, Color::TRANSPARENT),
        TextStyle::new(12.0, Color::rgb(38, 38, 41)),
        TextStyle::new(12.0, Color::rgb(126, 126, 132)),
    )
    .with_padding(Edges::uniform(4.0));
    WorkbenchUiStyle::new(
        Color::WHITE,
        Color::rgb(246, 246, 247),
        Color::rgb(248, 248, 249),
        Color::rgb(222, 222, 224),
        Color::rgb(38, 38, 41),
        Color::rgb(126, 126, 132),
        Color::rgb(235, 235, 237),
        Color::rgb(15, 110, 96),
        Color::rgb(16, 124, 16),
        Color::rgb(154, 103, 0),
        Color::rgb(180, 38, 38),
        SearchBoxStyle::new(input, icons::SEARCH, Color::rgb(126, 126, 132)).with_icon_size(18.0),
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
