use super::*;

pub(crate) struct NativeApp {
    pub(super) window: Option<WindowHandle>,
    pub(super) presentation: Option<ShellPresentation>,
    pub(super) frame_scheduler: FrameScheduler,
    pub(super) retained_runtime: RetainedRuntime,
    pub(super) inspector_part: InspectorPartState,
    pub(super) workspace_pane_host: WorkspacePaneHost,
    pub(super) file_editor_host: FileEditorHost,
    pub(super) file_editor_input: FileEditorInputState,
    pub(super) file_editor_search: file_editor_search::FileEditorSearchState,
    pub(super) language_service: language_service_host::NativeLanguageService,
    pub(super) tab_container: TabContainerState,
    pub(super) session_search: SessionSearch,
    pub(super) workbench_host: WorkbenchHost,
    pub(super) pane_view_states: HashMap<(TabInputKey, PaneId), TerminalPaneViewState>,
    /// Last host view projection used to save and restore feature-specific view state.
    ///
    /// The canonical active group remains in the active `zeta_workbench::PanePart`; this is only a transient
    /// host identity for the terminal view state cache.
    pub(super) active_pane: Option<(TabInputKey, PaneId)>,
    pub(super) terminal_pane_resize: Option<TerminalPaneResize>,
    pub(super) session_context_menu: SessionContextMenuState,
    pub(super) git_branch_context_menu: GitBranchContextMenuState,
    pub(super) workspace_path_picker: WorkspacePathPickerState,
    pub(super) remote_connection_picker: RemoteConnectionPickerState,
    pub(super) remote_connection_manager: RemoteConnectionManagerState,
    pub(super) remote_connection_launch: Option<remote_connection_process::RemoteConnectionLaunch>,
    pub(super) remote_tunnel_manager: RemoteTunnelManagerState,
    pub(super) remote_tunnel_host: Option<NativeRemoteTunnelHost>,
    pub(super) ui_dispatch: UiDispatch,
    pub(super) agent_session: Option<AgentSession>,
    pub(super) app_server_host: AppServerHost,
    pub(super) thread_projection: ThreadProjection,
    pub(super) thread_timeline_scroll: ThreadTimelineScroll,
    pub(super) workspace_surface: WorkspaceSurface,
    pub(super) workspace_context: WorkspaceContext,
    pub(super) composer: Composer,
    pub(super) text_layout: TextInputLayoutEngine,
    pub(super) caret_blink: CaretBlinkController,
    pub(super) code_editor_style: CodeEditorStyle,
    pub(super) event_proxy: zui::app::AppProxy<NativeEvent>,
    pub(super) clipboard: ClipboardHandle,
    pub(super) cursor_position: Option<Point>,
    pub(super) command_registry: command_dispatch::NativeCommandRegistry,
    pub(super) terminal_pointer: TerminalPointer,
    pub(super) terminal_scroll: TerminalScroll,
    pub(super) terminal_selection: TerminalSelection,
    pub(super) keybindings: keybindings::NativeKeybindings,
    pub(super) keybindings_resource: KeybindingsResource,
    pub(super) keyboard_shortcuts: KeyboardShortcutsState,
    pub(super) language_server_settings: LanguageServerSettingsState,
    pub(super) settings_section: SettingsPageSection,
    pub(super) modifiers: ModifiersState,
    pub(super) pending_focus: Option<ElementId>,
    pub(super) physical_extent: PhysicalExtent,
    pub(super) scale_factor: f64,
    pub(super) failed: bool,
    pub(super) palette: ShellPalette,
    pub(super) theme_scheme: ColorScheme,
    pub(super) theme_follows_system: bool,
}

pub(super) struct TerminalPaneResize {
    pub(super) tab_key: TabInputKey,
    pub(super) split_id: PaneSplitId,
    pub(super) resizable: Resizable,
}

