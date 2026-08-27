//! Logical workbench tab inputs and selection state.

use std::path::Path;
use std::path::PathBuf;

use zeta_protocol::Session;
use zeta_protocol::SessionId;

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
/// This record contains the stable input identity and the labels needed by the shell projection.
/// Session lifecycle and Thread state remain owned by the App Server session adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TabInput {
    key: TabInputKey,
    title: String,
    workspace: String,
    workspace_root: Option<PathBuf>,
    status_label: String,
}

impl TabInput {
    pub fn from_settings() -> Self {
        Self {
            key: TabInputKey::Settings,
            title: "Settings".to_owned(),
            workspace: "Application".to_owned(),
            workspace_root: None,
            status_label: String::new(),
        }
    }

    pub fn from_session(session: &Session, workspace: &str) -> Self {
        Self {
            key: TabInputKey::session(session.session_id.clone()),
            title: session.title.clone(),
            workspace: workspace_label(session, workspace),
            workspace_root: session
                .workspace
                .as_ref()
                .map(|binding| binding.root.clone()),
            status_label: "Active".to_owned(),
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
        &self.title
    }

    pub fn workspace(&self) -> &str {
        &self.workspace
    }

    pub fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    pub fn status_label(&self) -> &str {
        &self.status_label
    }

    pub(crate) fn update_from_session(&mut self, session: &Session, workspace: &str) {
        debug_assert_eq!(self.session_id(), Some(&session.session_id));
        self.title = session.title.clone();
        self.workspace = workspace_label(session, workspace);
        self.workspace_root = session
            .workspace
            .as_ref()
            .map(|binding| binding.root.clone());
        self.status_label = "Active".to_owned();
    }

    pub fn update_status(&mut self, status_label: impl Into<String>) {
        self.status_label = status_label.into();
    }
}

/// A change made while inserting or refreshing one logical TabInput.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TabInputChange {
    Added(TabInputKey),
    Updated(TabInputKey),
}

fn workspace_label<'a>(session: &'a Session, fallback: &'a str) -> String {
    session
        .workspace
        .as_ref()
        .and_then(|binding| binding.root.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| fallback.to_owned())
}
