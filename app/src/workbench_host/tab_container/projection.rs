//! Mapping from orientation-neutral Workbench tabs into one Native UI identity scope.

use zeta_ui::TabListOrientation;
use zeta_workbench::TabGroupId;
use zui::ui::ElementId;
use zui::ui::NavigationAxis;

use crate::shell_interaction::FIRST_TAB_CONTAINER_SESSION_TAB;
use crate::shell_interaction::FIRST_TITLEBAR_SESSION_TAB;
use crate::shell_interaction::TAB_CONTAINER;
use crate::shell_interaction::TAB_CONTAINER_SETTINGS_TAB;
use crate::shell_interaction::TITLEBAR;
use crate::shell_interaction::TITLEBAR_SETTINGS_TAB;
use crate::shell_interaction::TITLEBAR_TAB_CONTAINER;
use crate::shell_interaction::session_tab_id;
use crate::shell_interaction::tab_group_list_id;
use crate::shell_interaction::titlebar_session_tab_id;
use crate::shell_interaction::titlebar_tab_group_list_id;
use crate::workbench_host::TabInput;
use crate::workbench_host::TabInputKey;
use crate::workbench_host::TabPart;

/// UI mount that projects the same logical Tab Part into a concrete Workbench location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TabContainerPlacement {
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
            Self::Body => crate::shell_interaction::WINDOW,
            Self::Titlebar => TITLEBAR,
        }
    }

    pub(super) const fn settings_id(self) -> ElementId {
        match self {
            Self::Body => TAB_CONTAINER_SETTINGS_TAB,
            Self::Titlebar => TITLEBAR_SETTINGS_TAB,
        }
    }

    pub(super) fn session_id(self, index: usize) -> ElementId {
        match self {
            Self::Body => session_tab_id(index),
            Self::Titlebar => titlebar_session_tab_id(index),
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

#[derive(Clone, Copy)]
pub(crate) struct WorkbenchTab<'a> {
    pub(super) id: ElementId,
    pub(super) kind: WorkbenchTabKind,
    pub(super) name: &'a str,
    pub(super) workspace: &'a str,
    pub(super) status_label: &'a str,
}

#[derive(Clone)]
pub(crate) struct WorkbenchTabGroup<'a> {
    pub(super) id: TabGroupId,
    pub(super) label: Option<&'a str>,
    pub(super) collapsed: bool,
    pub(super) tabs: Vec<WorkbenchTab<'a>>,
}

impl<'a> WorkbenchTabGroup<'a> {
    pub(crate) fn new(
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

    pub(crate) const fn id(&self) -> TabGroupId {
        self.id
    }

    pub(crate) fn insert_tab(&mut self, index: usize, tab: WorkbenchTab<'a>) {
        self.tabs.insert(index.min(self.tabs.len()), tab);
    }
}

/// Resolves one projection's element identity without leaking UI identity into Workbench state.
pub(crate) fn tab_input_element_id(
    tab_part: &TabPart,
    selected: Option<&TabInputKey>,
    placement: TabContainerPlacement,
) -> ElementId {
    let mut session_index = 0;
    for input in tab_part.inputs() {
        let id = if input.is_settings() {
            placement.settings_id()
        } else {
            placement.session_id(session_index)
        };
        if selected.is_some_and(|selected| input.key() == selected) {
            return id;
        }
        if input.is_session() {
            session_index += 1;
        }
    }
    match placement {
        TabContainerPlacement::Body => FIRST_TAB_CONTAINER_SESSION_TAB,
        TabContainerPlacement::Titlebar => FIRST_TITLEBAR_SESSION_TAB,
    }
}

pub(crate) fn project_tab_groups<'a>(
    tab_part: &'a TabPart,
    placement: TabContainerPlacement,
    include: impl Fn(&TabInput) -> bool,
) -> Vec<WorkbenchTabGroup<'a>> {
    let mut session_index = 0;
    tab_part
        .groups()
        .iter()
        .filter_map(|group| {
            let tabs = group
                .inputs()
                .iter()
                .filter_map(|input| {
                    let index = session_index;
                    if input.is_session() {
                        session_index += 1;
                    }
                    include(input).then(|| WorkbenchTab::from_input(index, input, placement))
                })
                .collect::<Vec<_>>();
            (!tabs.is_empty()).then(|| {
                WorkbenchTabGroup::new(group.id(), group.label(), group.is_collapsed(), tabs)
            })
        })
        .collect()
}

impl<'a> WorkbenchTab<'a> {
    pub(crate) fn from_input(
        index: usize,
        input: &'a TabInput,
        placement: TabContainerPlacement,
    ) -> Self {
        if input.is_settings() {
            Self::settings(placement.settings_id())
        } else {
            Self::new(
                placement.session_id(index),
                input.title(),
                input.workspace(),
                input.status_label(),
            )
        }
    }

    pub(crate) const fn new(
        id: ElementId,
        name: &'a str,
        workspace: &'a str,
        status_label: &'a str,
    ) -> Self {
        Self {
            id,
            kind: WorkbenchTabKind::Session,
            name,
            workspace,
            status_label,
        }
    }

    pub(crate) const fn settings(id: ElementId) -> Self {
        Self {
            id,
            kind: WorkbenchTabKind::Settings,
            name: "Settings",
            workspace: "Application",
            status_label: "",
        }
    }
}
