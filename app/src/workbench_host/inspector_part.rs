use std::time::Instant;

use crate::NativeApp;
use crate::shell_interaction::INSPECTOR_RESIZE_HANDLE;
use crate::shell_scene::inspector_resize_snapshot_for_viewport;
use crate::workbench_host::InspectorPartState;
use zeta_ui::Point;
use zeta_workbench_layout::InspectorLayoutSpec;
use zeta_workbench_layout::PartVisibility;
use zui::input::ElementState;
use zui::ui::DispatchInvalidation;

const MINIMUM_WIDTH: f32 = 360.0;
const MAXIMUM_WIDTH: f32 = 800.0;
const MINIMUM_MAIN_WIDTH: f32 = 400.0;

pub(crate) fn inspector_layout_spec(
    inspector: InspectorPartState,
) -> InspectorLayoutSpec {
    InspectorLayoutSpec::new(
        if inspector.is_expanded() {
            PartVisibility::Expanded
        } else {
            PartVisibility::Collapsed
        },
        inspector.preferred_width(),
        MINIMUM_WIDTH,
        MAXIMUM_WIDTH,
        MINIMUM_MAIN_WIDTH,
    )
}

impl NativeApp {
    pub(crate) fn route_inspector_resize_move(&mut self, point: Point) -> bool {
        if !self.inspector_resizable.is_dragging() {
            return false;
        }
        if let Some(next) = self.inspector_resizable.resize_to(point)
            && self.inspector_part.set_preferred_width(next.next_size())
        {
            self.terminal_selection.clear();
            self.rebuild_presentation();
            self.request_redraw();
        }
        self.update_cursor();
        true
    }

    pub(crate) fn route_inspector_resize_button(&mut self, state: ElementState) -> bool {
        let now = Instant::now();
        match state {
            ElementState::Pressed => {
                let Some(point) = self.cursor_position else {
                    return false;
                };
                let over_handle = self.presentation.as_ref().is_some_and(|presentation| {
                    presentation.interaction_frame().target_at(point)
                        == Some(INSPECTOR_RESIZE_HANDLE)
                });
                let Some(snapshot) = inspector_resize_snapshot_for_viewport(
                    self.logical_viewport(),
                    self.tab_container,
                    self.inspector_part,
                ) else {
                    return false;
                };
                if !over_handle || !self.inspector_resizable.begin_drag(snapshot, point, now) {
                    return false;
                }
            }
            ElementState::Released => {
                let presence = self.sash_pointer_presence(INSPECTOR_RESIZE_HANDLE);
                if !self.inspector_resizable.end_drag(presence, now) {
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

    pub(crate) fn cancel_inspector_resize(&mut self) {
        if self.inspector_resizable.cancel() {
            self.rebuild_presentation();
            self.update_cursor();
            self.request_redraw();
        }
    }
}
