use std::ops::Range;
use zeta_ui_dispatch::ElementId;

const SHELL_SCOPE: u32 = 1;

pub(crate) const WINDOW: ElementId = ElementId::scoped(SHELL_SCOPE, 1);
pub(crate) const TITLEBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 2);
pub(crate) const MAIN_SURFACE: ElementId = ElementId::scoped(SHELL_SCOPE, 3);
pub(crate) const TERMINAL_OUTPUT: ElementId = ElementId::scoped(SHELL_SCOPE, 4);
pub(crate) const COMPOSER_PANEL: ElementId = ElementId::scoped(SHELL_SCOPE, 5);
pub(crate) const COMPOSER: ElementId = ElementId::scoped(SHELL_SCOPE, 6);
pub(crate) const CONTEXT_TOOLBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 7);
pub(crate) const CONTEXT_LOCATION: ElementId = ElementId::scoped(SHELL_SCOPE, 8);
pub(crate) const CONTEXT_WORKING_DIRECTORY: ElementId = ElementId::scoped(SHELL_SCOPE, 9);
pub(crate) const CONTEXT_GIT_BRANCH: ElementId = ElementId::scoped(SHELL_SCOPE, 10);
pub(crate) const CONTEXT_DIFF: ElementId = ElementId::scoped(SHELL_SCOPE, 11);
pub(crate) const SESSION_SIDEBAR_TOGGLE: ElementId = ElementId::scoped(SHELL_SCOPE, 12);
pub(crate) const SESSION_SIDEBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 13);
pub(crate) const SESSION_TAB_LIST: ElementId = ElementId::scoped(SHELL_SCOPE, 14);
pub(crate) const ACTIVE_SESSION_TAB: ElementId = ElementId::scoped(SHELL_SCOPE, 15);
pub(crate) const SESSION_SIDEBAR_RESIZE_HANDLE: ElementId = ElementId::scoped(SHELL_SCOPE, 16);
pub(crate) const SESSION_CONTEXT_MENU: ElementId = ElementId::scoped(SHELL_SCOPE, 17);
const SESSION_CONTEXT_MENU_PIN: ElementId = ElementId::scoped(SHELL_SCOPE, 18);
const SESSION_CONTEXT_MENU_CLOSE: ElementId = ElementId::scoped(SHELL_SCOPE, 19);
const SESSION_CONTEXT_MENU_RENAME: ElementId = ElementId::scoped(SHELL_SCOPE, 20);
const SESSION_CONTEXT_MENU_FORK: ElementId = ElementId::scoped(SHELL_SCOPE, 21);
pub(crate) const AGENT_SIDEBAR_TOGGLE: ElementId = ElementId::scoped(SHELL_SCOPE, 22);
pub(crate) const AGENT_SIDEBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 23);
pub(crate) const SESSION_SIDEBAR_TOOLBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 24);
pub(crate) const SESSION_SEARCH_INPUT: ElementId = ElementId::scoped(SHELL_SCOPE, 25);
pub(crate) const SESSION_SIDEBAR_ACTION_BAR: ElementId = ElementId::scoped(SHELL_SCOPE, 26);
pub(crate) const ADD_SESSION: ElementId = ElementId::scoped(SHELL_SCOPE, 27);
pub(crate) const AGENT_EXPLORER_PANE: ElementId = ElementId::scoped(SHELL_SCOPE, 28);
pub(crate) const AGENT_EDITOR_PANE: ElementId = ElementId::scoped(SHELL_SCOPE, 29);
pub(crate) const MULTI_DIFF_EDITOR: ElementId = ElementId::scoped(SHELL_SCOPE, 30);
pub(crate) const MULTI_DIFF_SCROLLBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 31);
pub(crate) const AGENT_SIDEBAR_NAVIGATION: ElementId = ElementId::scoped(SHELL_SCOPE, 32);
pub(crate) const AGENT_CHANGES: ElementId = ElementId::scoped(SHELL_SCOPE, 33);
pub(crate) const AGENT_FILES: ElementId = ElementId::scoped(SHELL_SCOPE, 34);
pub(crate) const AGENT_SIDEBAR_TOOLBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 35);
pub(crate) const AGENT_FILES_ACTION_BAR: ElementId = ElementId::scoped(SHELL_SCOPE, 36);
pub(crate) const AGENT_FILES_REFRESH: ElementId = ElementId::scoped(SHELL_SCOPE, 37);
pub(crate) const AGENT_FILES_SEARCH: ElementId = ElementId::scoped(SHELL_SCOPE, 38);
pub(crate) const AGENT_FILE_SEARCH_INPUT: ElementId = ElementId::scoped(SHELL_SCOPE, 39);
pub(crate) const THREAD_TIMELINE: ElementId = ElementId::scoped(SHELL_SCOPE, 40);
pub(crate) const COMPOSER_MODE: ElementId = ElementId::scoped(SHELL_SCOPE, 41);
pub(crate) const COMPOSER_INTERACTION: ElementId = ElementId::scoped(SHELL_SCOPE, 42);
pub(crate) const COMPOSER_INFO_BAR: ElementId = ElementId::scoped(SHELL_SCOPE, 43);
const FIRST_COMPOSER_INTERACTION_ITEM: u32 = 100;

