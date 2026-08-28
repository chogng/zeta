/// Product surface currently projected into the central workspace.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WorkspaceSurfaceKind {
    #[default]
    Agent,
    Editor,
    Terminal,
}

/// Central workspace selection with a reversible terminal overlay transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkspaceSurface {
    active: WorkspaceSurfaceKind,
    terminal_return: WorkspaceSurfaceKind,
}

impl Default for WorkspaceSurface {
    fn default() -> Self {
        Self {
            active: WorkspaceSurfaceKind::Agent,
            terminal_return: WorkspaceSurfaceKind::Agent,
        }
    }
}

impl WorkspaceSurface {
    pub const fn active(self) -> WorkspaceSurfaceKind {
        self.active
    }

    pub const fn is_editor(self) -> bool {
        matches!(self.active, WorkspaceSurfaceKind::Editor)
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self.active, WorkspaceSurfaceKind::Terminal)
    }

    pub fn show_agent(&mut self) {
        self.active = WorkspaceSurfaceKind::Agent;
        self.terminal_return = WorkspaceSurfaceKind::Agent;
    }

    pub fn show_editor(&mut self) {
        self.active = WorkspaceSurfaceKind::Editor;
        self.terminal_return = WorkspaceSurfaceKind::Editor;
    }

    pub fn toggle_terminal(&mut self) {
        if self.is_terminal() {
            self.active = self.terminal_return;
        } else {
            self.terminal_return = self.active;
            self.active = WorkspaceSurfaceKind::Terminal;
        }
    }
}

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;
