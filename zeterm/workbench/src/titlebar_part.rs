use crate::Pane;
use crate::PaneId;
use crate::PaneInput;
use crate::PanePart;
use crate::PaneSplitDirection;

/// The Workbench command surface.
///
/// The titlebar does not own Tab Part or Pane Part state. It is the narrow coordination boundary
/// used by titlebar actions to mutate the owning Part through [`Workbench`](crate::Workbench).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TitlebarPart;

impl TitlebarPart {
    /// Creates a titlebar Part.
    pub const fn new() -> Self {
        Self
    }

    /// Creates a horizontally split pane for a tab item.
    pub fn create_pane(&self, pane_part: &mut PanePart, input: PaneInput) -> PaneId {
        pane_part
            .split_active_with_input(PaneSplitDirection::Horizontal, Some(input))
            .0
    }

    /// Creates a pane in the requested split direction for a tab item.
    pub fn create_pane_with_direction(
        &self,
        pane_part: &mut PanePart,
        input: PaneInput,
        direction: PaneSplitDirection,
    ) -> PaneId {
        pane_part.split_active_with_input(direction, Some(input)).0
    }

    /// Destroys the active group for a tab item and returns all of its inputs.
    pub fn destroy_active_panes(&self, pane_part: &mut PanePart) -> Option<Vec<Pane>> {
        pane_part.destroy_active_panes()
    }

    /// Destroys a specific group for a tab item and returns all of its inputs.
    pub fn destroy_panes(&self, pane_part: &mut PanePart, pane_id: PaneId) -> Option<Vec<Pane>> {
        pane_part.destroy_panes(pane_id)
    }
}