pub(crate) fn composer_interaction_item_id(index: usize) -> ElementId {
    let local = u32::try_from(index)
        .ok()
        .and_then(|index| FIRST_COMPOSER_INTERACTION_ITEM.checked_add(index))
        .expect("composer interaction item index must fit its element scope");
    ElementId::scoped(SHELL_SCOPE, local)
}

pub(crate) fn composer_interaction_item_index(
    id: ElementId,
    mut visible_range: Range<usize>,
) -> Option<usize> {
    visible_range.find(|index| composer_interaction_item_id(*index) == id)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AgentSidebarPaneAction {
    Changes,
    Files,
}

impl AgentSidebarPaneAction {
    pub(crate) const ALL: [Self; 2] = [Self::Changes, Self::Files];

    pub(crate) const fn element_id(self) -> ElementId {
        match self {
            Self::Changes => AGENT_CHANGES,
            Self::Files => AGENT_FILES,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Changes => "Changes",
            Self::Files => "Files",
        }
    }

    pub(crate) const fn view(self) -> crate::agent_sidebar_workspace::AgentSidebarView {
        match self {
            Self::Changes => crate::agent_sidebar_workspace::AgentSidebarView::Changes,
            Self::Files => crate::agent_sidebar_workspace::AgentSidebarView::Files,
        }
    }

    pub(crate) const fn from_element_id(id: ElementId) -> Option<Self> {
        match id {
            AGENT_CHANGES => Some(Self::Changes),
            AGENT_FILES => Some(Self::Files),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContextAction {
    Location,
    WorkingDirectory,
    GitBranch,
    Diff,
}

impl ContextAction {
    pub(crate) const ALL: [Self; 4] = [
        Self::Location,
        Self::WorkingDirectory,
        Self::GitBranch,
        Self::Diff,
    ];

    pub(crate) const fn element_id(self) -> ElementId {
        match self {
            Self::Location => CONTEXT_LOCATION,
            Self::WorkingDirectory => CONTEXT_WORKING_DIRECTORY,
            Self::GitBranch => CONTEXT_GIT_BRANCH,
            Self::Diff => CONTEXT_DIFF,
        }
    }

    pub(crate) const fn from_element_id(id: ElementId) -> Option<Self> {
        match id {
            CONTEXT_LOCATION => Some(Self::Location),
            CONTEXT_WORKING_DIRECTORY => Some(Self::WorkingDirectory),
            CONTEXT_GIT_BRANCH => Some(Self::GitBranch),
            CONTEXT_DIFF => Some(Self::Diff),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionContextMenuAction {
    Pin,
    Close,
    Rename,
    Fork,
}

impl SessionContextMenuAction {
    pub(crate) const ALL: [Self; 4] = [Self::Pin, Self::Close, Self::Rename, Self::Fork];

    pub(crate) const fn element_id(self) -> ElementId {
        match self {
            Self::Pin => SESSION_CONTEXT_MENU_PIN,
            Self::Close => SESSION_CONTEXT_MENU_CLOSE,
            Self::Rename => SESSION_CONTEXT_MENU_RENAME,
            Self::Fork => SESSION_CONTEXT_MENU_FORK,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Pin => "Pin",
            Self::Close => "Close",
            Self::Rename => "Rename",
            Self::Fork => "Fork",
        }
    }

    pub(crate) const fn from_element_id(id: ElementId) -> Option<Self> {
        match id {
            SESSION_CONTEXT_MENU_PIN => Some(Self::Pin),
            SESSION_CONTEXT_MENU_CLOSE => Some(Self::Close),
            SESSION_CONTEXT_MENU_RENAME => Some(Self::Rename),
            SESSION_CONTEXT_MENU_FORK => Some(Self::Fork),
            _ => None,
        }
    }

    pub(crate) fn is_menu_element(id: ElementId) -> bool {
        id == SESSION_CONTEXT_MENU || Self::from_element_id(id).is_some()
    }
}

#[cfg(test)]
#[path = "shell_interaction_tests.rs"]
mod tests;
