//! Mapping from orientation-neutral Workbench tabs into one UI identity scope.

use std::path::PathBuf;

use crate::TabListOrientation;
use zui::ui::ElementId;
use zui::ui::NavigationAxis;

use super::identity::{
    FIRST_TAB_CONTAINER_SESSION_TAB, FIRST_TITLEBAR_SESSION_TAB, TAB_CONTAINER,
    TAB_CONTAINER_SETTINGS_ACTION, TAB_CONTAINER_SETTINGS_CLOSE, TAB_CONTAINER_SETTINGS_TAB,
    TITLEBAR, TITLEBAR_SETTINGS_ACTION, TITLEBAR_SETTINGS_BUTTON, TITLEBAR_SETTINGS_CLOSE,
    TITLEBAR_SETTINGS_TAB, TITLEBAR_TAB_CONTAINER, WINDOW, session_tab_action_id,
    session_tab_close_id, session_tab_id, tab_group_list_id, titlebar_session_tab_action_id,
    titlebar_session_tab_close_id, titlebar_session_tab_id, titlebar_tab_group_list_id,
};
use crate::TabGroupId;
use crate::TabId;
use crate::TabInput;
use crate::TabInputKey;
use crate::TabPart;
use crate::TabStatus;

/// UI mount that projects the same logical Tab Part into a concrete Workbench location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabContainerPlacement {
    Body,
    Titlebar,
}

impl TabContainerPlacement {
    pub(super) const fn orientation(self) -> TabListOrientation {
        match self {
            Self::Body => TabListOrientation::Vertical,
            Self::Titlebar => TabListOrientation::Horizontal,
        }
    }

    pub(super) const fn navigation_axis(self) -> NavigationAxis {
        match self {
            Self::Body => NavigationAxis::Vertical,
            Self::Titlebar => NavigationAxis::Horizontal,
        }
    }

    pub(super) const fn container_id(self) -> ElementId {
        match self {
            Self::Body => TAB_CONTAINER,
            Self::Titlebar => TITLEBAR_TAB_CONTAINER,
        }
    }

    pub(super) const fn parent_id(self) -> ElementId {
        match self {
            Self::Body => WINDOW,
            Self::Titlebar => TITLEBAR,
        }
    }

    pub(super) const fn settings_id(self) -> ElementId {
        match self {
            Self::Body => TAB_CONTAINER_SETTINGS_TAB,
            Self::Titlebar => TITLEBAR_SETTINGS_TAB,
        }
    }

    pub(super) const fn settings_close_id(self) -> ElementId {
        match self {
            Self::Body => TAB_CONTAINER_SETTINGS_CLOSE,
            Self::Titlebar => TITLEBAR_SETTINGS_CLOSE,
        }
    }

    pub(super) const fn settings_action_id(self) -> ElementId {
        match self {
            Self::Body => TAB_CONTAINER_SETTINGS_ACTION,
            Self::Titlebar => TITLEBAR_SETTINGS_ACTION,
        }
    }

    pub(super) fn session_id(self, id: TabId) -> ElementId {
        match self {
            Self::Body => session_tab_id(id),
            Self::Titlebar => titlebar_session_tab_id(id),
        }
    }

    pub(super) fn close_id(self, id: TabId) -> ElementId {
        match self {
            Self::Body => session_tab_close_id(id),
            Self::Titlebar => titlebar_session_tab_close_id(id),
        }
    }

    pub(super) fn action_id(self, id: TabId) -> ElementId {
        match self {
            Self::Body => session_tab_action_id(id),
            Self::Titlebar => titlebar_session_tab_action_id(id),
        }
    }

    pub(super) fn group_list_id(self, group: TabGroupId) -> ElementId {
        match self {
            Self::Body => tab_group_list_id(group),
            Self::Titlebar => titlebar_tab_group_list_id(group),
        }
    }
}

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
pub struct WorkbenchTabGroup<'a> {
    pub(super) id: TabGroupId,
    pub(super) label: Option<&'a str>,
    pub(super) collapsed: bool,
    pub(super) tabs: Vec<WorkbenchTab<'a>>,
}

impl<'a> WorkbenchTabGroup<'a> {
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

    pub const fn id(&self) -> TabGroupId {
        self.id
    }

    pub fn insert_tab(&mut self, index: usize, tab: WorkbenchTab<'a>) {
        self.tabs.insert(index.min(self.tabs.len()), tab);
    }
}

