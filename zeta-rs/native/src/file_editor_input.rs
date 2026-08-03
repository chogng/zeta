use std::{ops::Range, time::Instant};

use zeta_editor::{CodeEditorCommand, CodeEditorPosition, CodeEditorSelectionMode};
use zeta_language_service::LanguageRequestKind;
use zeta_ui::TextInputCompositionEvent;
use zeta_winit::{ElementState, Key, KeyEvent, MouseScrollDelta, NamedKey};

use crate::NativeApp;
use crate::file_editor_auto_scroll::{FileEditorAutoScrollDirection, FileEditorAutoScrollState};
use crate::file_editor_host::FileEditorCloseRequest;
use crate::file_editor_pane::{FileEditorPane, FileEditorPrompt};
use crate::shell_interaction::{
    FILE_EDITOR_DOCUMENT, FILE_EDITOR_FIND_INPUT, FILE_EDITOR_PANE, FILE_EDITOR_REPLACE_INPUT,
    FileEditorAction, file_editor_close_index, file_editor_fold_index, file_editor_tab_index,
};
use crate::terminal_input::{code_editor_command, text_input_command};
use crate::terminal_selection::{read_clipboard_text, write_clipboard_text};

const ROWS_PER_WHEEL_STEP: f64 = 3.0;

/// Ephemeral platform-input state that is not part of an editor document or file lifecycle.
#[derive(Default)]
pub(crate) struct FileEditorInputState {
    dragging_selection: bool,
    fractional_rows: f64,
    prompt: FileEditorPrompt,
    auto_scroll: FileEditorAutoScrollState,
    hovered_diagnostic: Option<Range<usize>>,
    hovered_language_position: Option<CodeEditorPosition>,
    completion_selection: usize,
}

impl FileEditorInputState {
    pub(crate) fn cancel_pointer(&mut self) {
        self.dragging_selection = false;
        self.auto_scroll.stop();
        self.hovered_diagnostic = None;
        self.hovered_language_position = None;
        self.completion_selection = 0;
    }

    pub(crate) fn reset_for_document_change(&mut self) {
        self.dragging_selection = false;
        self.fractional_rows = 0.0;
        self.prompt = FileEditorPrompt::None;
        self.auto_scroll.stop();
        self.hovered_diagnostic = None;
        self.hovered_language_position = None;
        self.completion_selection = 0;
    }

    pub(crate) const fn prompt(&self) -> FileEditorPrompt {
        self.prompt
    }

    fn confirm_close(&mut self) {
        self.prompt = FileEditorPrompt::ConfirmClose;
    }

    fn dismiss_prompt(&mut self) {
        self.prompt = FileEditorPrompt::None;
    }

    fn begin_selection(&mut self) {
        self.dragging_selection = true;
        self.auto_scroll.stop();
    }

    fn end_selection(&mut self) {
        self.dragging_selection = false;
        self.auto_scroll.stop();
    }

    fn is_selecting(&self) -> bool {
        self.dragging_selection
    }

    fn update_hovered_diagnostic(&mut self, range: Option<Range<usize>>) -> bool {
        if self.hovered_diagnostic == range {
            return false;
        }
        self.hovered_diagnostic = range;
        true
    }

    fn update_hovered_language_position(&mut self, position: Option<CodeEditorPosition>) -> bool {
        if self.hovered_language_position == position {
            return false;
        }
        self.hovered_language_position = position;
        true
    }

    fn move_completion_selection(&mut self, delta: isize, item_count: usize) {
        if item_count == 0 {
            self.completion_selection = 0;
            return;
        }
        self.completion_selection = self
            .completion_selection
            .saturating_add_signed(delta)
            .min(item_count - 1);
    }

    pub(crate) const fn auto_scroll_deadline(&self) -> Option<Instant> {
        self.auto_scroll.deadline()
    }

    pub(crate) const fn completion_selection(&self) -> usize {
        self.completion_selection
    }

