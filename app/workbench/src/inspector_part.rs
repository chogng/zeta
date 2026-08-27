use std::time::Instant;

use zeta_ui::Point;
use zeta_ui::Resizable;
use zeta_ui::SashOrientation;
use zeta_ui::SashPointerPresence;
use zeta_ui::SashState;
use zeta_ui::SplitViewResizeSnapshot;
use zeta_ui::layout::InspectorLayoutSpec;
use zeta_ui::layout::PartVisibility;

const DEFAULT_WIDTH: f32 = 520.0;
const MINIMUM_WIDTH: f32 = 360.0;
const MAXIMUM_WIDTH: f32 = 800.0;
const MINIMUM_MAIN_WIDTH: f32 = 400.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum InspectorPartVisibility {
    #[default]
    Collapsed,
    Expanded,
}

/// Runtime visibility and layout state for the right Workbench Inspector.
///
/// Feature content is owned by the product host. This type only controls whether the Inspector
/// participates in workbench layout and how its width is resized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InspectorPartState {
    visibility: InspectorPartVisibility,
    preferred_width: f32,
    resizable: Resizable,
}

impl Default for InspectorPartState {
    fn default() -> Self {
        Self {
            visibility: InspectorPartVisibility::Collapsed,
            preferred_width: DEFAULT_WIDTH,
            resizable: Resizable::new(SashOrientation::Vertical),
        }
    }
}

impl InspectorPartState {
    /// Creates an expanded Inspector with the default width.
    pub const fn expanded() -> Self {
        Self {
            visibility: InspectorPartVisibility::Expanded,
            preferred_width: DEFAULT_WIDTH,
            resizable: Resizable::new(SashOrientation::Vertical),
        }
    }

    pub const fn is_expanded(self) -> bool {
        matches!(self.visibility, InspectorPartVisibility::Expanded)
    }

    /// Projects visibility and persisted sizing into the host-neutral workbench layout contract.
    pub const fn layout_spec(self) -> InspectorLayoutSpec {
        let visibility = if self.is_expanded() {
            PartVisibility::Expanded
        } else {
            PartVisibility::Collapsed
        };
        InspectorLayoutSpec::new(
            visibility,
            self.preferred_width,
            MINIMUM_WIDTH,
            MAXIMUM_WIDTH,
            MINIMUM_MAIN_WIDTH,
        )
    }

    pub const fn is_resizing(self) -> bool {
        self.resizable.is_dragging()
    }

    pub fn sash_pointer_presence(&mut self, presence: SashPointerPresence, now: Instant) -> bool {
        self.resizable.pointer_presence(presence, now)
    }

    pub fn advance_sash(&mut self, now: Instant) -> bool {
        self.resizable.advance(now)
    }

    pub const fn sash_state(self) -> SashState {
        self.resizable.presentation()
    }

    pub const fn sash_deadline(self) -> Option<Instant> {
        self.resizable.next_deadline()
    }

    pub fn toggle(&mut self) {
        self.visibility = match self.visibility {
            InspectorPartVisibility::Collapsed => InspectorPartVisibility::Expanded,
            InspectorPartVisibility::Expanded => InspectorPartVisibility::Collapsed,
        };
        self.resizable.cancel();
    }

    pub fn expand(&mut self) {
        self.visibility = InspectorPartVisibility::Expanded;
        self.resizable.cancel();
    }

    pub fn collapse(&mut self) {
        self.visibility = InspectorPartVisibility::Collapsed;
        self.resizable.cancel();
    }

    pub fn start_resizing(
        &mut self,
        snapshot: SplitViewResizeSnapshot,
        pointer: Point,
        now: Instant,
    ) -> bool {
        self.resizable.begin_drag(snapshot, pointer, now)
    }

    pub fn resize_to(&mut self, pointer: Point) -> bool {
        let Some(next) = self.resizable.resize_to(pointer) else {
            return false;
        };
        self.preferred_width = next.next_size();
        true
    }

    pub fn finish_resizing(&mut self, presence: SashPointerPresence, now: Instant) -> bool {
        self.resizable.end_drag(presence, now)
    }

    pub fn cancel_resizing(&mut self) -> bool {
        self.resizable.cancel()
    }
}

#[cfg(test)]
#[path = "inspector_part_tests.rs"]
mod tests;
