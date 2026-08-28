use crate::TabGroupId;
use crate::TabId;
use zui::ui::ElementId;

const SHELL_SCOPE: u32 = 1;
const TAB_CONTAINER_SCOPE: u32 = 14;
const TITLEBAR_TAB_SCOPE: u32 = 17;
const TAB_CONTAINER_GROUP_SCOPE: u32 = 18;
const TITLEBAR_TAB_GROUP_SCOPE: u32 = 19;
const TAB_CONTAINER_CLOSE_SCOPE: u32 = 20;
const TITLEBAR_TAB_CLOSE_SCOPE: u32 = 21;
const TAB_LAYOUT_MENU_SCOPE: u32 = 23;
const TAB_CONTAINER_ACTION_SCOPE: u32 = 24;
const TITLEBAR_TAB_ACTION_SCOPE: u32 = 25;
const FIRST_SESSION_TAB: u32 = 100;

pub const WINDOW: ElementId = ElementId::scoped(SHELL_SCOPE, 1);
pub const TITLEBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 2);
pub const TAB_CONTAINER_TOGGLE: ElementId = ElementId::scoped(SHELL_SCOPE, 12);
pub const TAB_CONTAINER: ElementId = ElementId::scoped(SHELL_SCOPE, 13);
pub const TAB_CONTAINER_SETTINGS_TAB: ElementId = ElementId::scoped(TAB_CONTAINER_SCOPE, 4);
pub const TAB_CONTAINER_SETTINGS_CLOSE: ElementId = ElementId::scoped(TAB_CONTAINER_CLOSE_SCOPE, 4);
pub const TAB_CONTAINER_SETTINGS_ACTION: ElementId =
    ElementId::scoped(TAB_CONTAINER_ACTION_SCOPE, 4);
pub const TITLEBAR_TAB_CONTAINER: ElementId = ElementId::scoped(SHELL_SCOPE, 53);
pub const TITLEBAR_SETTINGS_TAB: ElementId = ElementId::scoped(TITLEBAR_TAB_SCOPE, 4);
pub const TITLEBAR_SETTINGS_CLOSE: ElementId = ElementId::scoped(TITLEBAR_TAB_CLOSE_SCOPE, 4);
pub const TITLEBAR_SETTINGS_ACTION: ElementId = ElementId::scoped(TITLEBAR_TAB_ACTION_SCOPE, 4);
pub const WORKSPACE_PANE_TOGGLE: ElementId = ElementId::scoped(SHELL_SCOPE, 22);
pub const TAB_CONTAINER_TOOLBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 24);
pub const SESSION_SEARCH_INPUT: ElementId = ElementId::scoped(SHELL_SCOPE, 25);
pub const TAB_CONTAINER_ACTION_BAR: ElementId = ElementId::scoped(SHELL_SCOPE, 26);
pub const ADD_SESSION: ElementId = ElementId::scoped(SHELL_SCOPE, 27);
pub const TAB_LAYOUT_MENU_TRIGGER: ElementId = ElementId::scoped(SHELL_SCOPE, 54);
pub const TAB_LAYOUT_MENU: ElementId = ElementId::scoped(TAB_LAYOUT_MENU_SCOPE, 1);
pub const TAB_LAYOUT_MENU_MOVE_TO_TITLEBAR: ElementId = ElementId::scoped(TAB_LAYOUT_MENU_SCOPE, 2);
pub const FIRST_TAB_CONTAINER_SESSION_TAB: ElementId = ElementId::scoped(TAB_CONTAINER_SCOPE, 2);
pub const FIRST_TITLEBAR_SESSION_TAB: ElementId = ElementId::scoped(TITLEBAR_TAB_SCOPE, 2);
pub const FIRST_TAB_CONTAINER_SESSION_CLOSE: ElementId =
    ElementId::scoped(TAB_CONTAINER_CLOSE_SCOPE, FIRST_SESSION_TAB);
pub const FIRST_TAB_CONTAINER_SESSION_ACTION: ElementId =
    ElementId::scoped(TAB_CONTAINER_ACTION_SCOPE, FIRST_SESSION_TAB);
pub const FIRST_TITLEBAR_SESSION_ACTION: ElementId =
    ElementId::scoped(TITLEBAR_TAB_ACTION_SCOPE, FIRST_SESSION_TAB);
pub const TAB_CONTAINER_LIST: ElementId = ElementId::scoped(TAB_CONTAINER_GROUP_SCOPE, 1);
pub const TITLEBAR_TAB_LIST: ElementId = ElementId::scoped(TITLEBAR_TAB_GROUP_SCOPE, 1);

pub fn session_tab_id(id: TabId) -> ElementId {
    if id.value() == TabId::FIRST.value() {
        return FIRST_TAB_CONTAINER_SESSION_TAB;
    }
    tab_element_id(TAB_CONTAINER_SCOPE, FIRST_SESSION_TAB, id, "session tab")
}

pub fn titlebar_session_tab_id(id: TabId) -> ElementId {
    if id.value() == TabId::FIRST.value() {
        return FIRST_TITLEBAR_SESSION_TAB;
    }
    tab_element_id(
        TITLEBAR_TAB_SCOPE,
        FIRST_SESSION_TAB,
        id,
        "titlebar session tab",
    )
}

pub fn session_tab_close_id(id: TabId) -> ElementId {
    tab_element_id(
        TAB_CONTAINER_CLOSE_SCOPE,
        FIRST_SESSION_TAB,
        id,
        "session tab close button",
    )
}

pub fn titlebar_session_tab_close_id(id: TabId) -> ElementId {
    tab_element_id(
        TITLEBAR_TAB_CLOSE_SCOPE,
        FIRST_SESSION_TAB,
        id,
        "titlebar session tab close button",
    )
}

pub fn session_tab_action_id(id: TabId) -> ElementId {
    tab_element_id(
        TAB_CONTAINER_ACTION_SCOPE,
        FIRST_SESSION_TAB,
        id,
        "session tab action button",
    )
}

pub fn titlebar_session_tab_action_id(id: TabId) -> ElementId {
    tab_element_id(
        TITLEBAR_TAB_ACTION_SCOPE,
        FIRST_SESSION_TAB,
        id,
        "titlebar session tab action button",
    )
}

pub fn tab_group_list_id(group: TabGroupId) -> ElementId {
    if group == TabGroupId::DEFAULT {
        return TAB_CONTAINER_LIST;
    }
    tab_group_element_id(TAB_CONTAINER_GROUP_SCOPE, group, "tab group list")
}

pub fn titlebar_tab_group_list_id(group: TabGroupId) -> ElementId {
    if group == TabGroupId::DEFAULT {
        return TITLEBAR_TAB_LIST;
    }
    tab_group_element_id(TITLEBAR_TAB_GROUP_SCOPE, group, "titlebar tab group list")
}

fn tab_element_id(scope: u32, first: u32, id: TabId, label: &str) -> ElementId {
    let local = id
        .value()
        .checked_sub(TabId::FIRST.value())
        .and_then(|offset| first.checked_add(offset))
        .unwrap_or_else(|| panic!("{label} index must fit its element scope"));
    ElementId::scoped(scope, local)
}

fn tab_group_element_id(scope: u32, group: TabGroupId, label: &str) -> ElementId {
    let local = u32::try_from(group.value())
        .unwrap_or_else(|_| panic!("{label} identity must fit its element scope"));
    ElementId::scoped(scope, local)
}
