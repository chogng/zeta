use std::time::Instant;

use crate::NativeApp;
use crate::shell_interaction::AGENT_SIDEBAR_RESIZE_HANDLE;
use crate::shell_scene::sidebar_resize_snapshot_for_viewport;
use zeta_ui::Point;
use zui::input::ElementState;
use zui::ui::DispatchInvalidation;

pub(crate) use zeta_workbench::SidebarPartState;

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
