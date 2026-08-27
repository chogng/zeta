use std::time::Instant;

use crate::NativeApp;
use crate::shell_interaction::TAB_CONTAINER_RESIZE_HANDLE;
use zeta_ui::Point;
use zui::input::ElementState;
use zui::ui::DispatchInvalidation;

impl NativeApp {
    pub(super) fn route_tab_container_resize_move(&mut self, point: Point) -> bool {
        if !self.tab_container.is_resizing() {
            return false;
        }
        if self.tab_container.resize_to(point) {
            self.terminal_selection.clear();
            self.rebuild_presentation();
            self.request_redraw();
        }
        self.update_cursor();
        true
    }

    pub(super) fn route_tab_container_resize_button(&mut self, state: ElementState) -> bool {
        let now = Instant::now();
        match state {
            ElementState::Pressed => {
                let Some(point) = self.cursor_position else {
                    return false;
                };
                let over_handle = self.presentation.as_ref().is_some_and(|presentation| {
                    presentation.interaction_frame().target_at(point)
                        == Some(TAB_CONTAINER_RESIZE_HANDLE)
                });
                if !over_handle
                    || !self
                        .tab_container
                        .start_resizing(self.logical_viewport().width, point, now)
                {
                    return false;
                }
            }
            ElementState::Released => {
                let presence = self.sash_pointer_presence(TAB_CONTAINER_RESIZE_HANDLE);
                if !self.tab_container.finish_resizing(presence, now) {
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

    pub(super) fn cancel_tab_container_resize(&mut self) {
        if self.tab_container.cancel_resizing() {
            self.rebuild_presentation();
            self.update_cursor();
            self.request_redraw();
        }
    }
}
