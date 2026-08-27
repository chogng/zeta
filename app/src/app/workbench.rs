use super::*;

impl NativeApp {
    /// Closes one logical Session tab and releases every product resource keyed by that tab.
    pub(super) fn close_session_tab(&mut self, tab_key: &TabInputKey) -> bool {
        if !tab_key.is_session() {
            return false;
        }
        if let Some(session) = self.agent_session.as_ref()
            && let Some(session_id) = tab_key.session_id()
            && let Err(error) = session.stop_session(session_id.clone())
        {
            eprintln!("could not close Session {session_id}: {error}");
            return false;
        }
        let was_active = self.workbench.workbench().tab_part().active_tab_key() == Some(tab_key);
        let Some((closed, bindings)) = self.workbench.close_tab(tab_key) else {
            return false;
        };

        for binding in bindings {
            if let Some(terminal_key) = binding.terminal_key() {
                let _ = self.terminal_workspace.remove_key(terminal_key);
            }
        }
        self.pane_view_states.retain(|(key, _), _| key != tab_key);
        if self
            .active_pane
            .as_ref()
            .is_some_and(|(key, _)| key == tab_key)
        {
            self.active_pane = None;
            self.terminal_selection.clear();
            self.terminal_pointer.cancel();
        }
        if self
            .terminal_pane_resize
            .as_ref()
            .is_some_and(|resize| resize.tab_key == *tab_key)
        {
            self.terminal_pane_resize = None;
        }

        if was_active {
            match closed.active_tab().cloned() {
                Some(tab_key @ TabInputKey::Session(_)) => {
                    self.activate_session_tab_after_close(&tab_key);
                }
                Some(TabInputKey::Settings) => self.activate_settings_tab(),
                None => self.workspace_surface.show_agent(),
            }
        }
        self.rebuild_presentation_on_next_redraw();
        true
    }

    pub(super) fn ensure_terminal_for_session(&mut self, session_id: &SessionId) -> bool {
        match self
            .terminal_workspace
            .ensure_for_session(session_id, self.terminal_size())
        {
            Ok(()) => {
                let tab_key = TabInputKey::session(session_id.clone());
                let root_pane = self
                    .workbench
                    .workbench_mut()
                    .ensure_root_pane(tab_key.clone(), PaneInput::terminal(session_id.clone()));
                if let Some(terminal_key) = self.terminal_workspace.key_for_session(session_id) {
                    let Some(input) = self
                        .workbench
                        .workbench()
                        .pane_part(&tab_key)
                        .and_then(|pane_part| pane_part.pane_input(root_pane))
                        .cloned()
                    else {
                        return false;
                    };
                    let binding = self
                        .workbench
                        .pane_host_mut()
                        .ensure((PaneHostScope::Tab(tab_key), root_pane), PaneBinding::new());
                    if !binding.bind_terminal(&input, session_id, terminal_key) {
                        return false;
                    }
                }
                true
            }
            Err(error) => {
                eprintln!("could not start terminal for session: {error}");
                false
            }
        }
    }

    pub(super) fn activate_terminal_for_session(&mut self, session_id: &SessionId) -> bool {
        let tab_key = TabInputKey::session(session_id.clone());
        let Some(pane) = self
            .workbench
            .workbench()
            .pane_part(&tab_key)
            .map(|pane_part| pane_part.active_pane())
        else {
            return false;
        };
        let host_key = (PaneHostScope::Tab(tab_key.clone()), pane);
        let current = self
            .workbench
            .workbench()
            .pane_part(&tab_key)
            .and_then(|pane_part| pane_part.pane_input(pane))
            .cloned();
        if current
            .as_ref()
            .is_some_and(|input| matches!(input.kind(), PaneInputKind::Files | PaneInputKind::Diff))
        {
            if let Some(current) = current {
                let _ = self
                    .workbench
                    .workbench_mut()
                    .remember_workspace_return(&tab_key, current);
            }
        }
        let Some(terminal_key) = self
            .workbench
            .pane_host()
            .binding(&host_key)
            .and_then(PaneBinding::terminal_key)
            .or_else(|| self.terminal_workspace.key_for_session(session_id))
        else {
            return false;
        };
        self.workbench.workbench_mut().mount_input(
            &tab_key,
            pane,
            PaneInput::terminal(session_id.clone()),
        );
        self.workbench
            .pane_host_mut()
            .insert(host_key.clone(), PaneBinding::new());
        let Some(input) = self
            .workbench
            .workbench()
            .pane_part(&tab_key)
            .and_then(|pane_part| pane_part.pane_input(pane))
            .cloned()
        else {
            return false;
        };
        let binding = self
            .workbench
            .pane_host_mut()
            .ensure(host_key, PaneBinding::new());
        if !binding.bind_terminal(&input, session_id, terminal_key) {
            return false;
        }
        if !self.activate_pane_context(tab_key, pane) {
            return false;
        }
        if let Some(window) = self.window.as_ref()
            && let Some(terminal) = self.active_terminal()
        {
            let _ = window.set_title(terminal.core().title().unwrap_or(PRODUCT_DISPLAY_NAME));
        }
        true
    }

