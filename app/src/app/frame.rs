use super::presentation::with_shell_presentation_model;
use super::*;

impl NativeApp {
    pub(super) fn rebuild_presentation(&mut self) {
        let _trace = session_switch_trace::Span::new(None, "rebuild_presentation");
        let viewport = self.logical_viewport();
        let active_screen = self.active_screen();
        let terminal_size = terminal_grid_size_for_viewport(
            viewport,
            active_screen,
            self.tab_container,
            self.inspector_part,
        );
        self.resize_terminal_panes(viewport, active_screen, terminal_size);
        let scroll_limit = self.terminal_scroll_limit();
        self.terminal_scroll.clamp(scroll_limit);
        let window_control_insets = self
            .window
            .as_ref()
            .map(WindowHandle::window_control_insets)
            .unwrap_or(WindowControlInsets::NONE);
        let inspector_sash_state = self.inspector_resizable.presentation();
        let mut presentation = with_shell_presentation_model(
            self,
            window_control_insets,
            |model, text_layout, animation_bindings| {
                build_shell_presentation_with_animation_bindings(
                    viewport,
                    model,
                    text_layout,
                    inspector_sash_state,
                    animation_bindings,
                )
            },
        );
        let requested_focus = self.pending_focus.take();
        let preferred_focus = requested_focus.unwrap_or_else(|| {
            if self.workspace_surface.is_editor() {
                FILE_EDITOR_DOCUMENT
            } else {
                COMPOSER
            }
        });
        let focus_outcome = if requested_focus.is_some() {
            self.ui_dispatch
                .focus_element(presentation.interaction_frame(), preferred_focus)
        } else {
            self.ui_dispatch
                .reconcile_focus(presentation.interaction_frame(), preferred_focus)
        };
        if focus_outcome.invalidation != DispatchInvalidation::None {
            let inspector_sash_state = self.inspector_resizable.presentation();
            presentation = with_shell_presentation_model(
                self,
                window_control_insets,
                |model, text_layout, animation_bindings| {
                    build_shell_presentation_with_animation_bindings(
                        viewport,
                        model,
                        text_layout,
                        inspector_sash_state,
                        animation_bindings,
                    )
                },
            );
        }
        let pointer_requires_rebuild = self.cursor_position.is_some_and(|point| {
            let outcome = self
                .ui_dispatch
                .pointer_moved(point, presentation.interaction_frame());
            let sash_changed = self.sync_sash_pointer_presence(Instant::now());
            outcome.invalidation != DispatchInvalidation::None || sash_changed
        });
        if pointer_requires_rebuild {
            let inspector_sash_state = self.inspector_resizable.presentation();
            presentation = with_shell_presentation_model(
                self,
                window_control_insets,
                |model, text_layout, animation_bindings| {
                    build_shell_presentation_with_animation_bindings(
                        viewport,
                        model,
                        text_layout,
                        inspector_sash_state,
                        animation_bindings,
                    )
                },
            );
        }
        self.mount_shell_fragments(&mut presentation);
        self.presentation = Some(presentation);
        self.frame_scheduler.clear();
        if requested_focus.is_some() {
            self.sync_input_focus();
        }
        self.update_ime_cursor_area();
    }

    pub(super) fn resize_terminal_panes(
        &mut self,
        viewport: LogicalViewport,
        active_screen: ScreenBuffer,
        fallback_size: GridSize,
    ) {
        let Some(tab_key) = self
            .workbench
            .workbench()
            .tab_part()
            .active_tab_key()
            .cloned()
        else {
            self.terminal_workspace.resize_all(fallback_size);
            return;
        };
        let Some(layout) = self.workbench.workbench().pane_part(&tab_key) else {
            self.terminal_workspace.resize_all(fallback_size);
            return;
        };
        let panes = terminal_pane_bounds_for_viewport(
            viewport,
            active_screen,
            self.tab_container,
            self.inspector_part,
            layout,
        );
        if panes.is_empty() {
            self.terminal_workspace.resize_all(fallback_size);
            return;
        }
        let resize_requests = panes
            .into_iter()
            .filter_map(|(pane, bounds)| {
                self.workbench
                    .pane_host()
                    .binding(&(PaneHostScope::Tab(tab_key.clone()), pane))
                    .and_then(PaneBinding::terminal_key)
                    .map(|key| (key, terminal_grid_size_for_bounds(bounds)))
            })
            .collect::<Vec<_>>();
        for (key, size) in resize_requests {
            self.terminal_workspace.resize_key(key, size);
        }
    }

