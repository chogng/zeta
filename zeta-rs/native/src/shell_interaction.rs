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
pub(crate) const SIDEBAR_TOGGLE: ElementId = ElementId::scoped(SHELL_SCOPE, 12);
pub(crate) const SESSION_SIDEBAR: ElementId = ElementId::scoped(SHELL_SCOPE, 13);
pub(crate) const SESSION_TAB_LIST: ElementId = ElementId::scoped(SHELL_SCOPE, 14);
pub(crate) const ACTIVE_SESSION_TAB: ElementId = ElementId::scoped(SHELL_SCOPE, 15);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SessionSidebarState {
    #[default]
    Collapsed,
    Expanded,
}

impl SessionSidebarState {
    pub(crate) const fn is_expanded(self) -> bool {
        matches!(self, Self::Expanded)
    }

    pub(crate) const fn toggled(self) -> Self {
        match self {
            Self::Collapsed => Self::Expanded,
            Self::Expanded => Self::Collapsed,
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
