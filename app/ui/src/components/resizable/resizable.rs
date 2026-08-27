use std::time::{Duration, Instant};

use super::{SashOrientation, SashState};
use crate::Point;
use crate::SplitViewResize;
use crate::SplitViewResizeSnapshot;

const DEFAULT_SASH_HOVER_DELAY: Duration = Duration::from_millis(300);

/// Whether a pointer is currently over a Sash interaction target.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum SashPointerPresence {
    #[default]
    Outside,
    Over,
}

/// Platform-independent Sash presentation state and hover timing.
///
/// The controller does not read platform events or install timers. Hosts project their current
/// pointer relationship, call [`Self::advance`] from the frame clock, and use
/// [`Self::next_deadline`] to schedule the next wake-up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SashController {
    hover_delay: Duration,
    state: SashState,
    hover_deadline: Option<Instant>,
}

impl SashController {
    /// Creates a controller with an explicit delayed-hover duration.
    pub const fn new(hover_delay: Duration) -> Self {
        Self {
            hover_delay,
            state: SashState::Resting,
            hover_deadline: None,
        }
    }

    /// Returns the presentation state consumed by [`super::Sash`].
    pub const fn presentation(self) -> SashState {
        self.state
    }

    /// Returns the next delayed-hover wake-up, if one is pending.
    pub const fn next_deadline(self) -> Option<Instant> {
        self.hover_deadline
    }

    /// Projects the pointer relationship without reading platform input directly.
    pub fn pointer_presence(&mut self, presence: SashPointerPresence, now: Instant) -> bool {
        match presence {
            SashPointerPresence::Outside => {
                if self.state == SashState::Active {
                    return false;
                }
                let changed = self.state != SashState::Resting;
                self.state = SashState::Resting;
                self.hover_deadline = None;
                changed
            }
            SashPointerPresence::Over => {
                if matches!(self.state, SashState::Hovered | SashState::Active)
                    || self.hover_deadline.is_some()
                {
                    return false;
                }
                self.hover_deadline = Some(
                    now.checked_add(self.hover_delay)
                        .expect("Sash hover deadline must remain within the Instant range"),
                );
                false
            }
        }
    }

    /// Enters the active presentation while the host owns a resize gesture.
    pub fn begin_drag(&mut self, _now: Instant) -> bool {
        let changed = self.state != SashState::Active || self.hover_deadline.is_some();
        self.state = SashState::Active;
        self.hover_deadline = None;
        changed
    }

    /// Leaves the active presentation and projects the pointer relationship after release.
    pub fn end_drag(&mut self, presence: SashPointerPresence, _now: Instant) -> bool {
        let next_state = match presence {
            SashPointerPresence::Outside => SashState::Resting,
            SashPointerPresence::Over => SashState::Hovered,
        };
        let changed = self.state != next_state || self.hover_deadline.is_some();
        self.state = next_state;
        self.hover_deadline = None;
        changed
    }

    /// Cancels pending hover or active presentation, for example after window deactivation.
    pub fn cancel(&mut self) -> bool {
        let changed = self.state != SashState::Resting || self.hover_deadline.is_some();
        self.state = SashState::Resting;
        self.hover_deadline = None;
        changed
    }

    /// Advances a pending delayed hover and reports whether the painted presentation changed.
    pub fn advance(&mut self, now: Instant) -> bool {
        let Some(deadline) = self.hover_deadline else {
            return false;
        };
        if now < deadline {
            return false;
        }
        self.hover_deadline = None;
        if self.state != SashState::Resting {
            return false;
        }
        self.state = SashState::Hovered;
        true
    }
}

impl Default for SashController {
    fn default() -> Self {
        Self::new(DEFAULT_SASH_HOVER_DELAY)
    }
}

