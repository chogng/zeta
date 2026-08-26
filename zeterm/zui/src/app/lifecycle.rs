use std::collections::HashSet;
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

/// Current reusable application-host lifecycle phase.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ApplicationPhase {
    /// The event loop has been created but has not delivered its first resume.
    #[default]
    Initializing,
    /// Native application resources may be created and windows may receive events.
    Active,
    /// The platform temporarily suspended active window work.
    Suspended,
    /// The event loop is releasing application resources.
    Exiting,
}

/// Stable reason recorded when an application begins exiting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationExitReason {
    /// Product code explicitly requested a normal exit.
    Requested,
    /// The configured policy exited after the final product window closed.
    LastWindowClosed,
    /// Product or runtime work recorded a fatal [`super::ApplicationError`].
    FatalError,
    /// The platform event loop ended without an earlier ZUI exit request.
    Platform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WindowCommand {
    Opened(WindowId),
    Close(WindowId),
    Exit(ApplicationExitReason),
}

#[derive(Default)]
struct WindowCommandQueue {
    commands: VecDeque<WindowCommand>,
    pending_closes: HashSet<WindowId>,
    exit_pending: bool,
}

impl WindowCommandQueue {
    fn opened(&mut self, window: WindowId) {
        self.commands.push_back(WindowCommand::Opened(window));
    }

    fn close(&mut self, window: WindowId) -> bool {
        if !self.pending_closes.insert(window) {
            return false;
        }
        self.commands.push_back(WindowCommand::Close(window));
        true
    }

    fn exit(&mut self, reason: ApplicationExitReason) -> bool {
        if self.exit_pending {
            return false;
        }
        self.exit_pending = true;
        self.commands.push_back(WindowCommand::Exit(reason));
        true
    }

    fn pop(&mut self) -> Option<WindowCommand> {
        let command = self.commands.pop_front()?;
        if let WindowCommand::Close(window) = command {
            self.pending_closes.remove(&window);
        }
        Some(command)
    }
}

/// Shared lifecycle state used by the native host and deterministic test runtime.
pub(crate) struct LifecycleCore {
    phase: ApplicationPhase,
    exit_policy: ExitPolicy,
    exit_reason: Option<ApplicationExitReason>,
    product_windows: HashSet<WindowId>,
    commands: WindowCommandQueue,
}

impl LifecycleCore {
    pub(crate) fn new(exit_policy: ExitPolicy) -> Self {
        Self {
            phase: ApplicationPhase::Initializing,
            exit_policy,
            exit_reason: None,
            product_windows: HashSet::new(),
            commands: WindowCommandQueue::default(),
        }
    }

    pub(crate) const fn phase(&self) -> ApplicationPhase {
        self.phase
    }

    pub(crate) const fn exit_policy(&self) -> ExitPolicy {
        self.exit_policy
    }

    pub(crate) fn set_exit_policy(&mut self, exit_policy: ExitPolicy) {
        self.exit_policy = exit_policy;
    }

    /// Enters the active phase and returns whether this was the first resume.
    pub(crate) fn resumed(&mut self) -> bool {
        let first = self.phase == ApplicationPhase::Initializing;
        self.phase = ApplicationPhase::Active;
        first
    }

    pub(crate) fn suspended(&mut self) {
        self.phase = ApplicationPhase::Suspended;
    }

    pub(crate) fn record_window_opened(&mut self, window: WindowId) -> bool {
        if !self.product_windows.insert(window) {
            return false;
        }
        self.commands.opened(window);
        true
    }

    pub(crate) fn request_window_close(&mut self, window: WindowId) -> bool {
        self.commands.close(window)
    }

    pub(crate) fn record_window_closed(&mut self, window: WindowId) -> bool {
        self.product_windows.remove(&window)
    }

    pub(crate) fn has_product_windows(&self) -> bool {
        !self.product_windows.is_empty()
    }

    pub(crate) fn should_exit_after_last_window(&self) -> bool {
        !self.has_product_windows() && self.exit_policy == ExitPolicy::OnLastWindowClosed
    }

    pub(crate) fn request_exit(&mut self, reason: ApplicationExitReason) -> bool {
        self.commands.exit(reason)
    }

    pub(crate) fn next_command(&mut self) -> Option<WindowCommand> {
        self.commands.pop()
    }

    pub(crate) fn begin_exit(&mut self, reason: ApplicationExitReason) {
        if self.exit_reason.is_none() {
            self.exit_reason = Some(reason);
        }
        self.phase = ApplicationPhase::Exiting;
    }

    pub(crate) fn ensure_platform_exit(&mut self) {
        if self.exit_reason.is_none() {
            self.exit_reason = Some(ApplicationExitReason::Platform);
        }
        self.phase = ApplicationPhase::Exiting;
    }

    pub(crate) const fn exit_reason(&self) -> Option<ApplicationExitReason> {
        self.exit_reason
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