    fn wheel_rows(&mut self, delta: MouseScrollDelta) -> isize {
        let rows = match delta {
            MouseScrollDelta::LineDelta(_, vertical) => -f64::from(vertical) * ROWS_PER_WHEEL_STEP,
            MouseScrollDelta::PixelDelta(position) => {
                -position.y / f64::from(zeta_editor::CodeEditor::row_height())
            }
        };
        if rows.signum() != self.fractional_rows.signum() {
            self.fractional_rows = 0.0;
        }
        self.fractional_rows += rows;
        let whole_rows = self.fractional_rows.trunc() as isize;
        self.fractional_rows -= whole_rows as f64;
        whole_rows
    }
}

impl NativeApp {
    pub(super) fn file_editor_keyboard_input(&mut self, event: &KeyEvent) -> bool {
        if !self.workspace_surface.is_editor() {
            return false;
        }
        let shortcut = self.modifiers.control_key() || self.modifiers.super_key();
        if shortcut
            && !self.modifiers.alt_key()
            && matches!(&event.logical_key, Key::Character(text) if text.eq_ignore_ascii_case("f"))
        {
            self.file_editor_search.show_find();
            self.pending_focus = Some(FILE_EDITOR_FIND_INPUT);
            self.file_editor_changed();
            return true;
        }
        let replace_shortcut = (self.modifiers.control_key()
            && matches!(&event.logical_key, Key::Character(text) if text.eq_ignore_ascii_case("h")))
            || (self.modifiers.super_key()
                && self.modifiers.alt_key()
                && matches!(&event.logical_key, Key::Character(text) if text.eq_ignore_ascii_case("f")));
        if replace_shortcut {
            self.file_editor_search.show_replace();
            self.pending_focus = Some(FILE_EDITOR_FIND_INPUT);
            self.file_editor_changed();
            return true;
        }
        if self.file_editor_input.prompt() == FileEditorPrompt::ConfirmClose
            && event.logical_key == Key::Named(NamedKey::Escape)
        {
            self.file_editor_input.dismiss_prompt();
            self.pending_focus = Some(FILE_EDITOR_DOCUMENT);
            self.file_editor_changed();
            return true;
        }
        if !self.ui_dispatch.is_focused(FILE_EDITOR_DOCUMENT) {
            return self.file_editor_search_keyboard_input(event);
        }
        if let Some(completions) = self
            .language_service
            .active_completions(&self.file_editor_host)
        {
            let item_count = completions.items.len();
            if event.logical_key == Key::Named(NamedKey::Escape) {
                self.language_service.dismiss_completions();
                self.file_editor_changed();
                return true;
            }
            if event.logical_key == Key::Named(NamedKey::ArrowDown) {
                self.file_editor_input
                    .move_completion_selection(1, item_count);
                self.file_editor_changed();
                return true;
            }
            if event.logical_key == Key::Named(NamedKey::ArrowUp) {
                self.file_editor_input
                    .move_completion_selection(-1, item_count);
                self.file_editor_changed();
                return true;
            }
            if event.logical_key == Key::Named(NamedKey::Enter) {
                let edit = completions
                    .items
                    .get(self.file_editor_input.completion_selection)
                    .and_then(|item| item.edit.clone());
                self.language_service.dismiss_completions();
                if let Some(edit) = edit {
                    self.file_editor_host.apply_language_completion(&edit);
                    self.reveal_file_editor_caret();
                }
                self.file_editor_changed();
                return true;
            }
        }
        if event.logical_key == Key::Named(NamedKey::F3) {
            self.find_file_editor_match(self.modifiers.shift_key());
            return true;
        }
        if event.logical_key == Key::Named(NamedKey::F12) {
            self.language_service
                .request_active(&self.file_editor_host, LanguageRequestKind::Definition);
            self.rebuild_presentation();
            self.request_redraw();
            return true;
        }
        if shortcut && matches!(&event.logical_key, Key::Character(text) if text == " ") {
            self.language_service
                .request_active(&self.file_editor_host, LanguageRequestKind::Completion);
            self.rebuild_presentation();
            self.request_redraw();
            return true;
        }
        if event.logical_key == Key::Named(NamedKey::Escape) {
            self.file_editor_host.cancel_active_composition();
            self.file_editor_changed();
            return true;
        }
        let command = if event.logical_key == Key::Named(NamedKey::Enter)
            && !(self.modifiers.control_key() || self.modifiers.super_key())
        {
            Some(CodeEditorCommand::Newline)
        } else {
            code_editor_command(event, self.modifiers)
        };
        let Some(command) = command else {
            return false;
        };
        self.language_service.dismiss_completions();
        let navigation = self
            .file_editor_pane()
            .map(|pane| pane.navigation())
            .unwrap_or_default();
        self.file_editor_host.apply_in_view(command, navigation);
        self.reveal_file_editor_caret();
        self.file_editor_changed();
        true
    }

