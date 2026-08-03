use crate::NativeApp;
use crate::shell_interaction::SESSION_SIDEBAR_RESIZE_HANDLE;
use zeta_ui::{
    Point, Rect, SplitViewLayout, SplitViewLayoutPriority, SplitViewOrientation, SplitViewPane,
    SplitViewResizeSnapshot,
};
use zeta_winit::ElementState;
use zui::DispatchInvalidation;

const DEFAULT_WIDTH: f32 = 200.0;
const MINIMUM_WIDTH: f32 = 160.0;
const MAXIMUM_WIDTH: f32 = 480.0;
const MINIMUM_MAIN_WIDTH: f32 = 240.0;
const SIDEBAR_PANE_INDEX: usize = 0;
const MAIN_PANE_INDEX: usize = 1;

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
    resize: Option<SessionSidebarResize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SessionSidebarResize {
    pointer_origin: f32,
    snapshot: SplitViewResizeSnapshot,
    current_size: f32,
}

impl Default for SessionSidebarState {
    fn default() -> Self {
        Self {
            visibility: SessionSidebarVisibility::Collapsed,
            preferred_width: DEFAULT_WIDTH,
            resize: None,
        }
    }
}

impl SessionSidebarState {
    #[cfg(test)]
    pub(crate) const fn expanded() -> Self {
        Self {
            visibility: SessionSidebarVisibility::Expanded,
            preferred_width: DEFAULT_WIDTH,
            resize: None,
        }
    }

    pub(crate) const fn is_expanded(self) -> bool {
        matches!(self.visibility, SessionSidebarVisibility::Expanded)
    }

    pub(crate) const fn is_resizing(self) -> bool {
        self.resize.is_some()
    }

    pub(crate) fn toggle(&mut self) {
        self.visibility = match self.visibility {
            SessionSidebarVisibility::Collapsed => SessionSidebarVisibility::Expanded,
            SessionSidebarVisibility::Expanded => SessionSidebarVisibility::Collapsed,
        };
        self.resize = None;
    }

    #[cfg(test)]
    pub(crate) fn visible_width(self, viewport_width: f32) -> Option<f32> {
        let bounds = self
            .layout(Rect::from_xywh(0.0, 0.0, viewport_width, 1.0))
            .pane_bounds(SIDEBAR_PANE_INDEX)?;
        (bounds.size.width > 0.0).then_some(bounds.size.width)
    }

    pub(crate) fn layout(self, bounds: Rect) -> SplitViewLayout {
        let sidebar_is_visible =
            self.is_expanded() && bounds.size.width >= MINIMUM_WIDTH + MINIMUM_MAIN_WIDTH;
        let sidebar = SplitViewPane::new(self.preferred_width, MINIMUM_WIDTH, MAXIMUM_WIDTH);
        let sidebar = if sidebar_is_visible {
            sidebar
        } else {
            sidebar.hidden()
        };
        let main_preferred_width = if sidebar_is_visible {
            (bounds.size.width - self.preferred_width).max(0.0)
        } else {
            bounds.size.width
        };
        let main = SplitViewPane::new(main_preferred_width, MINIMUM_MAIN_WIDTH, f32::INFINITY)
            .with_priority(SplitViewLayoutPriority::High);
        SplitViewLayout::new(bounds, SplitViewOrientation::Horizontal, &[sidebar, main])
    }

    fn start_resizing(&mut self, viewport_width: f32, pointer_x: f32) -> bool {
        if self.resize.is_some() {
            return false;
        }
        let layout = self.layout(Rect::from_xywh(0.0, 0.0, viewport_width, 1.0));
        let Some(sash) = layout.sash(0) else {
            return false;
        };
        let snapshot = sash.resize_snapshot();
        self.resize = Some(SessionSidebarResize {
            pointer_origin: pointer_x,
            snapshot,
            current_size: snapshot.resize(0.0).previous_size(),
        });
        true
    }

    fn resize_to(&mut self, pointer_x: f32) -> bool {
        let Some(mut resize) = self.resize else {
            return false;
        };
        let next = resize.snapshot.resize(pointer_x - resize.pointer_origin);
        debug_assert_eq!(next.previous_index(), SIDEBAR_PANE_INDEX);
        debug_assert_eq!(next.next_index(), MAIN_PANE_INDEX);
        if next.previous_size() == resize.current_size {
            return false;
        }
        resize.current_size = next.previous_size();
        self.resize = Some(resize);
        self.preferred_width = next.previous_size();
        true
    }

    fn finish_resizing(&mut self) -> bool {
        if self.resize.is_none() {
            return false;
        }
        self.resize = None;
        true
    }
}

impl NativeApp {
    pub(super) fn route_session_sidebar_resize_move(&mut self, point: Point) -> bool {
        if !self.session_sidebar.is_resizing() {
            return false;
        }
        if self.session_sidebar.resize_to(point.x) {
            self.terminal_selection.clear();
            self.rebuild_presentation();
            self.request_redraw();
        }
        self.update_cursor();
        true
    }

    pub(super) fn route_session_sidebar_resize_button(&mut self, state: ElementState) -> bool {
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
                    || !self
                        .session_sidebar
                        .start_resizing(self.logical_viewport().width, point.x)
                {
                    return false;
                }
            }
            ElementState::Released => {
                if !self.session_sidebar.finish_resizing() {
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

    pub(super) fn cancel_session_sidebar_resize(&mut self) {
        if self.session_sidebar.finish_resizing() {
            self.rebuild_presentation();
            self.update_cursor();
            self.request_redraw();
        }
    }
}

#[cfg(test)]
#[path = "session_sidebar_tests.rs"]
mod tests;
