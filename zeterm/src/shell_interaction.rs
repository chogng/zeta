use std::ops::Range;
use zui::ElementId;

#[cfg(test)]
pub(crate) use zeta_agent_sidebar::AGENT_CHANGES;
#[cfg(test)]
pub(crate) use zeta_agent_sidebar::AGENT_EDITOR_PANE;
pub(crate) use zeta_agent_sidebar::AGENT_EXPLORER_PANE;
pub(crate) use zeta_agent_sidebar::AGENT_FILE_SEARCH_INPUT;
#[cfg(test)]
pub(crate) use zeta_agent_sidebar::AGENT_FILES;
#[cfg(test)]
pub(crate) use zeta_agent_sidebar::AGENT_FILES_ACTION_BAR;
pub(crate) use zeta_agent_sidebar::AGENT_FILES_REFRESH;
pub(crate) use zeta_agent_sidebar::AGENT_FILES_SEARCH;
pub(crate) use zeta_agent_sidebar::AGENT_SIDEBAR;
#[cfg(test)]
pub(crate) use zeta_agent_sidebar::AGENT_SIDEBAR_NAVIGATION;
pub(crate) use zeta_agent_sidebar::AGENT_SIDEBAR_TOOLBAR;
pub(crate) use zeta_agent_sidebar::AgentSidebarPaneAction;
pub(crate) use zeta_agent_sidebar::MULTI_DIFF_EDITOR;

const SHELL_SCOPE: u32 = 1;
const FILE_EDITOR_ACTION_SCOPE: u32 = 7;

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
pub(crate) const AGENT_SIDEBAR_RESIZE_HANDLE: ElementId = ElementId::scoped(SHELL_SCOPE, 51);
pub(crate) const SESSION_SIDEBAR_TOOLBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 24);
pub(crate) const SESSION_SEARCH_INPUT: ElementId = ElementId::scoped(SHELL_SCOPE, 25);
pub(crate) const SESSION_SIDEBAR_ACTION_BAR: ElementId = ElementId::scoped(SHELL_SCOPE, 26);
pub(crate) const ADD_SESSION: ElementId = ElementId::scoped(SHELL_SCOPE, 27);
pub(crate) const THREAD_TIMELINE: ElementId = ElementId::scoped(SHELL_SCOPE, 40);
pub(crate) const COMPOSER_MODE: ElementId = ElementId::scoped(SHELL_SCOPE, 41);
pub(crate) const COMPOSER_INTERACTION: ElementId = ElementId::scoped(SHELL_SCOPE, 42);
pub(crate) const COMPOSER_INFO_BAR: ElementId = ElementId::scoped(SHELL_SCOPE, 43);
pub(crate) const FILE_EDITOR_PANE: ElementId = ElementId::scoped(SHELL_SCOPE, 44);
pub(crate) const FILE_EDITOR_DOCUMENT: ElementId = ElementId::scoped(SHELL_SCOPE, 45);
pub(crate) const FILE_EDITOR_TAB_LIST: ElementId = ElementId::scoped(SHELL_SCOPE, 46);
pub(crate) const FILE_EDITOR_FIND_INPUT: ElementId = ElementId::scoped(SHELL_SCOPE, 47);
pub(crate) const FILE_EDITOR_REPLACE_INPUT: ElementId = ElementId::scoped(SHELL_SCOPE, 48);
pub(crate) const FILE_EDITOR_SEARCH_BAR: ElementId = ElementId::scoped(SHELL_SCOPE, 49);
pub(crate) const LANGUAGE_SERVER_SETTINGS_TOGGLE: ElementId = ElementId::scoped(SHELL_SCOPE, 50);
pub(crate) const FILE_EDITOR_NOTICE: ElementId = ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, 1);
const FILE_EDITOR_RELOAD: ElementId = ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, 2);
const FILE_EDITOR_OVERWRITE: ElementId = ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, 3);
const FILE_EDITOR_SAVE_AND_CLOSE: ElementId = ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, 4);
const FILE_EDITOR_DISCARD_AND_CLOSE: ElementId = ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, 5);
const FILE_EDITOR_CANCEL_CLOSE: ElementId = ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, 6);
const FILE_EDITOR_FIND_PREVIOUS: ElementId = ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, 7);
const FILE_EDITOR_FIND_NEXT: ElementId = ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, 8);
const FILE_EDITOR_REPLACE_CURRENT: ElementId = ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, 9);
const FILE_EDITOR_REPLACE_ALL: ElementId = ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, 10);
const FILE_EDITOR_CLOSE_SEARCH: ElementId = ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, 11);
const FIRST_COMPOSER_INTERACTION_ITEM: u32 = 100;
const FIRST_FILE_EDITOR_TAB: u32 = 200;
const FIRST_FILE_EDITOR_FOLD: u32 = 1_000;
const FIRST_FILE_EDITOR_CLOSE: u32 = 100;

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