    pub(super) fn save_active_pane_view(&mut self) {
        let Some(binding) = self.active_pane.clone() else {
            return;
        };
        let state = TerminalPaneViewState {
            scroll: std::mem::take(&mut self.terminal_scroll),
            pointer: std::mem::take(&mut self.terminal_pointer),
            selection: std::mem::take(&mut self.terminal_selection),
        };
        self.pane_view_states.insert(binding, state);
    }

    pub(super) fn restore_pane_view(&mut self, binding: &(TabInputKey, PaneId)) {
        let state = self.pane_view_states.remove(binding).unwrap_or_default();
        self.terminal_scroll = state.scroll;
        self.terminal_pointer = state.pointer;
        self.terminal_selection = state.selection;
    }

    pub(super) fn activate_pane_context(&mut self, tab_key: TabInputKey, pane: PaneId) -> bool {
        let binding = (tab_key.clone(), pane);
        if self.active_pane.as_ref() != Some(&binding) {
            self.save_active_pane_view();
            self.active_pane = Some(binding.clone());
            self.restore_pane_view(&binding);
        }
        if !self.workbench.workbench_mut().activate_pane(&tab_key, pane) {
            return false;
        }
        let host_binding = (PaneHostScope::Tab(tab_key.clone()), pane);
        let Some(pane_binding) = self.workbench.pane_host().binding(&host_binding) else {
            return false;
        };
        let terminal_key = pane_binding.terminal_key();
        let Some(terminal_key) = terminal_key else {
            return true;
        };
        self.terminal_workspace.activate_key(terminal_key)
            || self.terminal_workspace.active_key() == Some(terminal_key)
    }

    pub(super) fn active_pane_terminal_key(&self) -> Option<TerminalSessionKey> {
        match self.active_pane.as_ref() {
            Some((tab_key, pane)) => self
                .workbench
                .pane_host()
                .binding(&(PaneHostScope::Tab(tab_key.clone()), *pane))
                .and_then(PaneBinding::terminal_key),
            None => self.terminal_workspace.active_key(),
        }
    }

    pub(super) fn update_terminal_status(&mut self, key: TerminalSessionKey, status: &str) {
        let Some(session_id) = self.terminal_workspace.session_id_for_key(key) else {
            return;
        };
        self.workbench
            .workbench_mut()
            .tab_part_mut()
            .update_status(&session_id, status);
    }

    pub(super) fn active_terminal(&self) -> Option<&TerminalSession> {
        self.active_pane_terminal_key()
            .and_then(|key| self.terminal_workspace.terminal(key))
    }

    pub(super) fn active_terminal_mut(&mut self) -> Option<&mut TerminalSession> {
        let key = self.active_pane_terminal_key()?;
        self.terminal_workspace.terminal_mut(key)
    }

    pub(super) fn active_session_tab_key(&self) -> Option<TabInputKey> {
        self.workbench
            .workbench()
            .tab_part()
            .active_tab_key()
            .filter(|key| key.is_session())
            .cloned()
    }

