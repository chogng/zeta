//! TabPart's model, chrome UI, presentation state, and stable interaction identities.

mod identity;
mod inspector_state;
mod model;
mod style;
mod tabs;
mod tabs_state;
mod titlebar;
mod toolbar;

pub use model::{
    TabGroup, TabGroupId, TabInput, TabInputChange, TabInputKey, TabInputMetadata, TabPart,
};

pub use identity::{
    ADD_SESSION, FIRST_TAB_CONTAINER_SESSION_TAB, FIRST_TITLEBAR_SESSION_TAB, SESSION_SEARCH_INPUT,
    TAB_CONTAINER, TAB_CONTAINER_ACTION_BAR, TAB_CONTAINER_LIST, TAB_CONTAINER_SETTINGS_TAB,
    TAB_CONTAINER_TOGGLE, TAB_CONTAINER_TOOLBAR, TITLEBAR, TITLEBAR_SETTINGS_TAB,
    TITLEBAR_TAB_CONTAINER, TITLEBAR_TAB_LIST, WINDOW, WORKSPACE_PANE_TOGGLE, session_tab_id,
    tab_group_list_id, titlebar_session_tab_id, titlebar_tab_group_list_id,
};
pub use inspector_state::InspectorPartState;
pub use style::WorkbenchUiStyle;
pub use tabs::{
    TabContainer, TabContainerPlacement, WorkbenchTab, WorkbenchTabGroup, tab_input_element_id,
    workbench_tab_groups,
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
        SearchBoxStyle::new(input, icons::SEARCH, Color::rgb(126, 126, 132)).with_icon_size(18.0),
        icons::GEAR,
        icons::ADD,
        icons::LAYOUT_SIDEBAR_LEFT,
        icons::LAYOUT_SIDEBAR_LEFT_OFF_EMPTY,
        icons::LAYOUT_SIDEBAR_RIGHT,
        icons::LAYOUT_SIDEBAR_RIGHT_OFF_EMPTY,
    )
}
