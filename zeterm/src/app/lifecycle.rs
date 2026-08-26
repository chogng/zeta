use super::events::handle_terminal_event;
use super::*;

impl App<NativeEvent> for NativeApp {
    fn ready(&mut self, context: &mut AppContext<'_, NativeEvent>) {
        if self.window.is_some() {
            self.request_redraw();
            return;
        }

        let options = WindowOptions::new(PRODUCT_DISPLAY_NAME)
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT))
            .with_chrome(WindowChrome::ContentUnderTitlebar);
        let opened_window = match context.open_window(options) {
            Ok(opened_window) => opened_window,
            Err(error) => {
                self.fail(&error);
                context.exit_with_error(error);
                return;
            }
        };
        let window = opened_window.handle();
        let system_scheme = match window.theme() {
            Ok(Some(Theme::Dark)) => ColorScheme::Dark,
            Ok(Some(Theme::Light) | None) => ColorScheme::Light,
            Err(error) => {
                context.exit_with_error(ApplicationError::product(
                    "initial window theme query",
                    error,
                ));
                return;
            }
        };
        self.reload_theme(system_scheme);
        if let Err(error) = window.set_theme((!self.theme_follows_system).then_some(
            match self.theme_scheme {
                ColorScheme::Dark | ColorScheme::HighContrastDark => Theme::Dark,
                ColorScheme::Light | ColorScheme::HighContrastLight => Theme::Light,
            },
        )) {
            context.exit_with_error(ApplicationError::product(
                "initial window theme update",
                error,
            ));
            return;
        }
        self.physical_extent = opened_window.metrics().physical_extent();
        self.scale_factor = opened_window.metrics().scale_factor();
        let terminal_size = terminal_grid_size_for_viewport(
            self.logical_viewport(),
            ScreenBuffer::Primary,
            self.session_sidebar,
            self.sidebar_part,
        );
        if let Err(error) = self.terminal_workspace.spawn_initial(terminal_size) {
            self.fail(error);
            context.exit();
            return;
        }
        if self.app_server_host.is_remote()
            && let Err(error) = self
                .language_service
                .start_remote(self.event_proxy.clone(), self.app_server_host.clone())
        {
            self.fail(error);
            context.exit();
            return;
        }
        self.agent_session =
            match AgentSession::spawn(self.event_proxy.clone(), self.app_server_host.clone()) {
                Ok(session) => Some(session),
                Err(error) => {
                    self.fail(error);
                    context.exit();
                    return;
                }
            };
        self.window = Some(window);
        self.rebuild_presentation();
        self.sync_input_focus();
        self.request_redraw();
    }

    fn resumed(&mut self, _context: &mut AppContext<'_, NativeEvent>) {
        self.request_redraw();
    }

    fn window_event(&mut self, context: &mut WindowContext<'_, NativeEvent>, event: WindowEvent) {
        if self.window.as_ref().map(WindowHandle::id) != Some(context.id()) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                context.exit();
            }
            WindowEvent::Resized(size) => {
                self.terminal_selection.clear();
                self.physical_extent = PhysicalExtent::new(size.width, size.height);
                self.rebuild_presentation();
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.terminal_selection.clear();
                self.scale_factor = scale_factor;
                self.rebuild_presentation();
                self.request_redraw();
            }
            WindowEvent::ThemeChanged(theme) => {
                if !self.theme_follows_system {
                    return;
                }
                let system_scheme = match theme {
                    Theme::Dark => ColorScheme::Dark,
                    Theme::Light => ColorScheme::Light,
                };
                self.reload_theme(system_scheme);
                self.rebuild_presentation_on_next_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer_moved(position.x, position.y);
            }
            WindowEvent::CursorLeft { .. } => self.pointer_left(),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => self.keyboard_input(event),
            WindowEvent::Ime(event) => self.ime_input(event),
            WindowEvent::Focused(false) => {
                self.modifiers = ModifiersState::default();
                self.keybindings.cancel_chord();
                self.keyboard_shortcuts.window_blurred();
                self.terminal_pointer.cancel();
                self.file_editor_input.cancel_pointer();
                self.cancel_session_sidebar_resize();
                self.cancel_sidebar_resize();
                if self.cancel_terminal_pane_resize() {
                    self.update_cursor();
                }
                self.sidebar_pane_workspace.cancel_multi_diff_scrollbar();
                self.terminal_scroll.cancel_scrollbar();
                self.session_context_menu.dismiss();
                self.git_branch_context_menu.dismiss();
                self.workspace_path_picker.dismiss();
                self.ui_dispatch.window_blurred();
                self.sync_input_focus();
                self.rebuild_presentation();
                self.request_redraw();
            }
            WindowEvent::Focused(true) => {
                self.ui_dispatch.window_focused();
                self.sync_input_focus();
                self.rebuild_presentation();
                self.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.mouse_button_changed(state, button);
            }
            WindowEvent::MouseWheel { delta, .. } => self.mouse_wheel(delta),
            WindowEvent::Occluded(false) => {
                // macOS can reject initial surface acquisition while the new window activates.
                // The visible transition is the next reliable opportunity to present that frame.
                self.request_redraw();
            }
            WindowEvent::Occluded(true) => {}
            _ => {}
        }
    }

    fn redraw(&mut self, context: &mut WindowContext<'_, NativeEvent>) {
        self.redraw_frame(context);
    }

    fn accessibility_action(
        &mut self,
        _context: &mut AppContext<'_, NativeEvent>,
        action: AccessibilityAction,
    ) {
        if self.window.as_ref().map(WindowHandle::id) != Some(action.window()) {
            return;
        }
        let Some(presentation) = self.presentation.as_ref() else {
            return;
        };
        let outcome = match action.kind() {
            AccessibilityActionKind::Focus => self
                .ui_dispatch
                .focus_element(presentation.interaction_frame(), action.target()),
            AccessibilityActionKind::Activate => self
                .ui_dispatch
                .activate_element(presentation.interaction_frame(), action.target()),
        };
        self.apply_dispatch_outcome(outcome);
    }

    fn user_event(&mut self, _context: &mut AppContext<'_, NativeEvent>, event: NativeEvent) {
        match event {
            NativeEvent::Agent(event) => {
                self.handle_agent_session_event(event);
                return;
            }
            NativeEvent::LanguageService(event) => {
                self.language_service
                    .handle_event(event, &self.file_editor_host);
                if let Some(target) = self
                    .language_service
                    .take_definitions()
                    .and_then(|definitions| definitions.targets.into_iter().next())
                {
                    self.open_language_definition(target);
                    return;
                }
                self.rebuild_presentation();
                self.request_redraw();
                return;
            }
            NativeEvent::RemoteLanguage(event) => {
                self.language_service
                    .handle_remote_event(event, &self.file_editor_host);
                if let Some(target) = self
                    .language_service
                    .take_definitions()
                    .and_then(|definitions| definitions.targets.into_iter().next())
                {
                    self.open_language_definition(target);
                    return;
                }
                self.rebuild_presentation();
                self.request_redraw();
                return;
            }
            NativeEvent::RemoteWindowLaunch(event) => {
                self.handle_remote_window_launch_event(event);
                return;
            }
            NativeEvent::RemoteTunnel(event) => {
                self.handle_remote_tunnel_event(event);
                return;
            }
            NativeEvent::Terminal(event) => {
                handle_terminal_event(self, event.key, event.event);
                return;
            }
            NativeEvent::TerminalReady(ready) => {
                match self.terminal_workspace.handle_ready(ready) {
                    TerminalReadyOutcome::Active {
                        key,
                        buffered_events,
                    } => {
                        if buffered_events.is_empty() {
                            self.rebuild_presentation_on_next_redraw();
                        } else {
                            for event in buffered_events {
                                handle_terminal_event(self, key, event);
                            }
                        }
                    }
                    TerminalReadyOutcome::Inactive {
                        key,
                        buffered_events,
                    } => {
                        for event in buffered_events {
                            handle_terminal_event(self, key, event);
                        }
                    }
                    TerminalReadyOutcome::Failed { key, error } => {
                        session_switch_trace::event(
                            None,
                            "terminal-ready-failed",
                            format_args!("key={key:?} error={error}"),
                        );
                        eprintln!("could not create terminal runtime: {error}");
                        self.rebuild_presentation_on_next_redraw();
                    }
                    TerminalReadyOutcome::Ignored { key } => {
                        session_switch_trace::event(
                            None,
                            "terminal-ready-ignored",
                            format_args!("key={key:?}"),
                        );
                    }
                }
                return;
            }
        }
    }

    fn about_to_wait(&mut self, context: &mut AppContext<'_, NativeEvent>) {
        let now = Instant::now();
        self.keybindings.advance_chord(now);
        self.advance_keyboard_shortcuts(now);
        if let KeybindingsResourcePoll::Rejected(error) =
            self.keybindings_resource.poll(now, &mut self.keybindings)
        {
            eprintln!("{error}");
        }
        let caret_changed = matches!(
            self.caret_blink.advance(now),
            CaretBlinkAdvance::VisibilityChanged(_)
        );
        let scrollbar_changed = self
            .sidebar_pane_workspace
            .advance_multi_diff_scrollbar(now);
        let terminal_scrollbar_changed = self.terminal_scroll.advance_scrollbar(now);
        let session_sash_changed = self.session_sidebar.advance_sash(now);
        let sidebar_sash_changed = self.sidebar_part.advance_sash(now);
        let sash_changed = session_sash_changed || sidebar_sash_changed;
        let retained_runtime_due = self
            .retained_runtime
            .next_deadline()
            .is_some_and(|deadline| deadline <= now);
        let file_search_changed = self.sidebar_pane_workspace.poll_file_search();
        let file_editor_auto_scrolled = self.advance_file_editor_auto_scroll(now);
        if caret_changed
            || scrollbar_changed
            || terminal_scrollbar_changed
            || file_search_changed
            || file_editor_auto_scrolled
            || sash_changed
        {
            self.rebuild_presentation_on_next_redraw();
        } else if retained_runtime_due {
            self.request_redraw();
        }
        let mut deadlines = FrameDeadlineSet::default();
        for deadline in [
            self.caret_blink.next_deadline(),
            self.sidebar_pane_workspace.multi_diff_scrollbar_deadline(),
            self.terminal_scroll.scrollbar_deadline(),
            self.retained_runtime.next_deadline(),
            self.session_sidebar.sash_deadline(),
            self.sidebar_part.sash_deadline(),
            self.keybindings.chord_deadline(),
            self.keyboard_shortcuts_deadline(),
            Some(self.keybindings_resource.next_deadline()),
            self.file_editor_input.auto_scroll_deadline(),
        ]
        .into_iter()
        .flatten()
        {
            deadlines.include(deadline);
        }
        if self.sidebar_pane_workspace.file_search_pending() {
            deadlines.include(now + std::time::Duration::from_millis(50));
        }
        let control_flow = match deadlines.next_deadline() {
            Some(deadline) => ControlFlow::WaitUntil(deadline),
            None => ControlFlow::Wait,
        };
        context.set_control_flow(control_flow);
    }
}