    fn file_editor_search_keyboard_input(&mut self, event: &KeyEvent) -> bool {
        let query_focused = self.ui_dispatch.is_focused(FILE_EDITOR_FIND_INPUT);
        let replacement_focused = self.ui_dispatch.is_focused(FILE_EDITOR_REPLACE_INPUT);
        if !query_focused && !replacement_focused {
            return false;
        }
        if event.logical_key == Key::Named(NamedKey::Escape) {
            self.file_editor_search.hide();
            self.pending_focus = Some(FILE_EDITOR_DOCUMENT);
            self.file_editor_changed();
            return true;
        }
        if event.logical_key == Key::Named(NamedKey::Enter) {
            if replacement_focused {
                self.replace_current_file_editor_match();
            } else {
                self.find_file_editor_match(self.modifiers.shift_key());
            }
            return true;
        }
        let Some(command) = text_input_command(event, self.modifiers) else {
            return false;
        };
        let previous_query = self.file_editor_search.query_input().text().to_owned();
        if query_focused {
            self.file_editor_search.apply_query(command);
        } else {
            self.file_editor_search.apply_replacement(command);
        }
        if query_focused && self.file_editor_search.query_input().text() != previous_query {
            let query = self.file_editor_search.query();
            if !query.text().is_empty() {
                self.file_editor_host.find_nearest(&query);
                self.reveal_file_editor_caret();
            }
        }
        self.file_editor_changed();
        true
    }

    fn find_file_editor_match(&mut self, previous: bool) {
        let query = self.file_editor_search.query();
        if query.text().is_empty() {
            return;
        }
        if previous {
            self.file_editor_host.find_previous(&query);
        } else {
            self.file_editor_host.find_next(&query);
        }
        self.reveal_file_editor_caret();
        self.file_editor_changed();
    }

    fn replace_current_file_editor_match(&mut self) {
        let query = self.file_editor_search.query();
        if query.text().is_empty() {
            return;
        }
        let replacement = self.file_editor_search.replacement().to_owned();
        if self.file_editor_host.replace_current(&query, &replacement) {
            self.file_editor_host.find_next(&query);
        }
        self.reveal_file_editor_caret();
        self.file_editor_changed();
    }

    pub(super) fn apply_file_editor_composition(&mut self, event: TextInputCompositionEvent) {
        self.file_editor_host.apply_composition(event);
        self.reveal_file_editor_caret();
        self.file_editor_changed();
    }

