use crate::{Icon, IconId, generated::artwork};

/// Stable semantic product icons available to callers.
///
/// Constants in this module are maintained explicitly. Their identities do not
/// depend on SVG filenames, and multiple identities may intentionally share one
/// artwork definition.
///
/// Keep every semantic ID-to-artwork mapping on one line so the catalog remains
/// directly scannable.
#[rustfmt::skip]
pub mod icons {
    use super::{Icon, IconId, artwork};

    pub const ADD: Icon = Icon::new(IconId::new("add"), artwork::ADD);
    pub const AGENT: Icon = Icon::new(IconId::new("agent"), artwork::AGENT);
    pub const ARROW_UP: Icon = Icon::new(IconId::new("arrow-up"), artwork::ARROW_UP);
    pub const BROWSER_WEB: Icon = Icon::new(IconId::new("browser-web"), artwork::BROWSER_WEB);
    pub const CHAT: Icon = Icon::new(IconId::new("chat"), artwork::CHAT);
    pub const CHAT_FILLED: Icon = Icon::new(IconId::new("chat-filled"), artwork::CHAT_FILLED);
    pub const CHEVRON_DOWN: Icon = Icon::new(IconId::new("chevron-down"), artwork::CHEVRON_DOWN);
    pub const CHEVRON_RIGHT: Icon = Icon::new(IconId::new("chevron-right"), artwork::CHEVRON_RIGHT);
    pub const CLOSE: Icon = Icon::new(IconId::new("close"), artwork::CLOSE);
    pub const DIFF: Icon = Icon::new(IconId::new("diff"), artwork::GIT);
    pub const DROPDOWN_INDICATOR: Icon = Icon::new(IconId::new("dropdown-indicator"), artwork::CHEVRON_DOWN);
    pub const ELLIPSIS: Icon = Icon::new(IconId::new("ellipsis"), artwork::ELLIPSIS);
    pub const FILES: Icon = Icon::new(IconId::new("files"), artwork::FILES);
    pub const GEAR: Icon = Icon::new(IconId::new("gear"), artwork::GEAR);
    pub const GIT_BRANCH: Icon = Icon::new(IconId::new("git-branch"), artwork::GIT_BRANCH);
    pub const GIT_COMMIT: Icon = Icon::new(IconId::new("git-commit"), artwork::GIT_COMMIT);
    pub const HISTORY: Icon = Icon::new(IconId::new("history"), artwork::REFRESH);
    pub const LAYOUT_PANEL: Icon = Icon::new(IconId::new("layout-panel"), artwork::LAYOUT_PANEL);
    pub const LAYOUT_PANEL_OFF: Icon = Icon::new(IconId::new("layout-panel-off"), artwork::LAYOUT_PANEL_OFF);
    pub const LAYOUT_SIDEBAR_LEFT: Icon = Icon::new(IconId::new("layout-sidebar-left"), artwork::LAYOUT_SIDEBAR_LEFT);
    pub const LAYOUT_SIDEBAR_LEFT_EMPTY: Icon = Icon::new(IconId::new("layout-sidebar-left-empty"), artwork::LAYOUT_SIDEBAR_LEFT_EMPTY);
    pub const LAYOUT_SIDEBAR_LEFT_OFF: Icon = Icon::new(IconId::new("layout-sidebar-left-off"), artwork::LAYOUT_SIDEBAR_LEFT_OFF);
    pub const LAYOUT_SIDEBAR_RIGHT: Icon = Icon::new(IconId::new("layout-sidebar-right"), artwork::LAYOUT_SIDEBAR_RIGHT);
    pub const LAYOUT_SIDEBAR_RIGHT_EMPTY: Icon = Icon::new(IconId::new("layout-sidebar-right-empty"), artwork::LAYOUT_SIDEBAR_RIGHT_EMPTY);
    pub const LAYOUT_SIDEBAR_RIGHT_OFF: Icon = Icon::new(IconId::new("layout-sidebar-right-off"), artwork::LAYOUT_SIDEBAR_RIGHT_OFF);
    pub const LINK_EXTERNAL: Icon = Icon::new(IconId::new("link-external"), artwork::LINK_EXTERNAL);
    pub const LOCAL: Icon = Icon::new(IconId::new("local"), artwork::TERMINAL);
    pub const MENU: Icon = Icon::new(IconId::new("menu"), artwork::MENU);
    pub const MODEL: Icon = Icon::new(IconId::new("model"), artwork::MODEL);
    pub const SEARCH: Icon = Icon::new(IconId::new("search"), artwork::SEARCH);
    pub const SETTINGS: Icon = Icon::new(IconId::new("settings"), artwork::SETTINGS);
    pub const SPLIT_HORIZONTAL: Icon = Icon::new(IconId::new("split-horizontal"), artwork::SPLIT_HORIZONTAL);
    pub const START: Icon = Icon::new(IconId::new("start"), artwork::START);
    pub const SUBMENU_INDICATOR: Icon = Icon::new(IconId::new("submenu-indicator"), artwork::CHEVRON_RIGHT);
    pub const WORKING_DIRECTORY: Icon = Icon::new(IconId::new("working-directory"), artwork::NEW_FOLDER);
}

/// Sorted semantic icon catalog used by [`crate::icon_by_id`].
pub const ALL_ICONS: &[Icon] = &[
    icons::ADD,
    icons::AGENT,
    icons::ARROW_UP,
    icons::BROWSER_WEB,
    icons::CHAT,
    icons::CHAT_FILLED,
    icons::CHEVRON_DOWN,
    icons::CHEVRON_RIGHT,
    icons::CLOSE,
    icons::DIFF,
    icons::DROPDOWN_INDICATOR,
    icons::ELLIPSIS,
    icons::FILES,
    icons::GEAR,
    icons::GIT_BRANCH,
    icons::GIT_COMMIT,
    icons::HISTORY,
    icons::LAYOUT_PANEL,
    icons::LAYOUT_PANEL_OFF,
    icons::LAYOUT_SIDEBAR_LEFT,
    icons::LAYOUT_SIDEBAR_LEFT_EMPTY,
    icons::LAYOUT_SIDEBAR_LEFT_OFF,
    icons::LAYOUT_SIDEBAR_RIGHT,
    icons::LAYOUT_SIDEBAR_RIGHT_EMPTY,
    icons::LAYOUT_SIDEBAR_RIGHT_OFF,
    icons::LINK_EXTERNAL,
    icons::LOCAL,
    icons::MENU,
    icons::MODEL,
    icons::SEARCH,
    icons::SETTINGS,
    icons::SPLIT_HORIZONTAL,
    icons::START,
    icons::SUBMENU_INDICATOR,
    icons::WORKING_DIRECTORY,
];
