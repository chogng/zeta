use std::time::Instant;

use zeta_ui::{ScrollCommand, ScrollDelta, ScrollbarController, ScrollbarPresentation};
use zui::input::MouseScrollDelta;

use crate::NativeApp;
use crate::shell_interaction::{AGENT_EXPLORER_PANE, MULTI_DIFF_EDITOR};
use crate::terminal_projection::scroll_limit;
use zeta_agent_sidebar::FILE_LIST_ROW_HEIGHT;

const LINES_PER_WHEEL_STEP: f32 = 3.0;
const PIXELS_PER_LINE: f64 = 18.0;
const MULTI_DIFF_PIXELS_PER_LINE: f32 = 18.0;

/// Ephemeral viewport position over terminal-owned retained output.
#[derive(Default)]
pub(crate) struct TerminalScroll {
    offset: usize,
    fractional_lines: f64,
    scrollbar: ScrollbarController,
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

    pub(crate) fn scrollbar_activity(&mut self, now: Instant) {
        self.scrollbar.activity(now);
    }

    pub(crate) fn scrollbar_presentation(&self) -> ScrollbarPresentation {
        self.scrollbar.presentation()
    }

    pub(crate) fn advance_scrollbar(&mut self, now: Instant) -> bool {
        self.scrollbar.advance(now)
    }

    pub(crate) const fn scrollbar_deadline(&self) -> Option<Instant> {
        self.scrollbar.next_deadline()
    }

    pub(crate) fn cancel_scrollbar(&mut self) {
        self.scrollbar.cancel();
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
        if self.route_remote_tunnel_manager_wheel(delta) {
            return;
        }
        if self.route_remote_connection_manager_wheel(delta) {
            return;
        }
        if self.route_remote_connection_picker_wheel(delta) {
            return;
        }
        if self.route_workspace_path_picker_wheel(delta) {
            return;
        }
        if self.session_context_menu.is_open() {
            return;
        }
        if self.route_file_editor_wheel(delta) {
            return;
        }
        if self.route_multi_diff_wheel(delta) {
            return;
        }
        if self.route_file_list_wheel(delta) {
            return;
        }
        if self.route_composer_interaction_wheel(delta) {
            return;
        }
        if self.route_thread_timeline_wheel(delta) {
            return;
        }
        let position = self
            .cursor_position
            .and_then(|point| self.terminal_mouse_position(point));
        let modifiers = self.modifiers;
        let mut terminal_pointer = std::mem::take(&mut self.terminal_pointer);
        let captured = self.active_terminal_mut().map(|terminal| {
            match terminal_pointer.route_wheel(terminal, position, delta, modifiers) {
                Ok(captured) => captured,
                Err(error) => {
                    eprintln!("could not send terminal pointer input: {error}");
                    true
                }
            }
        });
        self.terminal_pointer = terminal_pointer;
        let Some(captured) = captured else {
            return;
        };
        if captured || position.is_none() {
            return;
        }
        let limit = self.terminal_scroll_limit();
        if self.terminal_scroll.scroll(delta, limit) {
            self.terminal_scroll.scrollbar_activity(Instant::now());
            self.terminal_selection.clear();
            self.rebuild_presentation();
            self.request_redraw();
        }
    }

    fn route_composer_interaction_wheel(&mut self, delta: MouseScrollDelta) -> bool {
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
            .contains(&crate::shell_interaction::COMPOSER_INTERACTION)
        {
            return false;
        }
        let Some(interaction_bounds) = presentation
            .accessibility_nodes
            .iter()
            .find(|node| node.id == crate::shell_interaction::COMPOSER_INTERACTION)
            .map(|node| node.bounds)
        else {
            return true;
        };
        let item_count = self
            .composer
            .interaction()
            .view()
            .map(|view| view.items().len())
            .unwrap_or(0);
        let viewport = zeta_composer::interaction_list_bounds(interaction_bounds);
        let content = zeta_composer::interaction_content_size(viewport, item_count);
        if self.composer.interaction_pane_mut().apply_scroll(
            composer_interaction_scroll_command(delta),
            viewport.size,
            content,
        ) {
            self.rebuild_presentation();
            self.request_redraw();
        }
        true
    }

    fn route_multi_diff_wheel(&mut self, delta: MouseScrollDelta) -> bool {
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
            .contains(&MULTI_DIFF_EDITOR)
        {
            return false;
        }
        let Some(viewport) = presentation
            .accessibility_nodes
            .iter()
            .find(|node| node.id == MULTI_DIFF_EDITOR)
            .map(|node| node.bounds.size)
        else {
            return false;
        };
        let changed = self.agent_sidebar_workspace.scroll_multi_diff(
            multi_diff_scroll_pixels(delta),
            viewport,
            std::time::Instant::now(),
        );
        if changed {
            self.rebuild_presentation_on_next_redraw();
        }
        true
    }

    fn route_file_list_wheel(&mut self, delta: MouseScrollDelta) -> bool {
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
            .contains(&AGENT_EXPLORER_PANE)
        {
            return false;
        }
        let Some(viewport) = presentation
            .accessibility_nodes
            .iter()
            .find(|node| node.id == AGENT_EXPLORER_PANE)
            .map(|node| node.bounds.size)
        else {
            return false;
        };
        let changed = self
            .agent_sidebar_workspace
            .scroll_file_list(file_list_scroll_pixels(delta), viewport);
        if changed {
            self.rebuild_presentation_on_next_redraw();
        }
        true
    }

    pub(super) fn terminal_scroll_limit(&self) -> usize {
        self.active_terminal()
            .map(|terminal| {
                scroll_limit(
                    terminal.core(),
                    terminal.core().grid().size().rows() as usize,
                )
            })
            .unwrap_or(0)
    }
}

fn composer_interaction_scroll_command(delta: MouseScrollDelta) -> ScrollCommand {
    let pixels = match delta {
        MouseScrollDelta::LineDelta(_, vertical) => {
            vertical * LINES_PER_WHEEL_STEP * zeta_composer::INTERACTION_ROW_HEIGHT
        }
        MouseScrollDelta::PixelDelta(position) => position.y as f32,
    };
    ScrollCommand::ByPixels(ScrollDelta::vertical(-pixels))
}

fn multi_diff_scroll_pixels(delta: MouseScrollDelta) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_, vertical) => {
            -vertical * LINES_PER_WHEEL_STEP * MULTI_DIFF_PIXELS_PER_LINE
        }
        MouseScrollDelta::PixelDelta(position) => -position.y as f32,
    }
}

fn file_list_scroll_pixels(delta: MouseScrollDelta) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_, vertical) => {
            -vertical * LINES_PER_WHEEL_STEP * FILE_LIST_ROW_HEIGHT
        }
        MouseScrollDelta::PixelDelta(position) => -position.y as f32,
    }
}

#[cfg(test)]
#[path = "terminal_scrollback_tests.rs"]
mod tests;