    pub(super) fn route_file_editor_pointer_move(&mut self) -> bool {
        if !self.file_editor_input.is_selecting() {
            let hovered = self
                .cursor_position
                .and_then(|point| self.file_editor_pane()?.diagnostic_range_at(point));
            let diagnostic_hovered = hovered.is_some();
            if self.file_editor_input.update_hovered_diagnostic(hovered) {
                self.rebuild_presentation();
                self.request_redraw();
            }
            let position = if diagnostic_hovered {
                None
            } else {
                self.cursor_position
                    .and_then(|point| self.file_editor_pane()?.text_position_at(point))
            };
            if self
                .file_editor_input
                .update_hovered_language_position(position)
            {
                if let Some(position) = position {
                    self.language_service.request_active_at(
                        &self.file_editor_host,
                        LanguageRequestKind::Hover,
                        position,
                    );
                } else {
                    self.language_service.dismiss_hover();
                }
                self.rebuild_presentation();
                self.request_redraw();
            }
            return false;
        }
        let Some(point) = self.cursor_position else {
            return true;
        };
        let Some(editor_bounds) = self.file_editor_pane().map(|pane| pane.editor_bounds()) else {
            return true;
        };
        self.file_editor_input
            .auto_scroll
            .update(point, editor_bounds, Instant::now());
        if point.y < editor_bounds.origin.y || point.y >= editor_bounds.bottom() {
            if self.advance_file_editor_auto_scroll(Instant::now()) {
                self.file_editor_changed();
            }
            return true;
        }
        let point = zeta_ui::Point::new(
            point.x.clamp(
                editor_bounds.origin.x,
                (editor_bounds.right() - 1.0).max(editor_bounds.origin.x),
            ),
            point.y,
        );
        let Some(position) = self
            .file_editor_pane()
            .and_then(|pane| pane.text_position_at(point))
        else {
            return true;
        };
        self.file_editor_host
            .move_active_caret(position, CodeEditorSelectionMode::Extend);
        self.reveal_file_editor_caret();
        self.file_editor_changed();
        true
    }

    pub(super) fn advance_file_editor_auto_scroll(&mut self, now: Instant) -> bool {
        if !self.file_editor_input.is_selecting() {
            self.file_editor_input.auto_scroll.stop();
            return false;
        }
        let direction = self.file_editor_input.auto_scroll.advance(now);
        if direction == FileEditorAutoScrollDirection::Idle {
            return false;
        }
        let Some((row_count, capacity)) = self.file_editor_scroll_metrics() else {
            self.file_editor_input.auto_scroll.stop();
            return false;
        };
        if !self
            .file_editor_host
            .scroll_active_rows(direction.row_delta(), row_count, capacity)
        {
            self.file_editor_input.auto_scroll.stop();
            return false;
        }
        let Some(bounds) = self.file_editor_pane().map(|pane| pane.editor_bounds()) else {
            self.file_editor_input.auto_scroll.stop();
            return false;
        };
        let cursor = self.cursor_position.unwrap_or(bounds.origin);
        let x = cursor
            .x
            .clamp(bounds.origin.x, (bounds.right() - 1.0).max(bounds.origin.x));
        let y = match direction {
            FileEditorAutoScrollDirection::Up => bounds.origin.y,
            FileEditorAutoScrollDirection::Down => (bounds.bottom() - 1.0).max(bounds.origin.y),
            FileEditorAutoScrollDirection::Idle => return false,
        };
        let Some(position) = self
            .file_editor_pane()
            .and_then(|pane| pane.text_position_at(zeta_ui::Point::new(x, y)))
        else {
            return false;
        };
        self.file_editor_host
            .move_active_caret(position, CodeEditorSelectionMode::Extend);
        self.caret_blink.activity(now);
        true
    }

