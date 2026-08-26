use zeta_protocol::SessionId;

use crate::terminal_session::TerminalSessionKey;

pub(crate) use zeta_workbench::PaneInput;
pub(crate) use zeta_workbench::PaneInputKind;

/// Runtime currently attached to a workbench [`PaneInput`] by the Native host.
///
/// This mapping is deliberately product-local. The workbench description stays free of PTY and
/// terminal-session handles while zeterm resolves a Session into its runtime key here.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PaneRuntime {
    Terminal(TerminalSessionKey),
}

/// Binding between one logical [`PaneInput`] and the runtime, if one has been mounted.
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

    /// Attaches a Terminal runtime only when this binding describes the matching Session.
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
