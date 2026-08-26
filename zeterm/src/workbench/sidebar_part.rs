use crate::NativeApp;
use crate::shell_interaction::AGENT_SIDEBAR_RESIZE_HANDLE;
use crate::shell_scene::sidebar_resize_snapshot_for_viewport;
use std::time::Instant;
use zeta_ui::Point;
use zeta_ui::Resizable;
use zeta_ui::SashOrientation;
use zeta_ui::SashPointerPresence;
use zeta_ui::SashState;
use zeta_ui::SplitViewResizeSnapshot;
use zeta_ui::layout::SidebarLayoutSpec;
use zeta_ui::layout::SidebarVisibility;
use zui::input::ElementState;
use zui::ui::DispatchInvalidation;

const DEFAULT_WIDTH: f32 = 520.0;
const MINIMUM_WIDTH: f32 = 360.0;
const MAXIMUM_WIDTH: f32 = 800.0;
const MINIMUM_MAIN_WIDTH: f32 = 400.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum SidebarPartVisibility {
    #[default]
    Collapsed,
    Expanded,
}

/// Runtime visibility and layout state for the right Sidebar Part.
///
/// Files and SCM feature content are owned by `zeta_agent_sidebar`; this type
/// only controls whether the SidebarPart participates in shell layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarPartState {
    visibility: SidebarPartVisibility,
    preferred_width: f32,
    resizable: Resizable,
}

impl Default for SidebarPartState {
    fn default() -> Self {
        Self {
            visibility: SidebarPartVisibility::Collapsed,
            preferred_width: DEFAULT_WIDTH,
            resizable: Resizable::new(SashOrientation::Vertical),
        }
    }
}

impl SidebarPartState {
    #[cfg(test)]
    pub(crate) const fn expanded() -> Self {
        Self {
            visibility: SidebarPartVisibility::Expanded,
            preferred_width: DEFAULT_WIDTH,
            resizable: Resizable::new(SashOrientation::Vertical),
        }
    }

    pub(crate) const fn is_expanded(self) -> bool {
        matches!(self.visibility, SidebarPartVisibility::Expanded)
    }

    /// Projects product visibility and persisted sizing into the host-neutral workspace layout
    /// contract. The layout crate owns pane geometry; this adapter owns product state.
    pub(crate) const fn layout_spec(self) -> SidebarLayoutSpec {
        let visibility = if self.is_expanded() {
            SidebarVisibility::Expanded
        } else {
            SidebarVisibility::Collapsed
        };
        SidebarLayoutSpec::new(
            visibility,
            self.preferred_width,
            MINIMUM_WIDTH,
            MAXIMUM_WIDTH,
            MINIMUM_MAIN_WIDTH,
        )
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
            SidebarPartVisibility::Collapsed => SidebarPartVisibility::Expanded,
            SidebarPartVisibility::Expanded => SidebarPartVisibility::Collapsed,
        };
        self.resizable.cancel();
    }

    pub(crate) fn expand(&mut self) {
        self.visibility = SidebarPartVisibility::Expanded;
        self.resizable.cancel();
    }

    pub(crate) fn start_resizing(
        &mut self,
        snapshot: SplitViewResizeSnapshot,
        pointer: Point,
        now: Instant,
    ) -> bool {
        self.resizable.begin_drag(snapshot, pointer, now)
    }

    pub(crate) fn resize_to(&mut self, pointer: Point) -> bool {
        let Some(next) = self.resizable.resize_to(pointer) else {
            return false;
        };
        self.preferred_width = next.next_size();
        true
    }

    pub(crate) fn finish_resizing(&mut self, presence: SashPointerPresence, now: Instant) -> bool {
        self.resizable.end_drag(presence, now)
    }

    pub(crate) fn cancel_resizing(&mut self) -> bool {
        self.resizable.cancel()
    }
}

impl NativeApp {
    pub(super) fn route_sidebar_resize_move(&mut self, point: Point) -> bool {
        if !self.sidebar_part.is_resizing() {
            return false;
        }
        if self.sidebar_part.resize_to(point) {
            self.terminal_selection.clear();
            self.rebuild_presentation();
            self.request_redraw();
        }
        self.update_cursor();
        true
    }

    pub(super) fn route_sidebar_resize_button(&mut self, state: ElementState) -> bool {
        let now = Instant::now();
        match state {
            ElementState::Pressed => {
                let Some(point) = self.cursor_position else {
                    return false;
                };
                let over_handle = self.presentation.as_ref().is_some_and(|presentation| {
                    presentation.interaction_frame().target_at(point)
                        == Some(AGENT_SIDEBAR_RESIZE_HANDLE)
                });
                let Some(snapshot) = sidebar_resize_snapshot_for_viewport(
                    self.logical_viewport(),
                    self.session_sidebar,
                    self.sidebar_part,
                ) else {
                    return false;
                };
                if !over_handle || !self.sidebar_part.start_resizing(snapshot, point, now) {
                    return false;
                }
            }
            ElementState::Released => {
                let presence = self.sash_pointer_presence(AGENT_SIDEBAR_RESIZE_HANDLE);
                if !self.sidebar_part.finish_resizing(presence, now) {
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

    pub(super) fn cancel_sidebar_resize(&mut self) {
        if self.sidebar_part.cancel_resizing() {
            self.rebuild_presentation();
            self.update_cursor();
            self.request_redraw();
        }
    }
}

#[cfg(test)]
#[path = "sidebar_part_tests.rs"]
mod tests;
