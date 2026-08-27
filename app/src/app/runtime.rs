use super::*;

impl NativeApp {
    pub(super) fn reload_theme(&mut self, system_scheme: ColorScheme) {
        let Ok(loader) = ThemeLoader::embedded() else {
            return;
        };
        let device_root = default_device_root();
        let loaded = loader.load(
            ThemeLoadOptions::new(&device_root, ThemeSurface::Graphical, system_scheme)
                .with_default_entry(DEFAULT_THEME_ENTRY),
        );
        for diagnostic in &loaded.diagnostics {
            eprintln!("theme: {}", diagnostic.message);
        }
        let Ok(palette) = ShellPalette::from_theme(&loaded.snapshot) else {
            return;
        };
        let Ok(editor_style) = code_editor_style(&loaded.snapshot) else {
            return;
        };
        self.palette = palette;
        self.theme_scheme = loaded.snapshot.color_scheme();
        self.theme_follows_system = loaded.follows_system;
        self.composer.set_input_style(editor_style.clone());
        self.code_editor_style = editor_style;
        self.workspace_pane_host
            .set_editor_style(palette.multi_diff_editor_style());
    }

    /// Mounts a workspace feature as the active leaf of the current Session workbench.
    ///
    /// Files and Changes are ordinary `PaneInput`s. Their feature state stays in the workspace
    /// feature, while this host only changes the descriptive binding and active-pane context.
    pub(super) fn select_workspace_pane_view(&mut self, view: WorkspacePaneView) {
        let Some(tab_key) = self.active_session_tab_key() else {
            return;
        };
        let Some(pane) = self
            .workbench_host
            .workbench()
            .pane_part(&tab_key)
            .map(|pane_part| pane_part.root_pane())
        else {
            return;
        };
        let host_key = (PaneHostScope::Tab(tab_key.clone()), pane);
        let current = self
            .workbench_host
            .workbench()
            .pane_part(&tab_key)
            .and_then(|pane_part| pane_part.pane_input(pane))
            .cloned();
        if current.as_ref().is_some_and(|input| {
            !matches!(input.kind(), PaneInputKind::Files | PaneInputKind::Diff)
        }) {
            if let Some(current) = current {
                let _ = self
                    .workbench_host
                    .workbench_mut()
                    .remember_workspace_return(&tab_key, current);
            }
        }
        let input = match view {
            WorkspacePaneView::Changes => {
                PaneInput::diff(self.workspace_context.working_directory().to_path_buf())
            }
            WorkspacePaneView::Files => {
                PaneInput::files(self.workspace_context.working_directory().to_path_buf())
            }
        };
        self.workbench_host
            .workbench_mut()
            .mount_input(&tab_key, pane, input);
        self.workbench_host.pane_host_mut().remove(&host_key);
        self.workbench_host
            .pane_host_mut()
            .insert(host_key, PaneBinding::new());
        self.workspace_surface.show_agent();
        self.inspector_part.collapse();
        let _ = self.activate_pane_context(tab_key, pane);
    }

    /// Restores the active Session's Agent pane after a workspace feature pane is dismissed.
    pub(super) fn show_agent_pane(&mut self) {
        let _ = self.bind_agent_pane();
        self.workspace_surface.show_agent();
    }

    /// Binds the active Session's Agent descriptor without changing the visible surface.
    ///
    /// File-editor and terminal transitions use this form when the `WorkspaceSurface` has already
    /// selected the surface that should remain visible.
    pub(super) fn bind_agent_pane(&mut self) -> bool {
        let Some(tab_key) = self.active_session_tab_key() else {
            return false;
        };
        let Some(thread_id) = self
            .thread_projection
            .thread()
            .map(|thread| thread.thread_id.clone())
        else {
            return false;
        };
        let Some(session_id) = tab_key.session_id().cloned() else {
            return false;
        };
        let Some(pane) = self
            .workbench_host
            .workbench()
            .pane_part(&tab_key)
            .map(|pane_part| pane_part.root_pane())
        else {
            return false;
        };
        self.workbench_host.workbench_mut().mount_input(
            &tab_key,
            pane,
            PaneInput::agent(session_id, thread_id),
        );
        let host_key = (PaneHostScope::Tab(tab_key.clone()), pane);
        self.workbench_host.pane_host_mut().remove(&host_key);
        self.workbench_host
            .pane_host_mut()
            .insert(host_key, PaneBinding::new());
        let _ = self
            .workbench_host
            .workbench_mut()
            .clear_workspace_return(&tab_key);
        self.activate_pane_context(tab_key, pane)
    }