/// Reusable resize gesture base for a caller-provided split snapshot.
///
/// `Resizable` owns only the Sash presentation controller and the drag-start-relative resize
/// calculation. The host owns pointer capture, pane visibility, preferred sizes, accessibility
/// identity, and the application of the returned [`SplitViewResize`] to product state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resizable {
    orientation: SashOrientation,
    sash: SashController,
    drag: Option<ResizableDrag>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResizableDrag {
    pointer_origin: f32,
    snapshot: SplitViewResizeSnapshot,
    current: SplitViewResize,
}

impl Resizable {
    /// Creates a vertical-axis or horizontal-axis resize base.
    pub const fn new(orientation: SashOrientation) -> Self {
        Self {
            orientation,
            sash: SashController::new(DEFAULT_SASH_HOVER_DELAY),
            drag: None,
        }
    }

    /// Creates a resize base with an explicit delayed-hover duration.
    pub const fn with_hover_delay(orientation: SashOrientation, hover_delay: Duration) -> Self {
        Self {
            orientation,
            sash: SashController::new(hover_delay),
            drag: None,
        }
    }

    pub const fn orientation(self) -> SashOrientation {
        self.orientation
    }

    pub const fn is_dragging(self) -> bool {
        self.drag.is_some()
    }

    pub const fn presentation(self) -> SashState {
        self.sash.presentation()
    }

    pub const fn next_deadline(self) -> Option<Instant> {
        self.sash.next_deadline()
    }

    /// Projects pointer presence to the embedded Sash controller.
    pub fn pointer_presence(&mut self, presence: SashPointerPresence, now: Instant) -> bool {
        self.sash.pointer_presence(presence, now)
    }

    /// Begins a drag from a layout snapshot and the current pointer position.
    pub fn begin_drag(
        &mut self,
        snapshot: SplitViewResizeSnapshot,
        pointer: Point,
        now: Instant,
    ) -> bool {
        if self.drag.is_some() {
            return false;
        }
        let pointer_origin = main_axis_coordinate(pointer, self.orientation);
        assert!(
            pointer_origin.is_finite(),
            "Resizable pointer coordinate must be finite"
        );
        self.drag = Some(ResizableDrag {
            pointer_origin,
            snapshot,
            current: snapshot.resize(0.0),
        });
        self.sash.begin_drag(now);
        true
    }

    /// Resolves the current pointer position against the drag-start snapshot.
    ///
    /// Returns `None` when no drag is active or when the constrained result is unchanged.
    pub fn resize_to(&mut self, pointer: Point) -> Option<SplitViewResize> {
        let mut drag = self.drag?;
        let coordinate = main_axis_coordinate(pointer, self.orientation);
        assert!(
            coordinate.is_finite(),
            "Resizable pointer coordinate must be finite"
        );
        let next = drag.snapshot.resize(coordinate - drag.pointer_origin);
        if next == drag.current {
            return None;
        }
        drag.current = next;
        self.drag = Some(drag);
        Some(next)
    }

    /// Ends the current drag and projects the pointer relationship after release.
    pub fn end_drag(&mut self, presence: SashPointerPresence, now: Instant) -> bool {
        if self.drag.take().is_none() {
            return false;
        }
        self.sash.end_drag(presence, now);
        true
    }

    /// Cancels the current drag and resets the Sash presentation.
    pub fn cancel(&mut self) -> bool {
        let presentation_changed = self.sash.cancel();
        let drag_changed = self.drag.take().is_some();
        drag_changed || presentation_changed
    }

    /// Advances the embedded delayed-hover presentation.
    pub fn advance(&mut self, now: Instant) -> bool {
        self.sash.advance(now)
    }
}

impl Default for Resizable {
    fn default() -> Self {
        Self::new(SashOrientation::default())
    }
}

fn main_axis_coordinate(point: Point, orientation: SashOrientation) -> f32 {
    match orientation {
        SashOrientation::Vertical => point.x,
        SashOrientation::Horizontal => point.y,
    }
}

#[cfg(test)]
#[path = "resizable_tests.rs"]
mod tests;
