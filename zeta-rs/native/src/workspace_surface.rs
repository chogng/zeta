/// Top-level product surface shown in the main workspace.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum WorkspaceSurface {
    #[default]
    Agent,
    Terminal,
}

impl WorkspaceSurface {
    pub(crate) fn toggle(&mut self) {
        *self = match self {
            Self::Agent => Self::Terminal,
            Self::Terminal => Self::Agent,
        };
    }

    pub(crate) const fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal)
    }
}

#[cfg(test)]
#[path = "workspace_surface_tests.rs"]
mod tests;