    pub(super) fn route_file_editor_pointer_button(&mut self, state: ElementState) -> bool {
        if !self.workspace_surface.is_editor() {
            return false;
        }
        if state == ElementState::Released {
            let handled = self.file_editor_input.is_selecting();
            self.file_editor_input.end_selection();
            return handled;
        }
        let Some(point) = self.cursor_position else {
            return false;
        };
        let document_hit = self
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.interaction_frame.target_at(point))
            == Some(FILE_EDITOR_DOCUMENT);
        if !document_hit {
            return false;
        }
        let Some(position) = self
            .file_editor_pane()
            .and_then(|pane| pane.text_position_at(point))
        else {
            return true;
        };
        let mode = if self.modifiers.shift_key() {
            CodeEditorSelectionMode::Extend
        } else {
            CodeEditorSelectionMode::Move
        };
        self.file_editor_host.move_active_caret(position, mode);
        self.reveal_file_editor_caret();
        self.file_editor_input.begin_selection();
        self.file_editor_changed();
        true
    }

    pub(super) fn route_file_editor_wheel(&mut self, delta: MouseScrollDelta) -> bool {
        if !self.workspace_surface.is_editor() {
            return false;
        }
        let Some(point) = self.cursor_position else {
            return false;
        };
        let in_editor = self
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.interaction_frame.target_at(point))
            .is_some_and(|target| {
                self.presentation.as_ref().is_some_and(|presentation| {
                    presentation
                        .interaction_frame
                        .ancestry(target)
                        .contains(&FILE_EDITOR_PANE)
                })
            });
        if !in_editor {
            return false;
        }
        let rows = self.file_editor_input.wheel_rows(delta);
        let Some((row_count, capacity)) = self.file_editor_scroll_metrics() else {
            return true;
        };
        if rows != 0
            && self
                .file_editor_host
                .scroll_active_rows(rows, row_count, capacity)
        {
            self.rebuild_presentation_on_next_redraw();
        }
        true
    }

    pub(super) fn activate_file_editor_element(&mut self, id: zui::ElementId) -> bool {
        if let Some(index) = file_editor_close_index(id, 0..self.file_editor_host.tabs().len()) {
            self.file_editor_host.select(index);
            match self.file_editor_host.request_close_active() {
                FileEditorCloseRequest::CanClose => self.close_active_file_editor_tab(),
                FileEditorCloseRequest::NeedsConfirmation => {
                    self.file_editor_input.confirm_close();
                    self.pending_focus = Some(FileEditorAction::SaveAndClose.element_id());
                    self.rebuild_presentation_on_next_redraw();
                    self.request_redraw();
                }
            }
            return true;
        }
        if let Some(action) = FileEditorAction::from_element_id(id) {
            self.activate_file_editor_action(action);
            return true;
        }
        if let Some(index) = file_editor_tab_index(id, 0..self.file_editor_host.tabs().len()) {
            self.file_editor_host.select(index);
            self.file_editor_input.dismiss_prompt();
            self.workspace_surface.show_editor();
            self.file_editor_input.fractional_rows = 0.0;
            self.pending_focus = Some(FILE_EDITOR_DOCUMENT);
            self.rebuild_presentation_on_next_redraw();
            self.request_redraw();
            return true;
        }
        let Some(pane) = self.file_editor_pane() else {
            return false;
        };
        let Some(index) = file_editor_fold_index(id, 0..pane.fold_control_count()) else {
            return false;
        };
        let Some(control) = pane.fold_control(index) else {
            return true;
        };
        self.file_editor_host.toggle_active_fold(control);
        self.reveal_file_editor_caret();
        self.pending_focus = Some(FILE_EDITOR_DOCUMENT);
        self.caret_blink.activity(Instant::now());
        self.rebuild_presentation_on_next_redraw();
        self.request_redraw();
        true
    }

    fn activate_file_editor_action(&mut self, action: FileEditorAction) {
        match action {
            FileEditorAction::Reload => {
                self.file_editor_host.reload_active_external();
                self.file_editor_input.fractional_rows = 0.0;
                self.file_editor_input.dismiss_prompt();
                self.pending_focus = Some(FILE_EDITOR_DOCUMENT);
            }
            FileEditorAction::Overwrite => {
                if self.overwrite_active_workspace_file() {
                    self.pending_focus = Some(FILE_EDITOR_DOCUMENT);
                }
            }
            FileEditorAction::SaveAndClose => {
                self.file_editor_input.dismiss_prompt();
                if self.try_save_active_workspace_file() {
                    self.close_active_file_editor_tab();
                    return;
                }
            }
            FileEditorAction::DiscardAndClose => {
                self.file_editor_input.dismiss_prompt();
                self.close_active_file_editor_tab();
                return;
            }
            FileEditorAction::CancelClose => {
                self.file_editor_input.dismiss_prompt();
                self.pending_focus = Some(FILE_EDITOR_DOCUMENT);
            }
            FileEditorAction::FindPrevious => {
                self.find_file_editor_match(true);
                return;
            }
            FileEditorAction::FindNext => {
                self.find_file_editor_match(false);
                return;
            }
            FileEditorAction::ReplaceCurrent => {
                self.replace_current_file_editor_match();
                return;
            }
            FileEditorAction::ReplaceAll => {
                let query = self.file_editor_search.query();
                if !query.text().is_empty() {
                    let replacement = self.file_editor_search.replacement().to_owned();
                    self.file_editor_host.replace_all(&query, &replacement);
                }
                self.file_editor_changed();
                return;
            }
            FileEditorAction::CloseSearch => {
                self.file_editor_search.hide();
                self.pending_focus = Some(FILE_EDITOR_DOCUMENT);
            }
        }
        self.rebuild_presentation();
        self.request_redraw();
    }

    fn close_active_file_editor_tab(&mut self) {
        let closing_path = self
            .file_editor_host
            .active()
            .map(|tab| tab.path().to_path_buf());
        self.file_editor_host.close_active_discarding_changes();
        if let Some(path) = closing_path {
            self.language_service.close(&path);
        }
        self.file_editor_input.dismiss_prompt();
        self.file_editor_input.fractional_rows = 0.0;
        if self.file_editor_host.active().is_some() {
            self.pending_focus = Some(FILE_EDITOR_DOCUMENT);
        } else {
            self.workspace_surface.show_agent();
            self.pending_focus = Some(crate::shell_interaction::COMPOSER);
        }
        self.rebuild_presentation_on_next_redraw();
        self.request_redraw();
    }

    pub(super) fn copy_file_editor_selection(&mut self) -> bool {
        if !self.ui_dispatch.is_focused(FILE_EDITOR_DOCUMENT) {
            return false;
        }
        if let Some(text) = self
            .file_editor_host
            .active()
            .and_then(|tab| tab.document().selected_text())
            && let Err(error) = write_clipboard_text(text.to_owned())
        {
            eprintln!("could not copy file editor text: {error}");
        }
        true
    }

    pub(super) fn paste_into_file_editor(&mut self) -> bool {
        if !self.ui_dispatch.is_focused(FILE_EDITOR_DOCUMENT) {
            return false;
        }
        let text = match read_clipboard_text() {
            Ok(text) => text,
            Err(error) => {
                eprintln!("could not paste file editor text: {error}");
                return true;
            }
        };
        self.file_editor_host.apply(CodeEditorCommand::Insert(text));
        self.reveal_file_editor_caret();
        self.file_editor_changed();
        true
    }

    pub(super) fn file_editor_changed(&mut self) {
        self.language_service
            .synchronize_active(&self.file_editor_host);
        self.caret_blink.activity(Instant::now());
        self.rebuild_presentation();
        self.update_ime_cursor_area();
        self.request_redraw();
    }

    fn reveal_file_editor_caret(&mut self) {
        let Some(pane) = self.file_editor_pane() else {
            return;
        };
        let Some(caret_row) = pane.caret_visual_row() else {
            return;
        };
        let row_count = pane.visual_row_count();
        let capacity = pane.visible_row_capacity();
        self.file_editor_host
            .reveal_active_visual_row(caret_row, row_count, capacity);
    }

    fn file_editor_scroll_metrics(&self) -> Option<(usize, usize)> {
        let pane = self.file_editor_pane()?;
        Some((pane.visual_row_count(), pane.visible_row_capacity()))
    }

    fn file_editor_pane(&self) -> Option<FileEditorPane<'_>> {
        let bounds = self
            .presentation
            .as_ref()?
            .accessibility_nodes
            .iter()
            .find(|node| node.id == FILE_EDITOR_PANE)?
            .bounds;
        Some(
            FileEditorPane::new(
                bounds,
                &self.file_editor_host,
                self.code_editor_style.clone(),
                self.palette,
                self.caret_blink.visibility(),
            )
            .with_prompt(self.file_editor_input.prompt())
            .with_diagnostics(
                self.language_service
                    .active_editor_diagnostics(&self.file_editor_host),
            )
            .with_pointer_position(self.cursor_position)
            .with_search_mode(self.file_editor_search.mode()),
        )
    }
}

#[cfg(test)]
#[path = "file_editor_input_tests.rs"]
mod tests;
