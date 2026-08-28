//! Logical Workbench tab inputs and their product metadata.

use std::path::Path;
use std::path::PathBuf;

use zeta_protocol::SessionId;

use crate::TabStatus;

/// Stable logical identity for one input that can be shown by a Workbench tab.
///
/// UI element identities deliberately do not belong here. They are allocated by the mounted tab
/// list when the input is projected into a frame.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TabInputKey {
    Session(SessionId),
    Settings,
}

impl TabInputKey {
    pub fn session(session_id: SessionId) -> Self {
        Self::Session(session_id)
    }

    pub fn session_id(&self) -> Option<&SessionId> {
        match self {
            Self::Session(session_id) => Some(session_id),
            Self::Settings => None,
        }
    }

    pub const fn is_session(&self) -> bool {
        matches!(self, Self::Session(_))
    }

    pub const fn is_settings(&self) -> bool {
        matches!(self, Self::Settings)
    }
}

/// Product-owned logical input behind one Workbench tab.
///
/// This record contains the stable input identity and the labels needed by the Workbench view.
/// Session lifecycle and Thread state remain owned by the App Server session adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TabInput {
    key: TabInputKey,
    metadata: TabInputMetadata,
}

/// Display metadata supplied by the product owner for one Workbench tab.
///
/// The Workbench stores the current values used by its tab surfaces, but it does not derive them
/// from Session protocol records or decide product copy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TabInputMetadata {
    title: String,
    workspace: String,
    workspace_roots: Vec<PathBuf>,
    status: TabStatus,
}

impl TabInputMetadata {
    pub fn new(title: impl Into<String>, workspace: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            workspace: workspace.into(),
            workspace_roots: Vec::new(),
            status: TabStatus::default(),
        }
    }

    pub fn with_workspace_root(mut self, workspace_root: PathBuf) -> Self {
        self.workspace_roots = vec![workspace_root];
        self
    }

    #[cfg(test)]
    /// Supplies the primary Workspace root followed by any additional roots shown for this tab.
    pub fn with_workspace_roots(
        mut self,
        workspace_roots: impl IntoIterator<Item = PathBuf>,
    ) -> Self {
        self.workspace_roots.clear();
        for root in workspace_roots {
            if !self.workspace_roots.contains(&root) {
                self.workspace_roots.push(root);
            }
        }
        self
    }

    pub fn with_status(mut self, status: TabStatus) -> Self {
        self.status = status;
        self
    }
}

impl TabInput {
    pub fn from_settings() -> Self {
        Self {
            key: TabInputKey::Settings,
            metadata: TabInputMetadata::new("Settings", "Application"),
        }
    }

    pub fn session(session_id: SessionId, metadata: TabInputMetadata) -> Self {
        Self {
            key: TabInputKey::session(session_id),
            metadata,
        }
    }

    pub fn key(&self) -> &TabInputKey {
        &self.key
    }

    pub fn session_id(&self) -> Option<&SessionId> {
        self.key.session_id()
    }

    pub const fn is_session(&self) -> bool {
        self.key.is_session()
    }

    pub const fn is_settings(&self) -> bool {
        self.key.is_settings()
    }

    pub fn title(&self) -> &str {
        &self.metadata.title
    }

    pub fn workspace(&self) -> &str {
        &self.metadata.workspace
    }

    pub fn workspace_root(&self) -> Option<&Path> {
        self.metadata.workspace_roots.first().map(PathBuf::as_path)
    }

    /// Returns the primary Workspace root followed by additional roots in display order.
    pub fn workspace_roots(&self) -> &[PathBuf] {
        &self.metadata.workspace_roots
    }

    pub const fn status(&self) -> &TabStatus {
        &self.metadata.status
    }

    pub(crate) fn update_from(&mut self, input: Self) {
        debug_assert_eq!(self.key, input.key);
        self.metadata = input.metadata;
    }

    pub fn update_status(&mut self, status: TabStatus) {
        self.metadata.status = status;
    }
}

/// A change made while inserting or refreshing one logical TabInput.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TabInputChange {
    Added(TabInputKey),
    Updated(TabInputKey),
}

#[cfg(test)]
#[path = "tab_input_tests.rs"]
mod tests;
