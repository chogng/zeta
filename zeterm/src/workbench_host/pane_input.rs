use crate::terminal_session::TerminalSessionKey;
use zeta_workbench::{PaneInput, PaneInputKind};

/// Runtime currently attached to a workbench pane by the Native host.
///
/// This mapping is deliberately product-local. The workbench description stays free of PTY and
/// terminal-session handles while zeterm resolves a Session into its runtime key here.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PaneRuntime {
    Terminal(TerminalSessionKey),
}

/// Binding between one workbench group and its feature runtime, if one has been mounted.
///
/// The logical [`PaneInput`] is owned by `zeta_workbench::PaneGroup`. This type deliberately keeps
/// only the product-local runtime handle so there is no second logical-input owner in the host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PaneBinding {
    runtime: Option<PaneRuntime>,
}

impl PaneBinding {
    pub(crate) const fn new() -> Self {
        Self { runtime: None }
    }

    pub(crate) const fn terminal(key: TerminalSessionKey) -> Self {
        Self {
            runtime: Some(PaneRuntime::Terminal(key)),
        }
    }

    pub(crate) fn terminal_key(&self) -> Option<TerminalSessionKey> {
        match self.runtime {
            Some(PaneRuntime::Terminal(key)) => Some(key),
            None => None,
        }
    }

    /// Attaches a Terminal runtime only when this binding describes the matching Session.
    pub(crate) fn bind_terminal(
        &mut self,
        input: &PaneInput,
        session_id: &zeta_protocol::SessionId,
        key: TerminalSessionKey,
    ) -> bool {
        if input.kind() != PaneInputKind::Terminal
            || input.terminal_session_id() != Some(session_id)
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
