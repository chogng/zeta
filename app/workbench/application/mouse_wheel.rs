use std::time::Instant;

use zeta_ui_components::{ScrollCommand, ScrollDelta};
use zui::input::MouseScrollDelta;

use crate::WorkbenchApplication;
use crate::terminal_history::scroll_limit;
use crate::terminal_pointer::TerminalPointerRouting;
use zeta_files::FILE_LIST_ROW_HEIGHT;
use zeta_files::FILES_PANE;
use zeta_scm::MULTI_DIFF_EDITOR;

const LINES_PER_WHEEL_STEP: f32 = 3.0;
const TAB_CONTAINER_PIXELS_PER_LINE: f32 = 18.0;
const MULTI_DIFF_PIXELS_PER_LINE: f32 = 18.0;
const SETTINGS_PIXELS_PER_LINE: f32 = 18.0;

impl WorkbenchApplication {
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
        if self.route_directory_picker_wheel(delta) {
            return;
        }
        if self.workbench.tab_context_menu().is_open() {
            return;
        }
        if self.route_tab_container_wheel(delta) {
            return;
        }
        if self.route_settings_wheel(delta) {
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
        if let Some(point) = self.cursor_position {
            let _ = self.activate_terminal_pane_at(point);
        }
        let position = self
            .cursor_position
            .and_then(|point| self.terminal_mouse_position(point));
        let modifiers = self.modifiers;
        let mut terminal_pointer = std::mem::take(&mut self.terminal_view_mut().pointer);
        let captured = self.active_terminal_mut().map(|terminal| {
            match terminal_pointer.route_wheel(terminal, position, delta, modifiers) {
                Ok(captured) => captured,
                Err(error) => {
                    eprintln!("could not send terminal pointer input: {error}");
                    true
                }
            }
        });
        self.terminal_view_mut().pointer = terminal_pointer;
        let Some(captured) = captured else {
            return;
        };
        if captured || position.is_none() {
            return;
        }
        let limit = self.terminal_scroll_limit();
        if self.terminal_view_mut().scroll.scroll(delta, limit) {
            self.terminal_view_mut()
                .scroll
                .scrollbar_activity(Instant::now());
            self.terminal_view_mut().selection.clear();
            self.rebuild_presentation();
            self.request_redraw();
        }
    }

    fn route_tab_container_wheel(&mut self, delta: MouseScrollDelta) -> bool {
        let Some(point) = self.cursor_position else {
            return false;
        };
        let Some(presentation) = self.presentation.as_ref() else {
            return false;
        };
        let Some(bounds) = presentation.element_bounds(crate::TAB_CONTAINER) else {
            return false;
        };
        if !bounds.contains(point) {
            return false;
        }
        let Some(metrics) = presentation.tab_container_scroll_metrics else {
            return true;
        };
        if self
            .workbench
            .scroll_tab_container(tab_container_scroll_command(delta), metrics)
        {
            self.rebuild_presentation_on_next_redraw();
        }
        true
    }

    fn route_settings_wheel(&mut self, delta: MouseScrollDelta) -> bool {
        let Some(point) = self.cursor_position else {
            return false;
        };
        let Some(viewport) = self.settings_keybindings_viewport() else {
            return false;
        };
        let Some(bounds) = self.presentation.as_ref().and_then(|presentation| {
            presentation.element_bounds(zeta_settings::SETTINGS_KEYBINDINGS_LIST)
        }) else {
            return false;
        };
        if !bounds.contains(point) {
            return false;
        }
        if self.settings.scroll_keybindings(
            settings_scroll_command(delta),
            viewport,
            Instant::now(),
        ) {
            self.rebuild_presentation_on_next_redraw();
        }
        true
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
            .contains(&zeta_session::interaction::COMPOSER_INTERACTION)
        {
            return false;
        }
        let Some(interaction_bounds) =
            presentation.element_bounds(zeta_session::interaction::COMPOSER_INTERACTION)
        else {
            return true;
        };
        let item_count = self
            .session_pane
            .composer_interaction_view()
            .map(|view| view.items().len())
            .unwrap_or(0);
        let viewport = zeta_session::interaction_list_bounds(interaction_bounds);
        let content = zeta_session::interaction_content_size(viewport, item_count);
        if self.session_pane.scroll_composer_interaction(
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
            .element_bounds(MULTI_DIFF_EDITOR)
            .map(|bounds| bounds.size)
        else {
            return false;
        };
        let changed = self.scm.editor_mut().scroll(
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
            .contains(&FILES_PANE)
        {
            return false;
        }
        let Some(viewport) = presentation
            .element_bounds(FILES_PANE)
            .map(|bounds| bounds.size)
        else {
            return false;
        };
        let changed = self.files.scroll(file_list_scroll_pixels(delta), viewport);
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
            vertical * LINES_PER_WHEEL_STEP * zeta_session::INTERACTION_ROW_HEIGHT
        }
        MouseScrollDelta::PixelDelta(position) => position.y as f32,
    };
    ScrollCommand::ByPixels(ScrollDelta::vertical(-pixels))
}

fn tab_container_scroll_command(delta: MouseScrollDelta) -> ScrollCommand {
    let pixels = match delta {
        MouseScrollDelta::LineDelta(_, vertical) => {
            vertical * LINES_PER_WHEEL_STEP * TAB_CONTAINER_PIXELS_PER_LINE
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

fn settings_scroll_command(delta: MouseScrollDelta) -> ScrollCommand {
    let pixels = match delta {
        MouseScrollDelta::LineDelta(_, vertical) => {
            vertical * LINES_PER_WHEEL_STEP * SETTINGS_PIXELS_PER_LINE
        }
        MouseScrollDelta::PixelDelta(position) => position.y as f32,
    };
    ScrollCommand::ByPixels(ScrollDelta::vertical(-pixels))
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
#[path = "mouse_wheel_tests.rs"]
mod tests;
