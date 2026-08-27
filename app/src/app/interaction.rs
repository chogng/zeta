use super::*;

impl NativeApp {
    pub(super) fn logical_pointer_position(&self, physical_x: f64, physical_y: f64) -> Point {
        let scale_factor = if self.scale_factor.is_finite() && self.scale_factor > 0.0 {
            self.scale_factor as f32
        } else {
            1.0
        };
        Point::new(
            physical_x as f32 / scale_factor,
            physical_y as f32 / scale_factor,
        )
    }

    pub(super) fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            let _ = window.request_redraw();
        }
    }

    pub(super) fn update_cursor(&self) {
        let feedback = self
            .presentation
            .as_ref()
            .map(|presentation| {
                self.ui_dispatch
                    .pointer_feedback(presentation.interaction_frame())
            })
            .unwrap_or_default();
        let cursor = if let Some(resize) = self.terminal_pane_resize.as_ref() {
            match resize.resizable.orientation() {
                SashOrientation::Vertical => CursorIcon::ColResize,
                SashOrientation::Horizontal => CursorIcon::RowResize,
            }
        } else if self.tab_container.is_resizing() || self.inspector_resizable.is_dragging() {
            CursorIcon::ColResize
        } else {
            match feedback {
                CursorFeedback::Default => CursorIcon::Default,
                CursorFeedback::Text => CursorIcon::Text,
                CursorFeedback::Pointer => CursorIcon::Pointer,
                CursorFeedback::ResizeHorizontal => CursorIcon::ColResize,
                CursorFeedback::ResizeVertical => CursorIcon::RowResize,
            }
        };
        if let Some(window) = self.window.as_ref() {
            let _ = window.set_cursor(cursor);
        }
    }

    pub(super) fn sash_pointer_presence(&self, id: ElementId) -> SashPointerPresence {
        let Some(point) = self.cursor_position else {
            return SashPointerPresence::Outside;
        };
        let over = self.ui_dispatch.window_active()
            && self.presentation.as_ref().is_some_and(|presentation| {
                presentation.interaction_frame().target_at(point) == Some(id)
            });
        if over {
            SashPointerPresence::Over
        } else {
            SashPointerPresence::Outside
        }
    }

    pub(super) fn sync_sash_pointer_presence(&mut self, now: Instant) -> bool {
        let window_active = self.ui_dispatch.window_active();
        let session_hovered = window_active
            && self
                .ui_dispatch
                .is_hovered(shell_interaction::TAB_CONTAINER_RESIZE_HANDLE);
        let agent_hovered = window_active
            && self
                .ui_dispatch
                .is_hovered(shell_interaction::INSPECTOR_RESIZE_HANDLE);
        let session_presence = if session_hovered {
            SashPointerPresence::Over
        } else {
            SashPointerPresence::Outside
        };
        let agent_presence = if agent_hovered {
            SashPointerPresence::Over
        } else {
            SashPointerPresence::Outside
        };
        let session_changed = self
            .tab_container
            .sash_pointer_presence(session_presence, now);
        let agent_changed = self
            .inspector_resizable
            .pointer_presence(agent_presence, now);
        session_changed || agent_changed
    }

    pub(super) fn apply_dispatch_outcome(&mut self, outcome: DispatchOutcome) {
        let sash_changed = self.sync_sash_pointer_presence(Instant::now());
        let activation = matches!(outcome.intent, Some(UiIntent::Activate(_)));
        if let Some(intent) = outcome.intent {
            session_switch_trace::event(None, "ui-intent", format_args!("intent={intent:?}"));
            match intent {
                UiIntent::StartWindowDrag(_) => {
                    if let Some(window) = self.window.as_ref()
                        && let Err(error) = window.start_window_drag()
                    {
                        eprintln!("could not begin native window drag: {error}");
                    }
                }
                UiIntent::Activate(id) => self.activate_shell_element(id),
            }
        }
        match outcome.invalidation {
            DispatchInvalidation::None => {}
            DispatchInvalidation::Paint => {
                self.sync_input_focus();
                self.rebuild_presentation_on_next_redraw();
            }
            DispatchInvalidation::Fragment => {
                self.sync_input_focus();
                if activation {
                    self.rebuild_presentation_on_next_redraw();
                } else if let Some(id) = outcome.fragment {
                    self.rebuild_fragment_on_next_redraw(id);
                } else {
                    self.rebuild_overlay_on_next_redraw();
                }
            }
        }
        if sash_changed {
            self.rebuild_presentation_on_next_redraw();
        }
    }

    pub(super) fn activate_shell_element(&mut self, id: zui::ui::ElementId) {
        if self.activate_language_server_settings_element(id) {
            return;
        }
        if self.activate_file_editor_element(id) {
            return;
        }
        let interaction_item_count = self
            .composer
            .interaction()
            .view()
            .map(|view| view.items().len())
            .unwrap_or(0);
        if let Some(index) =
            shell_interaction::composer_interaction_item_index(id, 0..interaction_item_count)
        {
            self.activate_composer_interaction_item(index);
            return;
        }
        if let Some(action) = self.workspace_pane_host.activate_file_tree_element(id) {
            match action {
                WorkspacePaneAction::OpenFile { path } => self.open_workspace_file(path),
                WorkspacePaneAction::LoadChildren { element, path } => {
                    self.load_file_tree_directory(element, path);
                }
                WorkspacePaneAction::Handled
                | WorkspacePaneAction::StateChanged
                | WorkspacePaneAction::Focus(_) => {}
            }
            return;
        }
        if self.workspace_pane_host.toggle_multi_diff_fold(id) {
            return;
        }
        if self.activate_keyboard_shortcuts_element(id) {
            return;
        }
        if self.activate_remote_connection_manager_element(id) {
            return;
        }
        if self.activate_remote_tunnel_manager_element(id) {
            return;
        }
        if self.activate_remote_connection_picker_element(id) {
            return;
        }
        if self.activate_git_branch_context_menu_element(id) {
            return;
        }
        if self.activate_workspace_path_picker_element(id) {
            return;
        }
        if matches!(
            id,
            shell_interaction::TAB_CONTAINER_SETTINGS_TAB
                | shell_interaction::TITLEBAR_SETTINGS_TAB
        ) {
            self.activate_settings_tab();
            return;
        }
        if let Some(index) = shell_interaction::session_tab_index(
            id,
            0..self.workbench_host.workbench().tab_part().session_count(),
        ) {
            session_switch_trace::event(
                None,
                "session-tab-hit",
                format_args!(
                    "element={id:?} index={index} tab_count={}",
                    self.workbench_host.workbench().tab_part().session_count()
                ),
            );
            self.activate_session_tab(index);
            return;
        }
        if let Some(request) = command_dispatch::command_request_for_element(id) {
            self.dispatch_command(request);
        }
    }

    pub(super) fn pointer_moved(&mut self, physical_x: f64, physical_y: f64) {
        let point = self.logical_pointer_position(physical_x, physical_y);
        self.cursor_position = Some(point);
        if self.route_remote_connection_manager_pointer_move(point) {
            return;
        }
        if self.route_remote_tunnel_manager_pointer_move(point) {
            return;
        }
        if self.route_remote_connection_picker_pointer_move(point) {
            return;
        }
        if self.route_git_branch_context_menu_pointer_move(point) {
            return;
        }
        if self.route_workspace_path_picker_pointer_move(point) {
            return;
        }
        if self.route_session_context_menu_pointer_move(point) {
            return;
        }
        if self.route_tab_container_resize_move(point) {
            return;
        }
        if self.route_inspector_resize_move(point) {
            return;
        }
        if self.route_terminal_pane_resize_move(point) {
            return;
        }
        if self.route_file_editor_pointer_move() {
            return;
        }
        if self.route_multi_diff_scrollbar_move(point) {
            return;
        }
        let terminal_position = self.terminal_mouse_position(point);
        let terminal_captured = self.route_terminal_pointer_move(terminal_position);
        if !terminal_captured && self.route_terminal_selection_move(terminal_position) {
            return;
        }
        let outcome = self
            .presentation
            .as_ref()
            .map(|presentation| {
                self.ui_dispatch
                    .pointer_moved(point, presentation.interaction_frame())
            })
            .unwrap_or_default();
        self.update_cursor();
        self.apply_dispatch_outcome(outcome);
    }

    pub(super) fn pointer_left(&mut self) {
        self.cursor_position = None;
        self.file_editor_input.cancel_pointer();
        let pane_resize_cancelled = self.cancel_terminal_pane_resize();
        if self
            .workspace_pane_host
            .leave_multi_diff_scrollbar(Instant::now())
        {
            self.rebuild_presentation();
            self.request_redraw();
        }
        let outcome = self.ui_dispatch.pointer_left();
        if pane_resize_cancelled {
            self.rebuild_presentation();
            self.request_redraw();
        }
        self.update_cursor();
        self.apply_dispatch_outcome(outcome);
    }

    pub(super) fn primary_button_changed(&mut self, state: ElementState) {
        let composer_click = (state == ElementState::Pressed)
            .then(|| {
                let presentation = self.presentation.as_ref()?;
                let point = self.cursor_position?;
                (presentation.interaction_frame().target_at(point) == Some(COMPOSER))
                    .then_some((point, presentation))
            })
            .flatten()
            .and_then(|(point, presentation)| {
                presentation
                    .element_bounds(COMPOSER)
                    .map(|bounds| (point, bounds))
            });
        let Some(presentation) = self.presentation.as_ref() else {
            return;
        };
        let outcome = match state {
            ElementState::Pressed => self
                .ui_dispatch
                .press_primary(presentation.interaction_frame()),
            ElementState::Released => {
                let point = self.cursor_position.unwrap_or(Point::new(-1.0, -1.0));
                self.ui_dispatch
                    .release_primary(point, presentation.interaction_frame())
            }
        };
        self.apply_dispatch_outcome(outcome);
        if let Some((point, bounds)) = composer_click {
            let selection_mode = if self.modifiers.shift_key() {
                zeta_editor::CodeEditorSelectionMode::Extend
            } else {
                zeta_editor::CodeEditorSelectionMode::Move
            };
            if self
                .composer
                .move_caret_to_point(bounds, point, selection_mode)
            {
                self.composer_changed();
            }
        }
    }

    pub(super) fn mouse_button_changed(&mut self, state: ElementState, button: MouseButton) {
        if self.route_remote_connection_manager_button(state, button) {
            return;
        }
        if self.route_remote_tunnel_manager_button(state, button) {
            return;
        }
        if self.route_remote_connection_picker_button(state, button) {
            return;
        }
        if self.route_git_branch_context_menu_button(state, button) {
            return;
        }
        if self.route_workspace_path_picker_button(state, button) {
            return;
        }
        if self.route_session_context_menu_button(state, button) {
            return;
        }
        if button == MouseButton::Left && self.route_tab_container_resize_button(state) {
            return;
        }
        if button == MouseButton::Left && self.route_inspector_resize_button(state) {
            return;
        }
        if button == MouseButton::Left && self.route_terminal_pane_resize_button(state) {
            return;
        }
        if button == MouseButton::Left && self.route_multi_diff_scrollbar_button(state) {
            return;
        }
        if button == MouseButton::Left
            && state == ElementState::Pressed
            && let Some(point) = self.cursor_position
        {
            let _ = self.activate_terminal_pane_at(point);
        }
        let position = self
            .cursor_position
            .and_then(|point| self.terminal_mouse_position(point));
        if self.route_terminal_pointer_button(position, button, state) {
            return;
        }
        if button == MouseButton::Left && self.route_terminal_selection_button(position, state) {
            return;
        }
        if button == MouseButton::Left {
            self.primary_button_changed(state);
            self.route_file_editor_pointer_button(state);
        }
    }

    pub(super) fn multi_diff_bounds(&self) -> Option<zeta_ui::Rect> {
        self.presentation
            .as_ref()?
            .element_bounds(shell_interaction::MULTI_DIFF_EDITOR)
    }

    pub(super) fn route_multi_diff_scrollbar_move(&mut self, point: Point) -> bool {
        let Some(bounds) = self.multi_diff_bounds() else {
            return false;
        };
        let outcome =
            self.workspace_pane_host
                .move_multi_diff_scrollbar(point, bounds, Instant::now());
        if outcome.presentation_changed {
            self.rebuild_presentation_on_next_redraw();
        }
        outcome.handled
    }

    pub(super) fn route_multi_diff_scrollbar_button(&mut self, state: ElementState) -> bool {
        let Some(bounds) = self.multi_diff_bounds() else {
            return false;
        };
        let point = self.cursor_position.unwrap_or(Point::new(-1.0, -1.0));
        let now = Instant::now();
        let outcome = match state {
            ElementState::Pressed => self
                .workspace_pane_host
                .press_multi_diff_scrollbar(point, bounds, now),
            ElementState::Released => self
                .workspace_pane_host
                .release_multi_diff_scrollbar(point, bounds, now),
        };
        if outcome.presentation_changed {
            self.rebuild_presentation_on_next_redraw();
        }
        outcome.handled
    }
}
