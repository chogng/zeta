use std::path::PathBuf;

use zeta_protocol::{SessionId, ThreadId};

use crate::terminal_session::TerminalSessionKey;

/// Product content kind that can be mounted into one Pane.
///
/// The kind is intentionally independent from layout identity. A `PaneId` identifies a leaf in a
/// `PaneGroup`; this value identifies what the leaf is currently showing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum PaneInputKind {
    /// A terminal compatibility surface backed by one Agent Session.
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

/// Logical content input mounted by a product host into one Pane.
///
/// This is a descriptor, not a renderer widget. Feature crates own the payload, view state, and
/// runtime behind each variant; the host owns which descriptor is mounted in which Pane.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[allow(dead_code)]
pub(crate) enum PaneInput {
    /// A terminal associated with one product-level Session.
    Terminal(TerminalPaneInput),
    /// An Agent view associated with one Session and independently ordered Thread.
    Agent(AgentPaneInput),
    /// A file browser rooted at one workspace directory.
    Files(FilesPaneInput),
    /// A diff view rooted at one workspace directory.
    Diff(DiffPaneInput),
    /// The singleton Settings surface. The selected section is view state, not input identity.
    Settings,
}

/// Logical input for a terminal Pane.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct TerminalPaneInput {
    session_id: SessionId,
}

/// Logical input for an Agent conversation Pane.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct AgentPaneInput {
    session_id: SessionId,
    thread_id: ThreadId,
}

/// Logical input for a Files Pane.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct FilesPaneInput {
    workspace_root: PathBuf,
}

/// Logical input for a Diff Pane.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct DiffPaneInput {
    workspace_root: PathBuf,
}

impl PaneInput {
    pub(crate) fn terminal(session_id: SessionId) -> Self {
        Self::Terminal(TerminalPaneInput { session_id })
    }

    #[allow(dead_code)]
    pub(crate) fn agent(session_id: SessionId, thread_id: ThreadId) -> Self {
        Self::Agent(AgentPaneInput {
            session_id,
            thread_id,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn files(workspace_root: PathBuf) -> Self {
        Self::Files(FilesPaneInput { workspace_root })
    }

    #[allow(dead_code)]
    pub(crate) fn diff(workspace_root: PathBuf) -> Self {
        Self::Diff(DiffPaneInput { workspace_root })
    }

    #[allow(dead_code)]
    pub(crate) const fn settings() -> Self {
        Self::Settings
    }

    pub(crate) const fn kind(&self) -> PaneInputKind {
        match self {
            Self::Terminal(_) => PaneInputKind::Terminal,
            Self::Agent(_) => PaneInputKind::Agent,
            Self::Files(_) => PaneInputKind::Files,
            Self::Diff(_) => PaneInputKind::Diff,
            Self::Settings => PaneInputKind::Settings,
        }
    }

    pub(crate) fn terminal_session_id(&self) -> Option<&SessionId> {
        match self {
            Self::Terminal(input) => Some(&input.session_id),
            _ => None,
        }
    }
}

/// Runtime currently attached to a PaneInput by the Native host.
///
/// This enum is deliberately host-local. Adding an Agent or editor runtime should add a binding
/// at the corresponding host/feature boundary instead of making `PaneInput` own that runtime.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PaneRuntime {
    Terminal(TerminalSessionKey),
}

/// Binding between one logical PaneInput and the runtime, if one has been mounted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PaneBinding {
    input: PaneInput,
    runtime: Option<PaneRuntime>,
}

impl PaneBinding {
    pub(crate) fn new(input: PaneInput) -> Self {
        Self {
            input,
            runtime: None,
        }
    }

    pub(crate) fn terminal(session_id: SessionId, key: TerminalSessionKey) -> Self {
        Self {
            input: PaneInput::terminal(session_id),
            runtime: Some(PaneRuntime::Terminal(key)),
        }
    }

    pub(crate) fn input(&self) -> &PaneInput {
        &self.input
    }

    pub(crate) fn terminal_key(&self) -> Option<TerminalSessionKey> {
        match self.runtime {
            Some(PaneRuntime::Terminal(key)) => Some(key),
            None => None,
        }
    }

    pub(crate) fn clear_runtime(&mut self) {
        self.runtime = None;
    }

    /// Attaches a Terminal runtime only when this binding describes terminal content.
    pub(crate) fn bind_terminal(
        &mut self,
        session_id: &SessionId,
        key: TerminalSessionKey,
    ) -> bool {
        if self.input.kind() != PaneInputKind::Terminal
            || self.input.terminal_session_id() != Some(session_id)
        {
            return false;
        }
        self.runtime = Some(PaneRuntime::Terminal(key));
        true
    }
}

#[cfg(test)]
#[path = "pane_input_tests.rs"]
mod tests;