    pub(super) fn rebuild_overlay_presentation(&mut self) {
        let viewport = self.logical_viewport();
        let window_control_insets = self
            .window
            .as_ref()
            .map(WindowHandle::window_control_insets)
            .unwrap_or(WindowControlInsets::NONE);
        let Some(mut presentation) = self.presentation.take() else {
            self.rebuild_presentation();
            return;
        };
        let rebuilt = with_shell_presentation_model(
            self,
            window_control_insets,
            |model, text_layout, _animation_bindings| {
                rebuild_shell_overlays(&mut presentation, viewport, model, text_layout)
            },
        );
        if !rebuilt {
            self.presentation = Some(presentation);
            self.rebuild_presentation();
            return;
        }
        self.mount_shell_fragments(&mut presentation);
        self.presentation = Some(presentation);
        self.frame_scheduler.clear();
        self.update_ime_cursor_area();
    }

    pub(super) fn rebuild_presentation_on_next_redraw(&mut self) {
        if self.frame_scheduler.request(FrameInvalidation::Rebuild) == FrameSchedule::RequestFrame {
            self.request_redraw();
        }
    }

    pub(super) fn rebuild_overlay_on_next_redraw(&mut self) {
        if self.frame_scheduler.request(FrameInvalidation::Fragment) == FrameSchedule::RequestFrame
        {
            self.request_redraw();
        }
    }

    pub(super) fn rebuild_fragment_on_next_redraw(&mut self, id: ElementId) {
        if self.frame_scheduler.request_fragment(id) == FrameSchedule::RequestFrame {
            self.request_redraw();
        }
    }

    pub(super) fn mount_shell_fragments(&mut self, presentation: &mut ShellPresentation) {
        let fragment = language_server_settings::LANGUAGE_SERVER_SWITCH;
        let Some(content) = presentation.language_server_settings_content else {
            presentation.forget_retained_fragment(fragment);
            if self
                .retained_runtime
                .fragment_registry()
                .state(fragment)
                .is_some()
            {
                self.retained_runtime
                    .unmount(fragment)
                    .expect("retained shell fragment should be mounted before unmount");
            }
            return;
        };
        self.retained_runtime.mount(fragment);
        let target = language_server_settings::switch_animation_target(
            self.language_server_settings.switch_selection(),
        );
        let progress = self
            .retained_runtime
            .animation_registry()
            .value(language_server_settings::SWITCH_ANIMATION_KEY)
            .unwrap_or(target);
        presentation.record_retained_fragment(fragment);
        presentation.scene_mut().with_fragment(fragment, |scene| {
            language_server_settings::paint_switch_fragment(
                scene,
                content,
                &self.language_server_settings,
                self.palette,
                &self.ui_dispatch,
                progress,
            );
        });
    }

    pub(super) fn rebuild_shell_fragments(&mut self, ids: Vec<ElementId>) {
        let Some(mut presentation) = self.presentation.take() else {
            self.rebuild_presentation();
            return;
        };
        let target = language_server_settings::switch_animation_target(
            self.language_server_settings.switch_selection(),
        );
        let progress = self
            .retained_runtime
            .animation_registry()
            .value(language_server_settings::SWITCH_ANIMATION_KEY)
            .unwrap_or(target);
        let mut rebuilt = true;
        for id in ids {
            rebuilt &= rebuild_shell_fragment(
                &mut presentation,
                id,
                &self.language_server_settings,
                self.palette,
                &self.ui_dispatch,
                progress,
            );
        }
        self.presentation = Some(presentation);
        if rebuilt {
            self.frame_scheduler.clear();
        } else {
            self.rebuild_presentation();
        }
    }
}