/// Resolves one presentation's element identity without leaking UI identity into Workbench state.
pub fn tab_input_element_id(
    tab_part: &TabPart,
    selected: Option<&TabInputKey>,
    placement: TabContainerPlacement,
) -> ElementId {
    let Some(selected) = selected else {
        return first_session_id(placement);
    };
    mounted_tab_element_id(tab_part, selected, placement)
        .unwrap_or_else(|| first_session_id(placement))
}

/// Resolves one mounted tab without substituting another tab when the input is absent.
pub fn mounted_tab_element_id(
    tab_part: &TabPart,
    tab: &TabInputKey,
    placement: TabContainerPlacement,
) -> Option<ElementId> {
    if tab.is_settings() {
        return tab_part.input(tab).map(|_| placement.settings_id());
    }
    tab_part.tab_id(tab).map(|id| placement.session_id(id))
}

fn first_session_id(placement: TabContainerPlacement) -> ElementId {
    match placement {
        TabContainerPlacement::Body => FIRST_TAB_CONTAINER_SESSION_TAB,
        TabContainerPlacement::Titlebar => FIRST_TITLEBAR_SESSION_TAB,
    }
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
    for placement in [TabContainerPlacement::Body, TabContainerPlacement::Titlebar] {
        if element == placement.settings_action_id() {
            return tab_part
                .input(&TabInputKey::Settings)
                .map(|_| TabIntent::OpenActions(TabInputKey::Settings));
        }
    }
    if element == TabContainerPlacement::Body.settings_close_id()
        || element == TabContainerPlacement::Titlebar.settings_close_id()
    {
        return tab_part
            .input(&TabInputKey::Settings)
            .map(|_| TabIntent::Close(TabInputKey::Settings));
    }
    if element == TabContainerPlacement::Body.settings_id()
        || element == TabContainerPlacement::Titlebar.settings_id()
    {
        return Some(TabIntent::Activate(TabInputKey::Settings));
    }
    for input in tab_part.inputs().filter(|input| input.is_session()) {
        let tab_id = tab_part
            .tab_id(input.key())
            .expect("mounted Session TabInput must have a TabId");
        for placement in [TabContainerPlacement::Body, TabContainerPlacement::Titlebar] {
            if element == placement.session_id(tab_id) {
                return Some(TabIntent::Activate(input.key().clone()));
            }
            if element == placement.action_id(tab_id) {
                return Some(TabIntent::OpenActions(input.key().clone()));
            }
            if element == placement.close_id(tab_id) {
                return Some(TabIntent::Close(input.key().clone()));
            }
        }
    }
    None
}

/// Resolves the Session tab owning a mounted tab element.
pub fn tab_key_for_element(tab_part: &TabPart, element: ElementId) -> Option<&TabInputKey> {
    tab_part.inputs().find_map(|input| {
        if input.is_settings() {
            return [TabContainerPlacement::Body, TabContainerPlacement::Titlebar]
                .into_iter()
                .any(|placement| {
                    placement.settings_id() == element || placement.settings_action_id() == element
                })
                .then_some(input.key());
        }
        let tab_id = tab_part.tab_id(input.key())?;
        [TabContainerPlacement::Body, TabContainerPlacement::Titlebar]
            .into_iter()
            .any(|placement| {
                placement.session_id(tab_id) == element || placement.action_id(tab_id) == element
            })
            .then_some(input.key())
    })
}

pub fn workbench_tab_groups<'a>(
    tab_part: &'a TabPart,
    placement: TabContainerPlacement,
    include: impl Fn(&TabInput) -> bool,
) -> Vec<WorkbenchTabGroup<'a>> {
    tab_part
        .groups()
        .iter()
        .filter_map(|group| {
            let tabs = group
                .inputs()
                .iter()
                .filter(|input| include(input))
                .map(|input| WorkbenchTab::from_input(tab_part, input, placement))
                .collect::<Vec<_>>();
            (!tabs.is_empty()).then(|| {
                WorkbenchTabGroup::new(group.id(), group.label(), group.is_collapsed(), tabs)
            })
        })
        .collect()
}

impl<'a> WorkbenchTab<'a> {
    pub fn from_input(
        tab_part: &'a TabPart,
        input: &'a TabInput,
        placement: TabContainerPlacement,
    ) -> Self {
        if input.is_settings() {
            Self::settings(
                placement.settings_id(),
                placement.settings_action_id(),
                placement.settings_close_id(),
                tab_part.tab_name(input),
            )
        } else {
            let tab_id = tab_part
                .tab_id(input.key())
                .expect("mounted Session TabInput must have a TabId");
            Self::new(
                placement.session_id(tab_id),
                placement.action_id(tab_id),
                placement.close_id(tab_id),
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
