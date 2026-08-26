//! Product-owned Sessions sidebar state and resize behavior.

use crate::NativeApp;
use crate::shell_interaction::SESSION_SIDEBAR_RESIZE_HANDLE;
use std::time::Instant;
use zeta_ui::Point;
use zeta_ui::Rect;
use zeta_ui::Resizable;
use zeta_ui::SashOrientation;
use zeta_ui::SashPointerPresence;
use zeta_ui::SashState;
use zeta_ui::layout::SessionSidebarLayout;
use zeta_ui::layout::SessionSidebarLayoutSpec;
use zeta_ui::layout::SidebarVisibility;
use zui::input::ElementState;
use zui::ui::DispatchInvalidation;

const DEFAULT_WIDTH: f32 = 200.0;
const MINIMUM_WIDTH: f32 = 160.0;
const MAXIMUM_WIDTH: f32 = 480.0;
const MINIMUM_MAIN_WIDTH: f32 = 240.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum SessionSidebarVisibility {
    Collapsed,
    Expanded,
}

/// Runtime layout state for the Sessions sidebar.
///
/// The preferred width survives visibility changes and temporary viewport
/// constraints. Pointer routing owns the resize lifecycle and the scene only
/// consumes the effective width returned for its current viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SessionSidebarState {
    visibility: SessionSidebarVisibility,
    preferred_width: f32,
    resizable: Resizable,
}

impl Default for SessionSidebarState {
    fn default() -> Self {
        Self {
            visibility: SessionSidebarVisibility::Expanded,
            preferred_width: DEFAULT_WIDTH,
            resizable: Resizable::new(SashOrientation::Vertical),
        }
    }
}

impl SessionSidebarState {
    #[cfg(test)]
    pub(crate) const fn expanded() -> Self {
        Self {
            visibility: SessionSidebarVisibility::Expanded,
            preferred_width: DEFAULT_WIDTH,
            resizable: Resizable::new(SashOrientation::Vertical),
        }
    }

    #[cfg(test)]
    pub(crate) const fn collapsed() -> Self {
        Self {
            visibility: SessionSidebarVisibility::Collapsed,
            preferred_width: DEFAULT_WIDTH,
            resizable: Resizable::new(SashOrientation::Vertical),
        }
    }

    pub(crate) const fn is_expanded(self) -> bool {
        matches!(self.visibility, SessionSidebarVisibility::Expanded)
    }

    pub(crate) const fn is_resizing(self) -> bool {
        self.resizable.is_dragging()
    }

    pub(crate) fn sash_pointer_presence(
        &mut self,
        presence: SashPointerPresence,
        now: Instant,
    ) -> bool {
        self.resizable.pointer_presence(presence, now)
    }

    pub(crate) fn advance_sash(&mut self, now: Instant) -> bool {
        self.resizable.advance(now)
    }

    pub(crate) const fn sash_state(self) -> SashState {
        self.resizable.presentation()
    }

    pub(crate) const fn sash_deadline(self) -> Option<Instant> {
        self.resizable.next_deadline()
    }

    pub(crate) fn toggle(&mut self) {
        self.visibility = match self.visibility {
            SessionSidebarVisibility::Collapsed => SessionSidebarVisibility::Expanded,
            SessionSidebarVisibility::Expanded => SessionSidebarVisibility::Collapsed,
        };
        self.resizable.cancel();
    }

    #[cfg(test)]
    pub(crate) fn visible_width(self, viewport_width: f32) -> Option<f32> {
        let bounds = self
            .layout(Rect::from_xywh(0.0, 0.0, viewport_width, 1.0))
            .sessions_bounds()?;
        (bounds.size.width > 0.0).then_some(bounds.size.width)
    }

    pub(crate) fn layout(self, bounds: Rect) -> SessionSidebarLayout {
        self.layout_spec().for_bounds(bounds)
    }

    pub(crate) fn layout_spec(self) -> SessionSidebarLayoutSpec {
        SessionSidebarLayoutSpec::new(
            if self.is_expanded() {
                SidebarVisibility::Expanded
            } else {
                SidebarVisibility::Collapsed
            },
            self.preferred_width,
            MINIMUM_WIDTH,
            MAXIMUM_WIDTH,
            MINIMUM_MAIN_WIDTH,
        )
    }

    fn start_resizing(&mut self, viewport_width: f32, pointer: Point, now: Instant) -> bool {
        let layout = self.layout(Rect::from_xywh(0.0, 0.0, viewport_width, 1.0));
        let Some(snapshot) = layout.resize_snapshot() else {
            return false;
        };
        self.resizable.begin_drag(snapshot, pointer, now)
    }

    fn resize_to(&mut self, pointer: Point) -> bool {
        let Some(next) = self.resizable.resize_to(pointer) else {
            return false;
        };
        self.preferred_width = next.previous_size();
        true
    }

    fn finish_resizing(&mut self, presence: SashPointerPresence, now: Instant) -> bool {
        self.resizable.end_drag(presence, now)
    }

    fn cancel_resizing(&mut self) -> bool {
        self.resizable.cancel()
    }
}

impl NativeApp {
    pub(crate) fn route_session_sidebar_resize_move(&mut self, point: Point) -> bool {
        if !self.session_sidebar.is_resizing() {
            return false;
        }
        if self.session_sidebar.resize_to(point) {
            self.terminal_selection.clear();
            self.rebuild_presentation();
            self.request_redraw();
        }
        self.update_cursor();
        true
    }

    pub(crate) fn route_session_sidebar_resize_button(&mut self, state: ElementState) -> bool {
        let now = Instant::now();
        match state {
            ElementState::Pressed => {
                let Some(point) = self.cursor_position else {
                    return false;
                };
                let over_handle = self.presentation.as_ref().is_some_and(|presentation| {
                    presentation.interaction_frame().target_at(point)
                        == Some(SESSION_SIDEBAR_RESIZE_HANDLE)
                });
                if !over_handle
                    || !self.session_sidebar.start_resizing(
                        self.logical_viewport().width,
                        point,
                        now,
                    )
                {
                    return false;
                }
            }
            ElementState::Released => {
                let presence = self.sash_pointer_presence(SESSION_SIDEBAR_RESIZE_HANDLE);
                if !self.session_sidebar.finish_resizing(presence, now) {
                    return false;
                }
            }
        }
        self.rebuild_presentation();
        let hover_changed = self
            .cursor_position
            .zip(self.presentation.as_ref())
            .is_some_and(|(point, presentation)| {
                self.ui_dispatch
                    .pointer_moved(point, presentation.interaction_frame())
                    .invalidation
                    == DispatchInvalidation::Paint
            });
        if hover_changed {
            self.rebuild_presentation();
        }
        self.update_cursor();
        self.request_redraw();
        true
    }

    pub(crate) fn cancel_session_sidebar_resize(&mut self) {
        if self.session_sidebar.cancel_resizing() {
            self.rebuild_presentation();
            self.update_cursor();
            self.request_redraw();
        }
    }
}

#[cfg(test)]
#[path = "session_sidebar_tests.rs"]
mod tests;
