use zeta_winit::MouseScrollDelta;

use crate::NativeApp;
use crate::terminal_projection::scroll_limit;

const LINES_PER_WHEEL_STEP: f32 = 3.0;
const PIXELS_PER_LINE: f64 = 18.0;

/// Ephemeral viewport position over terminal-owned retained output.
#[derive(Default)]
pub(crate) struct TerminalScroll {
    offset: usize,
    fractional_lines: f64,
}

impl TerminalScroll {
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
    pub(super) fn mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let position = self
            .cursor_position
            .and_then(|point| self.terminal_mouse_position(point));
        let captured = if let Some(terminal) = self.terminal.as_mut() {
            match self
                .terminal_pointer
                .route_wheel(terminal, position, delta, self.modifiers)
            {
                Ok(captured) => captured,
                Err(error) => {
                    eprintln!("could not send terminal pointer input: {error}");
                    true
                }
            }
        } else {
            return;
        };
        if captured || position.is_none() {
            return;
        }
        let limit = self.terminal_scroll_limit();
        if self.terminal_scroll.scroll(delta, limit) {
            self.terminal_selection.clear();
            self.rebuild_presentation();
            self.request_redraw();
        }
    }

    pub(super) fn terminal_scroll_limit(&self) -> usize {
        self.terminal
            .as_ref()
            .map(|terminal| {
                scroll_limit(
                    terminal.core(),
                    terminal.core().grid().size().rows() as usize,
                )
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
#[path = "terminal_scrollback_tests.rs"]
mod tests;
