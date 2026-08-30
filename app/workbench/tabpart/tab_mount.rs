//! Mapping from Workbench tab inputs into Tab Container UI identities.

use std::path::PathBuf;

use zui::ui::ElementId;

use super::identity::{
    FIRST_TAB_CONTAINER_SESSION_TAB, TAB_CONTAINER_SETTINGS_ACTION, TAB_CONTAINER_SETTINGS_CLOSE,
    TAB_CONTAINER_SETTINGS_TAB, TITLEBAR_SETTINGS_BUTTON, session_tab_action_id,
    session_tab_close_id, session_tab_id,
};
use crate::TabGroupId;
use crate::TabInput;
use crate::TabInputKey;
use crate::TabPart;
use crate::TabStatus;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WorkbenchTabKind {
    Session,
    Settings,
}

#[derive(Clone)]
pub struct WorkbenchTab<'a> {
    pub(super) id: ElementId,
    pub(super) action_id: ElementId,
    pub(super) close_id: ElementId,
    pub(super) kind: WorkbenchTabKind,
    pub(super) name: &'a str,
    pub(super) location: &'a str,
    pub(super) dirs: &'a [PathBuf],
    pub(super) status: TabStatus,
    pub(super) pinned: bool,
}

#[derive(Clone)]
pub struct TabGroup<'a> {
    pub(super) id: TabGroupId,
    pub(super) label: Option<&'a str>,
    pub(super) collapsed: bool,
    pub(super) tabs: Vec<WorkbenchTab<'a>>,
}

impl<'a> TabGroup<'a> {
    pub fn new(
        id: TabGroupId,
        label: Option<&'a str>,
        collapsed: bool,
        tabs: Vec<WorkbenchTab<'a>>,
    ) -> Self {
        Self {
            id,
            label,
            collapsed,
            tabs,
        }
    }
}

/// Resolves one presentation's element identity without leaking UI identity into Workbench state.
pub fn tab_input_element_id(tab_part: &TabPart, selected: Option<&TabInputKey>) -> ElementId {
    let Some(selected) = selected else {
        return FIRST_TAB_CONTAINER_SESSION_TAB;
    };
    mounted_tab_element_id(tab_part, selected).unwrap_or(FIRST_TAB_CONTAINER_SESSION_TAB)
}

/// Resolves one mounted tab without substituting another tab when the input is absent.
pub fn mounted_tab_element_id(tab_part: &TabPart, tab: &TabInputKey) -> Option<ElementId> {
    if tab.is_settings() {
        return tab_part.input(tab).map(|_| TAB_CONTAINER_SETTINGS_TAB);
    }
    tab_part.tab_id(tab).map(session_tab_id)
}

/// Workbench-owned action resolved from one mounted tab element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TabIntent {
    Activate(TabInputKey),
    OpenActions(TabInputKey),
    Close(TabInputKey),
}

/// Resolves a tab or close-button identity without depending on current tab order.
pub fn tab_intent_for_element(tab_part: &TabPart, element: ElementId) -> Option<TabIntent> {
    if element == TITLEBAR_SETTINGS_BUTTON {
        return Some(TabIntent::Activate(TabInputKey::Settings));
    }
    if element == TAB_CONTAINER_SETTINGS_ACTION {
        return tab_part
            .input(&TabInputKey::Settings)
            .map(|_| TabIntent::OpenActions(TabInputKey::Settings));
    }
    if element == TAB_CONTAINER_SETTINGS_CLOSE {
        return tab_part
            .input(&TabInputKey::Settings)
            .map(|_| TabIntent::Close(TabInputKey::Settings));
    }
    if element == TAB_CONTAINER_SETTINGS_TAB {
        return Some(TabIntent::Activate(TabInputKey::Settings));
    }
    for input in tab_part.inputs().filter(|input| input.is_session()) {
        let tab_id = tab_part
            .tab_id(input.key())
            .expect("mounted Session TabInput must have a TabId");
        if element == session_tab_id(tab_id) {
            return Some(TabIntent::Activate(input.key().clone()));
        }
        if element == session_tab_action_id(tab_id) {
            return Some(TabIntent::OpenActions(input.key().clone()));
        }
        if element == session_tab_close_id(tab_id) {
            return Some(TabIntent::Close(input.key().clone()));
        }
    }
    None
}

/// Resolves the Session tab owning a mounted tab element.
pub fn tab_key_for_element(tab_part: &TabPart, element: ElementId) -> Option<&TabInputKey> {
    tab_part.inputs().find_map(|input| {
        if input.is_settings() {
            return (TAB_CONTAINER_SETTINGS_TAB == element
                || TAB_CONTAINER_SETTINGS_ACTION == element)
                .then_some(input.key());
        }
        let tab_id = tab_part.tab_id(input.key())?;
        (session_tab_id(tab_id) == element || session_tab_action_id(tab_id) == element)
            .then_some(input.key())
    })
}

pub fn workbench_tab_groups<'a>(
    tab_part: &'a TabPart,
    include: impl Fn(&TabInput) -> bool,
) -> Vec<TabGroup<'a>> {
    tab_part
        .groups()
        .iter()
        .filter_map(|group| {
            let tabs = group
                .inputs()
                .iter()
                .filter(|input| include(input))
                .map(|input| WorkbenchTab::from_input(tab_part, input))
                .collect::<Vec<_>>();
            (!tabs.is_empty())
                .then(|| TabGroup::new(group.id(), group.label(), group.is_collapsed(), tabs))
        })
        .collect()
}

impl<'a> WorkbenchTab<'a> {
    pub fn from_input(tab_part: &'a TabPart, input: &'a TabInput) -> Self {
        if input.is_settings() {
            Self::settings(
                TAB_CONTAINER_SETTINGS_TAB,
                TAB_CONTAINER_SETTINGS_ACTION,
                TAB_CONTAINER_SETTINGS_CLOSE,
                tab_part.tab_name(input),
            )
        } else {
            let tab_id = tab_part
                .tab_id(input.key())
                .expect("mounted Session TabInput must have a TabId");
            Self::new(
                session_tab_id(tab_id),
                session_tab_action_id(tab_id),
                session_tab_close_id(tab_id),
                tab_part.tab_name(input),
                input.location(),
                input.status().clone(),
                tab_part.is_tab_pinned(input.key()),
            )
            .with_dirs(input.dirs())
        }
    }

    pub fn new(
        id: ElementId,
        action_id: ElementId,
        close_id: ElementId,
        name: &'a str,
        location: &'a str,
        status: TabStatus,
        pinned: bool,
    ) -> Self {
        Self {
            id,
            action_id,
            close_id,
            kind: WorkbenchTabKind::Session,
            name,
            location,
            dirs: &[],
            status,
            pinned,
        }
    }

    pub fn with_dirs(mut self, dirs: &'a [PathBuf]) -> Self {
        self.dirs = dirs;
        self
    }

    pub fn settings(
        id: ElementId,
        action_id: ElementId,
        close_id: ElementId,
        name: &'a str,
    ) -> Self {
        Self {
            id,
            action_id,
            close_id,
            kind: WorkbenchTabKind::Settings,
            name,
            location: "Application",
            dirs: &[],
            status: TabStatus::idle("Settings"),
            pinned: false,
        }
    }
}
