use crate::Pane;
use crate::PaneInput;
use crate::PanePart;

/// Complete pane state owned by one Workbench tab.
///
/// A container owns one pane-group layout and the tab-local state that survives changes to the
/// active pane input. Switching Workbench tabs switches containers as a unit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaneContainer {
    pane_part: PanePart,
    workspace_return: Option<PaneInput>,
}

impl PaneContainer {
    /// Creates a container with one empty root pane group.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the pane-group topology owned by this container.
    pub const fn pane_part(&self) -> &PanePart {
        &self.pane_part
    }

    /// Returns the mutable pane-group topology owned by this container.
    pub(crate) const fn pane_part_mut(&mut self) -> &mut PanePart {
        &mut self.pane_part
    }

    /// Saves a logical input to restore when this container returns from Files or Diff.
    pub(crate) fn remember_workspace_return(&mut self, input: PaneInput) {
        self.workspace_return = Some(input);
    }

    /// Takes the logical input saved for this container, if any.
    pub(crate) fn take_workspace_return(&mut self) -> Option<PaneInput> {
        self.workspace_return.take()
    }

    /// Drops the logical input saved for this container.
    pub(crate) fn clear_workspace_return(&mut self) {
        self.workspace_return = None;
    }

    /// Takes every pane from every group in visual tree order.
    pub(crate) fn take_panes(self) -> Vec<Pane> {
        self.pane_part.take_panes()
    }
}

#[cfg(test)]
#[path = "pane_container_tests.rs"]
mod tests;
