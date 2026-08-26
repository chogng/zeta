//! Feature-owned Files and SCM panes for the current workspace.
//!
//! This module retains presentation-domain state and emits user intents. The
//! product host owns platform events, filesystem and SCM transport, and all
//! side effects.

#[path = "workspace_panes/files.rs"]
mod files;
#[path = "workspace_panes/navigation.rs"]
mod navigation;
#[path = "workspace_panes/scm.rs"]
mod scm;
#[path = "workspace_panes/style.rs"]
mod style;

use std::path::PathBuf;

use crate::shell_interaction::AGENT_CHANGES;
use crate::shell_interaction::AGENT_FILES;
use zui::ui::ElementId;

pub use files::DirectoryEntry;
pub use files::FILE_LIST_ROW_HEIGHT;
pub use files::FilesLayout;
pub use files::FilesPane;
pub use files::FilesPaneStyle;
pub use files::FilesState;
pub use files::FilesToolbar;
#[cfg(test)]
pub(crate) use files::FilesTreeRow;
pub use navigation::AgentSidebarNavigation;
pub use scm::EditorPane;
pub use scm::EditorPaneState;
pub use scm::ScmDiff;
pub use scm::ScmLayout;
pub use scm::ScmPaneStyle;
pub use scm::ScmState;
pub use scm::ScrollbarPointerOutcome;
pub use style::AgentSidebarStyle;

/// The two stable workspace-pane selection intents published by the current shell toolbar.
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

    pub const fn view(self) -> AgentSidebarView {
        match self {
            Self::Changes => AgentSidebarView::Changes,
            Self::Files => AgentSidebarView::Files,
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

#[cfg(test)]
pub(crate) const TEST_SCM_PANE_STYLE: ScmPaneStyle = ScmPaneStyle {
    surface: zeta_ui::Color::WHITE,
    border: zeta_ui::Color::rgb(222, 222, 224),
    text_muted: zeta_ui::Color::rgb(126, 126, 132),
};

/// A feature view that can be mounted in the sidebar Pane.
///
/// Selection is owned by the host's `PaneInput`; this enum is only the feature crate's
/// presentation vocabulary for its navigation buttons.
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

/// Retained product state for SidebarPart feature panes.
///
/// Hosts provide Files and SCM snapshots and execute returned actions, keeping
/// native platform and app-server dependencies outside this crate.
#[derive(Default)]
pub struct AgentSidebar {
    files: FilesState,
    scm: ScmState,
}

impl AgentSidebar {
    pub fn new(workspace_root: PathBuf) -> Self {
        let mut sidebar = Self::default();
        sidebar.files.set_workspace_root(workspace_root);
        sidebar
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
