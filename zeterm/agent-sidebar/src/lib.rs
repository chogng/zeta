//! Product-level Files and SCM panes hosted by the Agent sidebar.
//!
//! The crate retains presentation-domain state and emits user intents. Its host
//! owns platform events, filesystem and SCM transport, and all side effects.

mod files;
mod navigation;
mod scm;
mod style;

mod shell_interaction {
    use zui::ui::ElementId;

    pub const AGENT_SIDEBAR: ElementId = ElementId::scoped(1, 23);
    pub const AGENT_EXPLORER_PANE: ElementId = ElementId::scoped(1, 28);
    pub const AGENT_EDITOR_PANE: ElementId = ElementId::scoped(1, 29);
    pub const MULTI_DIFF_EDITOR: ElementId = ElementId::scoped(1, 30);
    pub const MULTI_DIFF_SCROLLBAR: ElementId = ElementId::scoped(1, 31);
    pub const AGENT_SIDEBAR_NAVIGATION: ElementId = ElementId::scoped(1, 32);
    pub const AGENT_CHANGES: ElementId = ElementId::scoped(1, 33);
    pub const AGENT_FILES: ElementId = ElementId::scoped(1, 34);
    pub const AGENT_SIDEBAR_TOOLBAR: ElementId = ElementId::scoped(1, 35);
    pub const AGENT_FILES_ACTION_BAR: ElementId = ElementId::scoped(1, 36);
    pub const AGENT_FILES_REFRESH: ElementId = ElementId::scoped(1, 37);
    pub const AGENT_FILES_SEARCH: ElementId = ElementId::scoped(1, 38);
    pub const AGENT_FILE_SEARCH_INPUT: ElementId = ElementId::scoped(1, 39);
    pub const AGENT_FILES_TOOLBAR: ElementId = ElementId::scoped(1, 52);

    /// The two stable pane-selection intents published by the Agent Sidebar.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum AgentSidebarPaneAction {
        Changes,
        Files,
    }

    impl AgentSidebarPaneAction {
        pub const ALL: [Self; 2] = [Self::Changes, Self::Files];
        pub const fn element_id(self) -> ElementId {
            match self {
                Self::Changes => AGENT_CHANGES,
                Self::Files => AGENT_FILES,
            }
        }
        pub const fn label(self) -> &'static str {
            match self {
                Self::Changes => "Changes",
                Self::Files => "Files",
            }
        }
        pub const fn view(self) -> crate::AgentSidebarView {
            match self {
                Self::Changes => crate::AgentSidebarView::Changes,
                Self::Files => crate::AgentSidebarView::Files,
            }
        }
        pub const fn from_element_id(id: ElementId) -> Option<Self> {
            match id {
                AGENT_CHANGES => Some(Self::Changes),
                AGENT_FILES => Some(Self::Files),
                _ => None,
            }
        }
    }
}

pub use shell_interaction::AGENT_CHANGES;
pub use shell_interaction::AGENT_EDITOR_PANE;
pub use shell_interaction::AGENT_EXPLORER_PANE;
pub use shell_interaction::AGENT_FILE_SEARCH_INPUT;
pub use shell_interaction::AGENT_FILES;
pub use shell_interaction::AGENT_FILES_ACTION_BAR;
pub use shell_interaction::AGENT_FILES_REFRESH;
pub use shell_interaction::AGENT_FILES_SEARCH;
pub use shell_interaction::AGENT_FILES_TOOLBAR;
pub use shell_interaction::AGENT_SIDEBAR;
pub use shell_interaction::AGENT_SIDEBAR_NAVIGATION;
pub use shell_interaction::AGENT_SIDEBAR_TOOLBAR;
pub use shell_interaction::AgentSidebarPaneAction;
pub use shell_interaction::MULTI_DIFF_EDITOR;
pub use shell_interaction::MULTI_DIFF_SCROLLBAR;

mod shell_style {
    #[cfg(test)]
    pub(crate) use crate::ScmPaneStyle as ShellPalette;

    #[cfg(test)]
    pub(crate) const SHELL_PALETTE: ShellPalette = ShellPalette {
        surface: zeta_ui::Color::WHITE,
        border: zeta_ui::Color::rgb(222, 222, 224),
        text_muted: zeta_ui::Color::rgb(126, 126, 132),
    };
}

mod workspace_context {
    pub(crate) use crate::ScmDiff as WorkspaceDiff;
}

use std::path::PathBuf;

use zui::ui::ElementId;

pub use files::DirectoryEntry;
pub use files::DirectoryEntryKind;
pub use files::EXPLORER_PANE;
pub use files::FILE_LIST_ROW_HEIGHT;
pub use files::FILES_TOOLBAR_HEIGHT;
pub use files::FilesEntry;
pub use files::FilesLayout;
pub use files::FilesPane;
pub use files::FilesPaneStyle;
pub use files::FilesState;
pub use files::FilesToolbar;
pub use files::FilesTreeRow;
pub use navigation::AgentSidebarNavigation;
pub use scm::EditorPane;
pub use scm::EditorPaneState;
pub use scm::SCM_TOOLBAR_HEIGHT;
pub use scm::ScmDiff;
pub use scm::ScmLayout;
pub use scm::ScmPaneStyle;
pub use scm::ScmState;
pub use scm::ScrollbarPointerOutcome;
pub use style::AgentSidebarStyle;

/// The active product pane in the Agent sidebar.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AgentSidebarView {
    Changes,
    #[default]
    Files,
}

/// A product intent raised by a Files-pane interaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentSidebarAction {
    Handled,
    StateChanged,
    Focus(ElementId),
    OpenFile { path: PathBuf },
    LoadChildren { element: ElementId, path: PathBuf },
}

/// Retained product state for Agent Sidebar panes.
///
/// Hosts provide Files and SCM snapshots and execute returned actions, keeping
/// native platform and app-server dependencies outside this crate.
#[derive(Default)]
pub struct AgentSidebar {
    active_view: AgentSidebarView,
    files: FilesState,
    scm: ScmState,
}

impl AgentSidebar {
    pub fn new(workspace_root: PathBuf) -> Self {
        let mut sidebar = Self::default();
        sidebar.files.set_workspace_root(workspace_root);
        sidebar
    }

    pub const fn active_view(&self) -> AgentSidebarView {
        self.active_view
    }
    pub fn select_view(&mut self, view: AgentSidebarView) {
        self.active_view = view;
    }
    pub const fn files(&self) -> &FilesState {
        &self.files
    }
    pub fn files_mut(&mut self) -> &mut FilesState {
        &mut self.files
    }
    pub const fn scm(&self) -> &ScmState {
        &self.scm
    }
    pub fn scm_mut(&mut self) -> &mut ScmState {
        &mut self.scm
    }

    pub fn replace_workspace(&mut self, workspace_root: PathBuf) {
        self.files.set_workspace_root(workspace_root);
        self.scm.replace_diffs([]);
    }
}
