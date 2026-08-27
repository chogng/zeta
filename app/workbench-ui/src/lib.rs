//! Workbench navigation, titlebar, and presentation state.

use zeta_ui_components::*;
use zui::ui::*;

mod workbench;

pub use workbench::{
    ADD_SESSION, FIRST_TAB_CONTAINER_SESSION_TAB, FIRST_TITLEBAR_SESSION_TAB, InspectorPartState,
    SESSION_SEARCH_INPUT, TAB_CONTAINER, TAB_CONTAINER_ACTION_BAR, TAB_CONTAINER_LIST,
    TAB_CONTAINER_SETTINGS_TAB, TAB_CONTAINER_TOGGLE, TAB_CONTAINER_TOOLBAR, TITLEBAR,
    TITLEBAR_HEIGHT, TITLEBAR_SETTINGS_TAB, TITLEBAR_TAB_CONTAINER, TITLEBAR_TAB_LIST,
    TabContainer, TabContainerPlacement, TabContainerState, TabContainerToolbar, Titlebar,
    TitlebarInsets, WINDOW, WORKSPACE_PANE_TOGGLE, WorkbenchTab, WorkbenchTabGroup,
    WorkbenchUiStyle, session_tab_id, tab_group_list_id, tab_input_element_id,
    titlebar_session_tab_id, titlebar_tab_group_list_id, workbench_tab_groups,
};
