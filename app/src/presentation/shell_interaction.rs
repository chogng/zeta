use std::ops::Range;
use zui::ui::ElementId;

pub(crate) use zeta_workbench::{
    ADD_SESSION, FIRST_TAB_CONTAINER_SESSION_TAB, SESSION_SEARCH_INPUT, TAB_CONTAINER_SETTINGS_TAB,
    TAB_CONTAINER_TOGGLE, TITLEBAR_SETTINGS_TAB, WINDOW, WORKSPACE_PANE_TOGGLE, session_tab_id,
    titlebar_session_tab_id,
};
#[cfg(test)]
pub(crate) use zeta_workbench::{
    FIRST_TITLEBAR_SESSION_TAB, TAB_CONTAINER_ACTION_BAR, TAB_CONTAINER_LIST,
    TAB_CONTAINER_TOOLBAR, TITLEBAR, TITLEBAR_TAB_LIST,
};

#[cfg(test)]
pub(crate) use zeta_session::interaction::SESSION_CONTEXT_MENU;
pub(crate) use zeta_session::interaction::SessionContextMenuAction;
#[cfg(test)]
pub(crate) use zeta_workspace_ui::interaction::{
    AGENT_CHANGES, AGENT_EDITOR_PANE, AGENT_FILES, AGENT_FILES_ACTION_BAR, AGENT_FILES_TOOLBAR,
    WORKSPACE_PANE_NAVIGATION,
};
pub(crate) use zeta_workspace_ui::interaction::{
    AGENT_EXPLORER_PANE, AGENT_FILE_SEARCH_INPUT, AGENT_FILES_REFRESH, AGENT_FILES_SEARCH,
    MULTI_DIFF_EDITOR, WORKSPACE_PANE, WORKSPACE_PANE_TOOLBAR,
};

pub(crate) use crate::workspace_panes::WorkspacePaneSelection;

const SHELL_SCOPE: u32 = 1;
const FILE_EDITOR_ACTION_SCOPE: u32 = 7;
#[cfg(test)]
const SESSION_CONTENT_SCOPE: u32 = 16;

pub(crate) const MAIN_SURFACE: ElementId = ElementId::scoped(SHELL_SCOPE, 3);
pub(crate) const TERMINAL_OUTPUT: ElementId = ElementId::scoped(SHELL_SCOPE, 4);
pub(crate) const COMPOSER_PANEL: ElementId = ElementId::scoped(SHELL_SCOPE, 5);
pub(crate) const COMPOSER: ElementId = ElementId::scoped(SHELL_SCOPE, 6);
pub(crate) const CONTEXT_TOOLBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 7);
pub(crate) const CONTEXT_LOCATION: ElementId = ElementId::scoped(SHELL_SCOPE, 8);
pub(crate) const CONTEXT_WORKING_DIRECTORY: ElementId = ElementId::scoped(SHELL_SCOPE, 9);
pub(crate) const CONTEXT_GIT_BRANCH: ElementId = ElementId::scoped(SHELL_SCOPE, 10);
pub(crate) const CONTEXT_DIFF: ElementId = ElementId::scoped(SHELL_SCOPE, 11);
pub(crate) const TAB_CONTAINER_RESIZE_HANDLE: ElementId = ElementId::scoped(SHELL_SCOPE, 16);
pub(crate) const INSPECTOR_RESIZE_HANDLE: ElementId = ElementId::scoped(SHELL_SCOPE, 51);
pub(crate) const THREAD_TIMELINE: ElementId = ElementId::scoped(SHELL_SCOPE, 40);
pub(crate) const COMPOSER_INTERACTION: ElementId = ElementId::scoped(SHELL_SCOPE, 42);
pub(crate) const COMPOSER_INFO_BAR: ElementId = ElementId::scoped(SHELL_SCOPE, 43);
pub(crate) const FILE_EDITOR_PANE: ElementId = ElementId::scoped(SHELL_SCOPE, 44);
pub(crate) const FILE_EDITOR_DOCUMENT: ElementId = ElementId::scoped(SHELL_SCOPE, 45);
pub(crate) const FILE_EDITOR_TAB_LIST: ElementId = ElementId::scoped(SHELL_SCOPE, 46);
pub(crate) const FILE_EDITOR_FIND_INPUT: ElementId = ElementId::scoped(SHELL_SCOPE, 47);
pub(crate) const FILE_EDITOR_REPLACE_INPUT: ElementId = ElementId::scoped(SHELL_SCOPE, 48);
pub(crate) const FILE_EDITOR_SEARCH_BAR: ElementId = ElementId::scoped(SHELL_SCOPE, 49);
#[cfg(test)]
pub(crate) const SESSION_HEADER: ElementId = ElementId::scoped(SESSION_CONTENT_SCOPE, 1);
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

pub(crate) fn session_tab_index(id: ElementId, mounted: Range<usize>) -> Option<usize> {
    mounted
        .clone()
        .find(|index| session_tab_id(*index) == id)
        .or_else(|| {
            mounted
                .into_iter()
                .find(|index| titlebar_session_tab_id(*index) == id)
        })
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
    dynamic_element_id_in_scope(SHELL_SCOPE, first, index, label)
}

fn dynamic_element_id_in_scope(scope: u32, first: u32, index: usize, label: &str) -> ElementId {
    let local = u32::try_from(index)
        .ok()
        .and_then(|index| first.checked_add(index))
        .unwrap_or_else(|| panic!("{label} index must fit its element scope"));
    ElementId::scoped(scope, local)
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

#[cfg(test)]
#[path = "shell_interaction_tests.rs"]
mod tests;
