use std::path::Path;
use std::path::PathBuf;

use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

/// Product content kind that can be mounted into one PaneGroup input.
///
/// The kind is independent from layout identity. A [`PaneGroupId`](crate::PaneGroupId) identifies
/// a leaf in a [`PaneGroup`](crate::PaneGroup); this value identifies what the group is showing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PaneInputKind {
    /// A terminal surface described by one product-level Session.
    Terminal,
    /// An Agent conversation or thread surface.
    Agent,
    /// A workspace file browser.
    Files,
    /// A workspace change or diff surface.
    Diff,
    /// The application settings surface.
    Settings,
}

/// Logical content description mounted by a product host into one Pane.
///
/// This is a description, not a renderer widget or runtime handle. In particular, the Terminal
/// variant carries only a [`SessionId`]; terminal startup and runtime-handle ownership stay in the
/// product host.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PaneInput {
    /// A terminal surface associated with one product-level Session.
    Terminal(SessionId),
    /// An Agent view associated with one Session and independently ordered Thread.
    Agent {
        session_id: SessionId,
        thread_id: ThreadId,
    },
    /// A file browser rooted at one workspace directory.
    Files { workspace_root: PathBuf },
    /// A diff view rooted at one workspace directory.
    Diff { workspace_root: PathBuf },
    /// The singleton Settings surface. The selected section is view state, not input identity.
    Settings,
}

impl PaneInput {
    /// Creates a Terminal description without allocating or starting a terminal runtime.
    pub fn terminal(session_id: SessionId) -> Self {
        Self::Terminal(session_id)
    }

    /// Creates an Agent description for one Session and Thread.
    pub fn agent(session_id: SessionId, thread_id: ThreadId) -> Self {
        Self::Agent {
            session_id,
            thread_id,
        }
    }

    /// Creates a Files description rooted at one workspace directory.
    pub fn files(workspace_root: PathBuf) -> Self {
        Self::Files { workspace_root }
    }

    /// Creates a Diff description rooted at one workspace directory.
    pub fn diff(workspace_root: PathBuf) -> Self {
        Self::Diff { workspace_root }
    }

    /// Creates the singleton Settings description.
    pub const fn settings() -> Self {
        Self::Settings
    }

    /// Returns the content kind without exposing any runtime binding.
    pub const fn kind(&self) -> PaneInputKind {
        match self {
            Self::Terminal(_) => PaneInputKind::Terminal,
            Self::Agent { .. } => PaneInputKind::Agent,
            Self::Files { .. } => PaneInputKind::Files,
            Self::Diff { .. } => PaneInputKind::Diff,
            Self::Settings => PaneInputKind::Settings,
        }
    }

    /// Returns the Session described by a Terminal input.
    pub fn terminal_session_id(&self) -> Option<&SessionId> {
        match self {
            Self::Terminal(session_id) => Some(session_id),
            _ => None,
        }
    }

    /// Returns the Session described by an Agent input.
    pub fn agent_session_id(&self) -> Option<&SessionId> {
        match self {
            Self::Agent { session_id, .. } => Some(session_id),
            _ => None,
        }
    }

    /// Returns the Thread described by an Agent input.
    pub fn thread_id(&self) -> Option<&ThreadId> {
        match self {
            Self::Agent { thread_id, .. } => Some(thread_id),
            _ => None,
        }
    }

    /// Returns the workspace root described by a Files or Diff input.
    pub fn workspace_root(&self) -> Option<&Path> {
        match self {
            Self::Files { workspace_root } | Self::Diff { workspace_root } => Some(workspace_root),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "pane_input_tests.rs"]
mod tests;