impl NativeApp {
    pub(super) fn new(application: ApplicationHandle<NativeEvent>, launch: ZetermLaunch) -> Self {
        let event_proxy = application.proxy();
        let clipboard = application.clipboard();
        let local_workspace_context = WorkspaceContext::capture_current();
        let app_server_host = launch.app_server_host(local_workspace_context.working_directory());
        let remote_tunnel_host = app_server_host
            .ssh_transport()
            .map(|(host, ssh_executable)| {
                NativeRemoteTunnelHost::new(host.clone(), ssh_executable.to_path_buf())
            });
        let workspace_context = if app_server_host.is_remote() {
            WorkspaceContext::capture_remote(app_server_host.workspace_root().to_path_buf())
        } else {
            local_workspace_context
        };
        let workspace_pane_host = WorkspacePaneHost::new(&workspace_context);
        let mut keybindings = keybindings::NativeKeybindings::default();
        let mut keybindings_resource = KeybindingsResource::new(
            local_profile_root().join("keybindings.json"),
            zeta_keybinding::HostPlatform::current(),
            Instant::now(),
        );
        if let KeybindingsResourcePoll::Rejected(error) =
            keybindings_resource.poll(Instant::now(), &mut keybindings)
        {
            eprintln!("{error}");
        }
        let composer = Composer::for_working_directory(workspace_context.working_directory());
        let language_service = if launch.is_remote() {
            language_service_host::NativeLanguageService::remote(
                workspace_context.working_directory(),
                event_proxy.clone(),
            )
        } else {
            language_service_host::NativeLanguageService::start(
                workspace_context.working_directory(),
                event_proxy.clone(),
            )
        };
        Self {
            window: None,
            presentation: None,
            frame_scheduler: FrameScheduler::default(),
            retained_runtime: RetainedRuntime::default(),
            inspector_part: InspectorPartState::default(),
            workspace_pane_host,
            file_editor_input: FileEditorInputState::default(),
            file_editor_search: file_editor_search::FileEditorSearchState::default(),
            language_service,
            file_editor_host: FileEditorHost::default(),
            tab_container: TabContainerState::default(),
            session_search: SessionSearch::default(),
            workbench_host: WorkbenchHost::new(event_proxy.clone(), app_server_host.clone()),
            pane_view_states: HashMap::new(),
            active_pane: None,
            terminal_pane_resize: None,
            session_context_menu: SessionContextMenuState::default(),
            git_branch_context_menu: GitBranchContextMenuState::default(),
            workspace_path_picker: WorkspacePathPickerState::default(),
            remote_connection_picker: RemoteConnectionPickerState::default(),
            remote_connection_manager: RemoteConnectionManagerState::default(),
            remote_connection_launch: None,
            remote_tunnel_manager: RemoteTunnelManagerState::default(),
            remote_tunnel_host,
            ui_dispatch: UiDispatch::default(),
            agent_session: None,
            app_server_host: app_server_host.clone(),
            thread_projection: ThreadProjection::default(),
            thread_timeline_scroll: ThreadTimelineScroll::default(),
            workspace_surface: WorkspaceSurface::default(),
            composer,
            workspace_context,
            text_layout: TextInputLayoutEngine::new(),
            caret_blink: CaretBlinkController::default(),
            code_editor_style: CodeEditorStyle::light(),
            event_proxy,
            clipboard,
            cursor_position: None,
            command_registry: command_dispatch::builtin_command_registry(),
            terminal_pointer: TerminalPointer::default(),
            terminal_scroll: TerminalScroll::default(),
            terminal_selection: TerminalSelection::default(),
            keybindings,
            keybindings_resource,
            keyboard_shortcuts: KeyboardShortcutsState::default(),
            language_server_settings: LanguageServerSettingsState::default(),
            settings_section: SettingsPageSection::default(),
            modifiers: ModifiersState::default(),
            pending_focus: None,
            physical_extent: PhysicalExtent::new(0, 0),
            scale_factor: 1.0,
            failed: false,
            palette: SHELL_PALETTE,
            theme_scheme: ColorScheme::Light,
            theme_follows_system: true,
        }
    }
}
