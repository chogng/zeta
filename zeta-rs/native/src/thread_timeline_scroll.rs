use zeta_winit::MouseScrollDelta;

use crate::NativeApp;
use crate::shell_interaction::THREAD_TIMELINE;
use crate::thread_timeline::{line_capacity, line_count};

const LINES_PER_WHEEL_STEP: f32 = 3.0;
const PIXELS_PER_LINE: f64 = 20.0;

/// Ephemeral scroll position over the current Agent Thread timeline.
#[derive(Default)]
pub(crate) struct ThreadTimelineScroll {
    offset: usize,
    fractional_lines: f64,
}

impl ThreadTimelineScroll {
    pub(crate) const fn offset(&self) -> usize {
        self.offset
    }

    pub(crate) fn scroll(&mut self, delta: MouseScrollDelta, limit: usize) -> bool {
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, vertical) => f64::from(vertical * LINES_PER_WHEEL_STEP),
            MouseScrollDelta::PixelDelta(position) => position.y / PIXELS_PER_LINE,
        };
        if lines.signum() != self.fractional_lines.signum() {
            self.fractional_lines = 0.0;
        }
        self.fractional_lines += lines;
        let whole_lines = self.fractional_lines.trunc() as isize;
        self.fractional_lines -= whole_lines as f64;
        let previous = self.offset;
        if whole_lines > 0 {
            self.offset = self.offset.saturating_add(whole_lines as usize).min(limit);
        } else {
            self.offset = self.offset.saturating_sub(whole_lines.unsigned_abs());
        }
        self.offset != previous
    }

    pub(crate) fn preserve_view_after_growth(&mut self, added_lines: usize, limit: usize) {
        if self.offset > 0 {
            self.offset = self.offset.saturating_add(added_lines).min(limit);
        }
    }

    pub(crate) fn clamp(&mut self, limit: usize) {
        self.offset = self.offset.min(limit);
    }

    pub(crate) fn reset(&mut self) {
        self.offset = 0;
        self.fractional_lines = 0.0;
    }
}

impl NativeApp {
    pub(super) fn route_thread_timeline_wheel(&mut self, delta: MouseScrollDelta) -> bool {
        let Some(point) = self.cursor_position else {
            return false;
        };
        let Some(presentation) = self.presentation.as_ref() else {
            return false;
        };
        let Some(target) = presentation.interaction_frame.target_at(point) else {
            return false;
        };
        if !presentation
            .interaction_frame
            .ancestry(target)
            .contains(&THREAD_TIMELINE)
        {
            return false;
        }
        let Some(bounds) = presentation
            .accessibility_nodes
            .iter()
            .find(|node| node.id == THREAD_TIMELINE)
            .map(|node| node.bounds)
        else {
            return false;
        };
        let limit = line_count(&self.thread_projection).saturating_sub(line_capacity(bounds));
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
        let Some(bounds) = presentation
            .accessibility_nodes
            .iter()
            .find(|node| node.id == THREAD_TIMELINE)
            .map(|node| node.bounds)
        else {
            return 0;
        };
        line_count(&self.thread_projection).saturating_sub(line_capacity(bounds))
    }
}

#[cfg(test)]
#[path = "thread_timeline_scroll_tests.rs"]
mod tests;
