use crate::TerminalSessionKey;
use zeta_workbench::{PaneInput, PaneInputKind};

/// Terminal runtime currently attached to a Workbench pane.
///
/// This mapping is deliberately product-local. The workbench description stays free of PTY and
/// terminal-session handles while app resolves a Session into its runtime key here.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PaneRuntime {
    Terminal(TerminalSessionKey),
}

/// Binding between one workbench group and its feature runtime, if one has been mounted.
///
/// The logical [`PaneInput`] is owned by `zeta_workbench::PaneGroup`. This type deliberately keeps
/// only the product-local runtime handle so there is no second logical-input owner in the host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaneBinding {
    runtime: Option<PaneRuntime>,
}

impl PaneBinding {
    pub const fn new() -> Self {
        Self { runtime: None }
    }

    pub const fn terminal(key: TerminalSessionKey) -> Self {
        Self {
            runtime: Some(PaneRuntime::Terminal(key)),
        }
    }

    pub fn terminal_key(&self) -> Option<TerminalSessionKey> {
        match self.runtime {
            Some(PaneRuntime::Terminal(key)) => Some(key),
            None => None,
        }
    }

    /// Attaches a Terminal runtime only when this binding describes the matching Session.
    pub fn bind_terminal(
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
#[path = "pane_binding_tests.rs"]
mod tests;