    pub(super) fn split_active_pane(&mut self, direction: PaneSplitDirection) {
        if !self.workspace_surface.is_terminal() {
            return;
        }
        let Some(tab_key) = self.active_session_tab_key() else {
            return;
        };
        let Some(session_id) = tab_key.session_id().cloned() else {
            return;
        };
        if !self.ensure_terminal_for_session(&session_id) {
            return;
        }
        let terminal_key = match self.terminal_workspace.spawn_pane(self.terminal_size()) {
            Ok(key) => key,
            Err(error) => {
                eprintln!("could not create split terminal Pane: {error}");
                return;
            }
        };
        self.terminal_workspace
            .bind_key_to_session(terminal_key, session_id.clone());
        let Some(pane) = self
            .workbench
            .workbench_mut()
            .create_pane_with_direction(PaneInput::terminal(session_id.clone()), direction)
        else {
            return;
        };
        self.workbench.pane_host_mut().insert(
            (PaneHostScope::Tab(tab_key.clone()), pane),
            PaneBinding::terminal(terminal_key),
        );
        let _ = self.activate_pane_context(tab_key, pane);
        self.rebuild_presentation_on_next_redraw();
    }

    pub(super) fn close_active_pane(&mut self) {
        if !self.workspace_surface.is_terminal() {
            return;
        }
        let Some(tab_key) = self.active_session_tab_key() else {
            return;
        };
        let Some(removed) = self.workbench.workbench_mut().destroy_pane() else {
            return;
        };
        let Some(removed) = removed.first() else {
            return;
        };
        let removed_pane = removed.id();
        let removed_binding = (tab_key.clone(), removed_pane);
        let removed_host_binding = (PaneHostScope::Tab(tab_key.clone()), removed_pane);
        let removed_binding_state = self.workbench.pane_host_mut().remove(&removed_host_binding);
        let removed_key = removed_binding_state
            .as_ref()
            .and_then(|binding| binding.terminal_key());
        if let Some(removed_key) = removed_key {
            let _ = self.terminal_workspace.remove_key(removed_key);
        }
        self.pane_view_states.remove(&removed_binding);
        if self.active_pane.as_ref() == Some(&removed_binding) {
            self.active_pane = None;
        }
        let Some(replacement_pane) = self
            .workbench
            .workbench()
            .pane_part(&tab_key)
            .map(|pane_part| pane_part.active_pane())
        else {
            return;
        };
        let _ = self.activate_pane_context(tab_key, replacement_pane);
        self.rebuild_presentation_on_next_redraw();
    }

    pub(super) fn focus_next_pane(&mut self) {
        self.focus_adjacent_pane(true);
    }

    pub(super) fn focus_previous_pane(&mut self) {
        self.focus_adjacent_pane(false);
    }

    pub(super) fn focus_adjacent_pane(&mut self, next: bool) {
        if !self.workspace_surface.is_terminal() {
            return;
        }
        let Some(tab_key) = self.active_session_tab_key() else {
            return;
        };
        let Some(pane) = (if next {
            self.workbench.workbench_mut().focus_next_pane(&tab_key)
        } else {
            self.workbench.workbench_mut().focus_previous_pane(&tab_key)
        }) else {
            return;
        };
        let _ = self.activate_pane_context(tab_key, pane);
        self.rebuild_presentation_on_next_redraw();
    }

    pub(super) fn terminal_pane_sash_at(
        &self,
        point: Point,
    ) -> Option<(
        TabInputKey,
        PaneSplitId,
        SplitViewOrientation,
        SplitViewResizeSnapshot,
    )> {
        if !self.workspace_surface.is_terminal() {
            return None;
        }
        let tab_key = self.active_session_tab_key()?;
        let layout = self.workbench.workbench().pane_part(&tab_key)?;
        terminal_pane_sash_for_viewport(
            self.logical_viewport(),
            self.active_screen(),
            self.tab_container,
            self.inspector_part,
            layout,
            point,
        )
        .map(|(split_id, orientation, snapshot)| (tab_key, split_id, orientation, snapshot))
    }