    pub(super) fn active_workspace_pane_kind(&self) -> Option<PaneInputKind> {
        let tab_key = self.active_session_tab_key()?;
        let pane = self
            .workbench_host
            .workbench()
            .pane_part(&tab_key)
            .map(|pane_part| pane_part.active_pane())?;
        self.workbench_host
            .workbench()
            .pane_part(&tab_key)
            .and_then(|pane_part| pane_part.pane_input(pane))
            .map(PaneInput::kind)
    }

    /// Restores a Files/Changes pane that was temporarily replaced by the Terminal surface.
    pub(super) fn restore_workspace_pane_after_terminal(&mut self) {
        let Some(tab_key) = self.active_session_tab_key() else {
            self.show_agent_pane();
            return;
        };
        let Some(input) = self
            .workbench_host
            .workbench_mut()
            .take_workspace_return(&tab_key)
        else {
            let _ = self.bind_agent_pane();
            return;
        };
        if !matches!(input.kind(), PaneInputKind::Files | PaneInputKind::Diff) {
            let _ = self.bind_agent_pane();
            return;
        }
        let Some(pane) = self
            .workbench_host
            .workbench()
            .pane_part(&tab_key)
            .map(|pane_part| pane_part.root_pane())
        else {
            return;
        };
        self.workbench_host
            .workbench_mut()
            .mount_input(&tab_key, pane, input);
        let host_key = (PaneHostScope::Tab(tab_key.clone()), pane);
        self.workbench_host.pane_host_mut().remove(&host_key);
        self.workbench_host
            .pane_host_mut()
            .insert(host_key, PaneBinding::new());
        self.workspace_surface.show_agent();
        let _ = self.activate_pane_context(tab_key, pane);
    }

    pub(super) fn fail(&mut self, message: impl std::fmt::Display) {
        eprintln!("{PRODUCT_DISPLAY_NAME} failed: {message}");
        self.failed = true;
    }

    pub(super) fn redraw_frame(&mut self, context: &mut WindowContext<'_, NativeEvent>) {
        let _trace = session_switch_trace::Span::frame("redraw");
        let now = Instant::now();
        let retained_report = self.retained_runtime.advance(now);
        let mut retained_cleanup_failed = false;
        if !retained_report.fragment().removed_ids().is_empty() {
            if let Some(presentation) = self.presentation.as_mut() {
                for id in retained_report.fragment().removed_ids() {
                    if presentation.remove_retained_fragment(*id).is_err() {
                        retained_cleanup_failed = true;
                    }
                }
            } else {
                retained_cleanup_failed = true;
            }
        }
        if retained_cleanup_failed {
            self.rebuild_presentation_on_next_redraw();
        }
        let _ = retained_report
            .animation()
            .schedule(&mut self.frame_scheduler);
        match self.frame_scheduler.take() {
            Some(FrameInvalidation::Fragment) => match self.frame_scheduler.take_fragment_ids() {
                Some(ids) => self.rebuild_shell_fragments(ids),
                None => self.rebuild_overlay_presentation(),
            },
            Some(FrameInvalidation::Rebuild) => self.rebuild_presentation(),
            Some(FrameInvalidation::Render) | None => {}
        }
        let Some(presentation) = self.presentation.as_ref() else {
            return;
        };
        let _render_trace = session_switch_trace::Span::frame("renderer.render_scene");
        if let Err(error) = context.present_frame(presentation.frame(), &self.ui_dispatch) {
            self.fail(&error);
            context.exit_with_error(ApplicationError::product("app frame rendering", error));
        }
    }

    pub(super) fn window_viewport(&self) -> LogicalViewport {
        LogicalViewport::from_physical(
            self.physical_extent.width,
            self.physical_extent.height,
            self.scale_factor,
        )
    }

    pub(super) fn logical_viewport(&self) -> LogicalViewport {
        self.window_viewport()
    }

    pub(super) fn active_screen(&self) -> ScreenBuffer {
        if self.workspace_surface.is_terminal() {
            ScreenBuffer::Alternate
        } else {
            ScreenBuffer::Primary
        }
    }

    pub(super) fn terminal_size(&self) -> GridSize {
        terminal_grid_size_for_viewport(
            self.logical_viewport(),
            self.active_screen(),
            self.tab_container,
            self.inspector_part,
        )
    }
}
