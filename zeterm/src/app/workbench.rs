use super::*;

impl NativeApp {
    pub(super) fn ensure_terminal_for_session(&mut self, session_id: &SessionId) -> bool {
        match self
            .terminal_workspace
            .ensure_for_session(session_id, self.terminal_size())
        {
            Ok(()) => {
                let tab_key = TabInputKey::session(session_id.clone());
                let (group_key, root_pane) = {
                    let group = self.pane_groups.entry(tab_key.clone()).or_default();
                    (tab_key.clone(), group.root_pane())
                };
                if let Some(terminal_key) = self.terminal_workspace.key_for_session(session_id) {
                    if !self.pane_host.ensure_terminal(
                        (PaneHostScope::Tab(group_key), root_pane),
                        session_id,
                        terminal_key,
                    ) {
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
        let pane = self
            .pane_groups
            .entry(tab_key.clone())
            .or_default()
            .active_pane();
        let host_key = (PaneHostScope::Tab(tab_key.clone()), pane);
        let current = self
            .pane_host
            .binding(&host_key)
            .map(|binding| binding.input().clone());
        if current
            .as_ref()
            .is_some_and(|input| matches!(input.kind(), PaneInputKind::Files | PaneInputKind::Diff))
        {
            if let Some(current) = current {
                self.workspace_pane_returns.insert(tab_key.clone(), current);
            }
        }
        let Some(terminal_key) = self
            .pane_host
            .terminal_key(&host_key)
            .or_else(|| self.terminal_workspace.key_for_session(session_id))
        else {
            return false;
        };
        self.pane_host.insert(
            host_key.clone(),
            PaneBinding::new(PaneInput::terminal(session_id.clone())),
        );
        if !self
            .pane_host
            .ensure_terminal(host_key, session_id, terminal_key)
        {
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
        if let Some(group) = self.pane_groups.get_mut(&tab_key) {
            if !group.activate(pane) {
                return false;
            }
        } else {
            return false;
        }
        let host_binding = (PaneHostScope::Tab(tab_key.clone()), pane);
        let Some(pane_binding) = self.pane_host.binding(&host_binding) else {
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
                .pane_host
                .terminal_key(&(PaneHostScope::Tab(tab_key.clone()), *pane)),
            None => self.terminal_workspace.active_key(),
        }
    }

    pub(super) fn update_terminal_status(&mut self, key: TerminalSessionKey, status: &str) {
        let Some(session_id) = self.terminal_workspace.session_id_for_key(key) else {
            return;
        };
        self.tab_inputs.update_status(&session_id, status);
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
        self.tab_inputs
            .active_key()
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
        let pane = self
            .pane_groups
            .entry(tab_key.clone())
            .or_default()
            .split_active(direction);
        self.pane_host.insert(
            (PaneHostScope::Tab(tab_key.clone()), pane),
            pane_input::PaneBinding::terminal(session_id, terminal_key),
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
        let Some(group) = self.pane_groups.get_mut(&tab_key) else {
            return;
        };
        let previous_active = group.active_pane();
        let root_pane = group.root_pane();
        let Some(removed_pane) = group.close_active() else {
            return;
        };
        let replacement_pane = group.active_pane();
        let removed_binding = (tab_key.clone(), removed_pane);
        let replacement_binding = (tab_key.clone(), replacement_pane);
        let removed_host_binding = (PaneHostScope::Tab(tab_key.clone()), removed_pane);
        let replacement_host_binding = (PaneHostScope::Tab(tab_key.clone()), replacement_pane);
        let removed_binding_state = self.pane_host.remove(&removed_host_binding);
        let removed_key = removed_binding_state
            .as_ref()
            .and_then(|binding| binding.terminal_key());
        if removed_pane == root_pane {
            let replacement_binding_state = self.pane_host.remove(&replacement_host_binding);
            let replacement_key = replacement_binding_state
                .as_ref()
                .and_then(|binding| binding.terminal_key());
            if let Some(replacement_key) = replacement_key {
                let _ = self.terminal_workspace.remove_key(replacement_key);
            }
            if let Some(removed_binding_state) = removed_binding_state {
                self.pane_host
                    .insert(replacement_host_binding.clone(), removed_binding_state);
            } else if let Some(mut replacement_binding_state) = replacement_binding_state {
                if replacement_key.is_some() {
                    replacement_binding_state.clear_runtime();
                }
                self.pane_host
                    .insert(replacement_host_binding.clone(), replacement_binding_state);
            }
            if let Some(view) = self.pane_view_states.remove(&removed_binding) {
                self.pane_view_states
                    .insert(replacement_binding.clone(), view);
            }
        } else {
            if let Some(removed_key) = removed_key {
                let _ = self.terminal_workspace.remove_key(removed_key);
            }
            self.pane_view_states.remove(&removed_binding);
        }
        if self.active_pane.as_ref() == Some(&removed_binding)
            || self.active_pane.as_ref() == Some(&(tab_key.clone(), previous_active))
        {
            self.active_pane = None;
        }
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
        let pane = {
            let Some(group) = self.pane_groups.get_mut(&tab_key) else {
                return;
            };
            if next {
                group.focus_next()
            } else {
                group.focus_previous()
            }
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
        let group = self.pane_groups.get(&tab_key)?;
        terminal_pane_sash_for_viewport(
            self.logical_viewport(),
            self.active_screen(),
            self.session_sidebar,
            self.sidebar_part,
            group,
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
        let changed = self
            .pane_groups
            .get_mut(&resize.tab_key)
            .is_some_and(|group| group.resize_split(resize.split_id, next));
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