    pub(super) fn route_terminal_pane_resize_move(&mut self, point: Point) -> bool {
        let Some(resize) = self.terminal_pane_resize.as_mut() else {
            return false;
        };
        let Some(next) = resize.resizable.resize_to(point) else {
            self.update_cursor();
            return true;
        };
        let total = next.previous_size() + next.next_size();
        let Some(ratio) = (total.is_finite() && total > 0.0)
            .then(|| (next.previous_size() / total).clamp(0.0, 1.0))
        else {
            self.update_cursor();
            return true;
        };
        let changed =
            self.workbench
                .workbench_mut()
                .resize_split(&resize.tab_key, resize.split_id, ratio);
        if changed {
            self.terminal_selection.clear();
            self.rebuild_presentation();
            self.request_redraw();
        }
        self.update_cursor();
        true
    }

    pub(super) fn route_terminal_pane_resize_button(&mut self, state: ElementState) -> bool {
        let now = Instant::now();
        match state {
            ElementState::Pressed => {
                if self.terminal_pane_resize.is_some() {
                    return true;
                }
                let Some(point) = self.cursor_position else {
                    return false;
                };
                let Some((tab_key, split_id, orientation, snapshot)) =
                    self.terminal_pane_sash_at(point)
                else {
                    return false;
                };
                let identity = shell_interaction::terminal_pane_sash_id(split_id);
                let over_sash = self.presentation.as_ref().is_some_and(|presentation| {
                    presentation.interaction_frame().target_at(point) == Some(identity)
                });
                if !over_sash {
                    return false;
                }
                let orientation = match orientation {
                    SplitViewOrientation::Horizontal => SashOrientation::Vertical,
                    SplitViewOrientation::Vertical => SashOrientation::Horizontal,
                };
                let mut resizable = Resizable::new(orientation);
                if !resizable.begin_drag(snapshot, point, now) {
                    return false;
                }
                self.terminal_pane_resize = Some(TerminalPaneResize {
                    tab_key,
                    split_id,
                    resizable,
                });
            }
            ElementState::Released => {
                let Some(mut resize) = self.terminal_pane_resize.take() else {
                    return false;
                };
                let identity = shell_interaction::terminal_pane_sash_id(resize.split_id);
                let presence = self.sash_pointer_presence(identity);
                let _ = resize.resizable.end_drag(presence, now);
            }
        }
        self.rebuild_presentation();
        self.update_cursor();
        self.request_redraw();
        true
    }

    pub(super) fn cancel_terminal_pane_resize(&mut self) -> bool {
        let Some(mut resize) = self.terminal_pane_resize.take() else {
            return false;
        };
        resize.resizable.cancel()
    }
}

impl NativeApp {
    /// Selects the singleton Settings workbench item and prepares its feature-owned state.
    pub(super) fn activate_settings_tab(&mut self) {
        if !self.workbench.workbench().tab_part().is_settings() {
            self.settings_section = zeta_settings::SettingsPageSection::LanguageServers;
        }
        let _ = self.workbench.workbench_mut().activate_settings();
        self.language_server_settings.open();
        self.keyboard_shortcuts.close();
        let _ = self.git_branch_context_menu.dismiss();
        let _ = self.workspace_path_picker.dismiss();
        let _ = self.remote_connection_picker.dismiss();
        self.dismiss_remote_connection_manager();
        self.dismiss_remote_tunnel_manager();
        self.dismiss_session_context_menu();
        self.pending_focus = Some(zeta_settings::SETTINGS_SEARCH_INPUT);
        self.keybindings.cancel_chord();
    }

    /// Returns to the last selected session without fabricating a session for Settings.
    pub(super) fn activate_session_workbench_tab(&mut self) {
        let was_terminal = self.workspace_surface.is_terminal();
        let _ = self
            .workbench
            .workbench_mut()
            .tab_part_mut()
            .activate_last_session();
        if let Some(session_id) = self
            .workbench
            .workbench()
            .tab_part()
            .selected_session()
            .cloned()
        {
            let _ = self.activate_terminal_for_session(&session_id);
            if !was_terminal {
                let _ = self.bind_agent_pane();
            }
        }
        self.language_server_settings.close();
        self.keyboard_shortcuts.close();
        self.pending_focus = Some(if self.workspace_surface.is_editor() {
            crate::shell_interaction::FILE_EDITOR_DOCUMENT
        } else {
            crate::shell_interaction::COMPOSER
        });
    }

    pub(super) fn close_settings_tab(&mut self) {
        self.activate_session_workbench_tab();
    }
}
