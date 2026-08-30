use super::*;

impl WorkbenchApplication {
    pub(super) fn reload_theme(&mut self, system_scheme: ColorScheme) {
        let Ok(loader) = ThemeLoader::embedded() else {
            return;
        };
        let device_root = default_device_root();
        let loaded = loader.load(
            ThemeLoadOptions::new(&device_root, system_scheme)
                .with_default_entry(DEFAULT_THEME_ENTRY),
        );
        for diagnostic in &loaded.diagnostics {
            eprintln!("theme: {}", diagnostic.message);
        }
        let Ok(palette) = UiTheme::from_snapshot(&loaded.snapshot) else {
            return;
        };
        let editor_style = CodeEditorStyle::from_theme(palette);
        self.palette = palette;
        self.theme_scheme = loaded.snapshot.color_scheme();
        self.theme_follows_system = loaded.follows_system;
        self.session_pane.set_composer_style(editor_style.clone());
        self.code_editor_style = editor_style;
        self.scm
            .editor_mut()
            .set_style(zeta_editor::MultiDiffEditorStyle::from_theme(palette));
    }

    pub(super) fn show_files_pane(&mut self) {
        if self.active_main_pane_kind() == Some(PaneInputKind::Diff) {
            let Some(tab_key) = self.active_session_tab_key() else {
                return;
            };
            let Some(pane) = self
                .workbench
                .workbench()
                .pane_part(&tab_key)
                .map(|pane_part| pane_part.active_group())
            else {
                return;
            };
            if self
                .workbench
                .ensure_input_with(
                    &tab_key,
                    pane,
                    PaneInput::files(self.env.working_directory().to_path_buf()),
                    PaneBinding::new,
                )
                .is_none()
            {
                return;
            }
            self.files_pane_expanded = true;
            self.main_surface.show_agent();
            self.workbench.collapse_inspector();
            let _ = self.activate_pane_context(tab_key, pane);
            return;
        }
        self.open_main_input(PaneInput::files(self.env.working_directory().to_path_buf()));
    }

    pub(super) fn show_changes_pane(&mut self) {
        self.open_main_input(PaneInput::diff(self.env.working_directory().to_path_buf()));
    }

    /// Mounts one application capability as the active input of the current PaneGroup.
    fn open_main_input(&mut self, input: PaneInput) {
        let Some(tab_key) = self.active_session_tab_key() else {
            return;
        };
        let Some(pane) = self
            .workbench
            .workbench()
            .pane_part(&tab_key)
            .map(|pane_part| pane_part.active_group())
        else {
            return;
        };
        if self
            .workbench
            .open_or_activate_input_with(&tab_key, pane, input, PaneBinding::new)
            .is_none()
        {
            return;
        }
        self.main_surface.show_agent();
        self.workbench.collapse_inspector();
        let _ = self.activate_pane_context(tab_key, pane);
    }

    /// Restores the active Session's Agent pane after a file feature pane is dismissed.
    pub(super) fn show_agent_pane(&mut self) {
        self.files_pane_expanded = false;
        let _ = self.bind_agent_pane();
        self.main_surface.show_agent();
    }

    /// Binds the active Session's Agent descriptor without changing the visible surface.
    ///
    /// File-editor and terminal transitions use this form when the `MainSurface` has already
    /// selected the surface that should remain visible.
    pub(super) fn bind_agent_pane(&mut self) -> bool {
        let Some(tab_key) = self.active_session_tab_key() else {
            return false;
        };
        let Some(thread_id) = self
            .session_pane
            .thread()
            .map(|thread| thread.thread_id.clone())
        else {
            return false;
        };
        let Some(session_id) = tab_key.session_id().cloned() else {
            return false;
        };
        let Some(pane) = self
            .workbench
            .workbench()
            .pane_part(&tab_key)
            .map(|pane_part| pane_part.root_pane())
        else {
            return false;
        };
        if self
            .workbench
            .open_or_activate_input_with(
                &tab_key,
                pane,
                PaneInput::agent(session_id, thread_id),
                PaneBinding::new,
            )
            .is_none()
        {
            return false;
        }
        self.activate_pane_context(tab_key, pane)
    }

    pub(super) fn active_main_pane_kind(&self) -> Option<PaneInputKind> {
        let tab_key = self.active_session_tab_key()?;
        let pane = self
            .workbench
            .workbench()
            .pane_part(&tab_key)
            .map(|pane_part| pane_part.active_pane())?;
        self.workbench
            .workbench()
            .pane_part(&tab_key)
            .and_then(|pane_part| pane_part.pane_input(pane))
            .map(PaneInput::kind)
    }

    /// Restores the input selected before the Terminal surface was opened.
    pub(super) fn restore_main_pane_after_terminal(&mut self) {
        let Some(tab_key) = self.active_session_tab_key() else {
            self.show_agent_pane();
            return;
        };
        if !self.main_surface.is_editor() {
            let _ = self.bind_agent_pane();
            return;
        }
        let Some(pane) = self
            .workbench
            .workbench()
            .pane_part(&tab_key)
            .map(|pane_part| pane_part.root_pane())
        else {
            return;
        };
        let input = self
            .workbench
            .workbench()
            .pane_part(&tab_key)
            .and_then(|part| part.group(pane))
            .and_then(|group| {
                group.inputs().find(|input| {
                    matches!(input.kind(), PaneInputKind::Files | PaneInputKind::Diff)
                })
            })
            .cloned();
        let Some(input) = input else {
            let _ = self.bind_agent_pane();
            return;
        };
        if self
            .workbench
            .open_or_activate_input_with(&tab_key, pane, input, PaneBinding::new)
            .is_none()
        {
            return;
        }
        let _ = self.activate_pane_context(tab_key, pane);
    }

    pub(super) fn fail(&mut self, message: impl std::fmt::Display) {
        eprintln!("{APP_DISPLAY_NAME} failed: {message}");
        self.failed = true;
    }

    pub(super) fn redraw_frame(&mut self, context: &mut WindowContext<'_, WorkbenchEvent>) {
        let now = Instant::now();
        let retained_report = self.retained_runtime.advance(now);
        let _ = retained_report
            .animation()
            .schedule(&mut self.frame_scheduler);
        match self.frame_scheduler.take() {
            Some(FrameInvalidation::Fragment) => match self.frame_scheduler.take_fragment_ids() {
                Some(_) => self.rebuild_presentation(),
                None => self.rebuild_overlay_presentation(),
            },
            Some(FrameInvalidation::Rebuild) => self.rebuild_presentation(),
            Some(FrameInvalidation::Render) | None => {}
        }
        let Some(presentation) = self.presentation.as_ref() else {
            return;
        };
        if let Err(error) = context.present_frame(presentation.frame(), &self.ui_dispatch) {
            self.fail(&error);
            context.exit_with_error(ApplicationError::host("app frame rendering", error));
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
        if self.main_surface.is_terminal() {
            ScreenBuffer::Alternate
        } else {
            ScreenBuffer::Primary
        }
    }

    pub(super) fn terminal_size(&self) -> GridSize {
        terminal_grid_size_for_viewport(
            self.logical_viewport(),
            self.active_screen(),
            self.workbench.tab_container_state(),
            self.workbench.inspector_state(),
        )
    }
}