pub(crate) fn file_editor_tab_id(index: usize) -> ElementId {
    dynamic_element_id(FIRST_FILE_EDITOR_TAB, index, "file editor tab")
}

pub(crate) fn file_editor_tab_index(id: ElementId, mut mounted: Range<usize>) -> Option<usize> {
    mounted.find(|index| file_editor_tab_id(*index) == id)
}

pub(crate) fn file_editor_close_id(index: usize) -> ElementId {
    file_editor_action_element_id(FIRST_FILE_EDITOR_CLOSE, index, "file editor close button")
}

pub(crate) fn file_editor_close_index(id: ElementId, mut mounted: Range<usize>) -> Option<usize> {
    mounted.find(|index| file_editor_close_id(*index) == id)
}

pub(crate) fn file_editor_fold_id(index: usize) -> ElementId {
    dynamic_element_id(FIRST_FILE_EDITOR_FOLD, index, "file editor fold")
}

pub(crate) fn file_editor_fold_index(id: ElementId, mut mounted: Range<usize>) -> Option<usize> {
    mounted.find(|index| file_editor_fold_id(*index) == id)
}

fn dynamic_element_id(first: u32, index: usize, label: &str) -> ElementId {
    let local = u32::try_from(index)
        .ok()
        .and_then(|index| first.checked_add(index))
        .unwrap_or_else(|| panic!("{label} index must fit its element scope"));
    ElementId::scoped(SHELL_SCOPE, local)
}

fn file_editor_action_element_id(first: u32, index: usize, label: &str) -> ElementId {
    let local = u32::try_from(index)
        .ok()
        .and_then(|index| first.checked_add(index))
        .unwrap_or_else(|| panic!("{label} index must fit its element scope"));
    ElementId::scoped(FILE_EDITOR_ACTION_SCOPE, local)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FileEditorAction {
    Reload,
    Overwrite,
    SaveAndClose,
    DiscardAndClose,
    CancelClose,
    FindPrevious,
    FindNext,
    ReplaceCurrent,
    ReplaceAll,
    CloseSearch,
}

impl FileEditorAction {
    pub(crate) const fn element_id(self) -> ElementId {
        match self {
            Self::Reload => FILE_EDITOR_RELOAD,
            Self::Overwrite => FILE_EDITOR_OVERWRITE,
            Self::SaveAndClose => FILE_EDITOR_SAVE_AND_CLOSE,
            Self::DiscardAndClose => FILE_EDITOR_DISCARD_AND_CLOSE,
            Self::CancelClose => FILE_EDITOR_CANCEL_CLOSE,
            Self::FindPrevious => FILE_EDITOR_FIND_PREVIOUS,
            Self::FindNext => FILE_EDITOR_FIND_NEXT,
            Self::ReplaceCurrent => FILE_EDITOR_REPLACE_CURRENT,
            Self::ReplaceAll => FILE_EDITOR_REPLACE_ALL,
            Self::CloseSearch => FILE_EDITOR_CLOSE_SEARCH,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Reload => "Reload from Disk",
            Self::Overwrite => "Overwrite",
            Self::SaveAndClose => "Save",
            Self::DiscardAndClose => "Don't Save",
            Self::CancelClose => "Cancel",
            Self::FindPrevious => "Previous",
            Self::FindNext => "Next",
            Self::ReplaceCurrent => "Replace",
            Self::ReplaceAll => "Replace All",
            Self::CloseSearch => "Close",
        }
    }

    pub(crate) const fn from_element_id(id: ElementId) -> Option<Self> {
        match id {
            FILE_EDITOR_RELOAD => Some(Self::Reload),
            FILE_EDITOR_OVERWRITE => Some(Self::Overwrite),
            FILE_EDITOR_SAVE_AND_CLOSE => Some(Self::SaveAndClose),
            FILE_EDITOR_DISCARD_AND_CLOSE => Some(Self::DiscardAndClose),
            FILE_EDITOR_CANCEL_CLOSE => Some(Self::CancelClose),
            FILE_EDITOR_FIND_PREVIOUS => Some(Self::FindPrevious),
            FILE_EDITOR_FIND_NEXT => Some(Self::FindNext),
            FILE_EDITOR_REPLACE_CURRENT => Some(Self::ReplaceCurrent),
            FILE_EDITOR_REPLACE_ALL => Some(Self::ReplaceAll),
            FILE_EDITOR_CLOSE_SEARCH => Some(Self::CloseSearch),
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
