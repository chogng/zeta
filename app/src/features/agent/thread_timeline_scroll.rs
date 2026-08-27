//! Host event adapter for the Session UI timeline scroll state.

use zui::input::MouseScrollDelta;

use crate::NativeApp;
use crate::shell_interaction::THREAD_TIMELINE;
use crate::thread_timeline::{line_capacity, line_count};

pub(crate) use zeta_session::ThreadTimelineScroll;
use zeta_session::TimelineScrollDelta;

impl NativeApp {
    pub(super) fn route_thread_timeline_wheel(&mut self, delta: MouseScrollDelta) -> bool {
        let Some(point) = self.cursor_position else {
            return false;
        };
        let Some(presentation) = self.presentation.as_ref() else {
            return false;
        };
        let Some(target) = presentation.interaction_frame().target_at(point) else {
            return false;
        };
        if !presentation
            .interaction_frame()
            .ancestry(target)
            .contains(&THREAD_TIMELINE)
        {
            return false;
        }
        let Some(bounds) = presentation.element_bounds(THREAD_TIMELINE) else {
            return false;
        };
        let limit = line_count(&self.thread_projection).saturating_sub(line_capacity(bounds));
        let delta = match delta {
            MouseScrollDelta::LineDelta(_, vertical) => TimelineScrollDelta::Lines(vertical),
            MouseScrollDelta::PixelDelta(position) => TimelineScrollDelta::Pixels(position.y),
        };
        if self.thread_timeline_scroll.scroll(delta, limit) {
            self.rebuild_presentation();
            self.request_redraw();
        }
        true
    }

    pub(crate) fn thread_timeline_scroll_limit(&self) -> usize {
        let Some(presentation) = self.presentation.as_ref() else {
            return 0;
        };
        let Some(bounds) = presentation.element_bounds(THREAD_TIMELINE) else {
            return 0;
        };
        line_count(&self.thread_projection).saturating_sub(line_capacity(bounds))
    }
}
