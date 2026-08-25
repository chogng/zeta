use std::collections::VecDeque;

use crate::window::WindowId;

/// Policy controlling whether an application exits automatically after closing its last window.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ExitPolicy {
    /// Exit after the final live window has completed its close lifecycle.
    #[default]
    OnLastWindowClosed,
    /// Keep the event loop alive until the application explicitly exits.
    Explicit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WindowCommand {
    Opened(WindowId),
    Close(WindowId),
    Exit,
}

#[derive(Default)]
pub(crate) struct WindowCommandQueue {
    commands: VecDeque<WindowCommand>,
}

impl WindowCommandQueue {
    pub(crate) fn push(&mut self, command: WindowCommand) {
        self.commands.push_back(command);
    }

    pub(crate) fn pop(&mut self) -> Option<WindowCommand> {
        self.commands.pop_front()
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
