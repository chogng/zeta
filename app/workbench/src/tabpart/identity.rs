use zeta_workbench::TabGroupId;
use zui::ui::ElementId;

const SHELL_SCOPE: u32 = 1;
const TAB_CONTAINER_SCOPE: u32 = 14;
const TITLEBAR_TAB_SCOPE: u32 = 17;
const TAB_CONTAINER_GROUP_SCOPE: u32 = 18;
const TITLEBAR_TAB_GROUP_SCOPE: u32 = 19;
const FIRST_SESSION_TAB: u32 = 100;

pub const WINDOW: ElementId = ElementId::scoped(SHELL_SCOPE, 1);
pub const TITLEBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 2);
pub const TAB_CONTAINER_TOGGLE: ElementId = ElementId::scoped(SHELL_SCOPE, 12);
pub const TAB_CONTAINER: ElementId = ElementId::scoped(SHELL_SCOPE, 13);
pub const TAB_CONTAINER_SETTINGS_TAB: ElementId = ElementId::scoped(TAB_CONTAINER_SCOPE, 4);
pub const TITLEBAR_TAB_CONTAINER: ElementId = ElementId::scoped(SHELL_SCOPE, 53);
pub const TITLEBAR_SETTINGS_TAB: ElementId = ElementId::scoped(TITLEBAR_TAB_SCOPE, 4);
pub const WORKSPACE_PANE_TOGGLE: ElementId = ElementId::scoped(SHELL_SCOPE, 22);
pub const TAB_CONTAINER_TOOLBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 24);
pub const SESSION_SEARCH_INPUT: ElementId = ElementId::scoped(SHELL_SCOPE, 25);
pub const TAB_CONTAINER_ACTION_BAR: ElementId = ElementId::scoped(SHELL_SCOPE, 26);
pub const ADD_SESSION: ElementId = ElementId::scoped(SHELL_SCOPE, 27);
pub const FIRST_TAB_CONTAINER_SESSION_TAB: ElementId = ElementId::scoped(TAB_CONTAINER_SCOPE, 2);
pub const FIRST_TITLEBAR_SESSION_TAB: ElementId = ElementId::scoped(TITLEBAR_TAB_SCOPE, 2);
pub const TAB_CONTAINER_LIST: ElementId = ElementId::scoped(TAB_CONTAINER_GROUP_SCOPE, 1);
pub const TITLEBAR_TAB_LIST: ElementId = ElementId::scoped(TITLEBAR_TAB_GROUP_SCOPE, 1);

pub fn session_tab_id(index: usize) -> ElementId {
    if index == 0 {
        return FIRST_TAB_CONTAINER_SESSION_TAB;
    }
    dynamic_element_id(
        TAB_CONTAINER_SCOPE,
        FIRST_SESSION_TAB,
        index - 1,
        "session tab",
    )
}

pub fn titlebar_session_tab_id(index: usize) -> ElementId {
    if index == 0 {
        return FIRST_TITLEBAR_SESSION_TAB;
    }
    dynamic_element_id(
        TITLEBAR_TAB_SCOPE,
        FIRST_SESSION_TAB,
        index - 1,
        "titlebar session tab",
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

fn dynamic_element_id(scope: u32, first: u32, index: usize, label: &str) -> ElementId {
    let local = u32::try_from(index)
        .ok()
        .and_then(|index| first.checked_add(index))
        .unwrap_or_else(|| panic!("{label} index must fit its element scope"));
    ElementId::scoped(scope, local)
}

fn tab_group_element_id(scope: u32, group: TabGroupId, label: &str) -> ElementId {
    let local = u32::try_from(group.value())
        .unwrap_or_else(|_| panic!("{label} identity must fit its element scope"));
    ElementId::scoped(scope, local)
}
