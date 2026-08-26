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
        self.sidebar_pane_workspace
            .set_editor_style(palette.multi_diff_editor_style());
    }

    /// Synchronizes the sidebar's logical content selection with its mounted PaneInput.
    ///
    /// The sidebar Part owns visibility and width, while this binding identifies the content leaf
    /// inside it. The feature crate keeps Files/SCM state; the Native host only changes which
    /// feature input is mounted.
    pub(super) fn select_sidebar_pane_view(&mut self, view: AgentSidebarView) {
        let input = match view {
            AgentSidebarView::Changes => {
                PaneInput::diff(self.workspace_context.working_directory().to_path_buf())
            }
            AgentSidebarView::Files => {
                PaneInput::files(self.workspace_context.working_directory().to_path_buf())
            }
        };
        self.pane_host.insert(
            (PaneHostScope::Sidebar, self.sidebar_pane_group.root_pane()),
            PaneBinding::new(input),
        );
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
            context.exit_with_error(ApplicationError::product("zeterm frame rendering", error));
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
            self.session_sidebar,
            self.sidebar_part,
        )
    }
}
