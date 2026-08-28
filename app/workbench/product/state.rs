use super::*;

pub(crate) struct ProductApp {
    pub(super) window: Option<WindowHandle>,
    pub(super) presentation: Option<WorkbenchPresentation>,
    pub(super) frame_scheduler: FrameScheduler,
    pub(super) retained_runtime: RetainedRuntime,
    pub(super) files: FilesState,
    pub(super) scm: ScmState,
    pub(super) file_editor_host: FileEditorHost,
    pub(super) file_editor_input: FileEditorInputState,
    pub(super) file_editor_search: FileEditorSearchState,
    pub(super) language_service: FileEditorLanguageService,
    pub(super) session_search: SessionSearchState,
    pub(super) workbench: WorkbenchHost<PaneBinding>,
    pub(super) terminal_workspace: TerminalWorkspace,
    pub(super) terminal_pane_views: TerminalPaneViews<PaneKey, TerminalPaneViewState>,
    pub(super) git_branch_context_menu: GitBranchContextMenuState,
    pub(super) workspace_path_picker: WorkspacePathPickerState,
    pub(super) remote_connection_picker: RemoteConnectionPickerState,
    pub(super) remote_connection_manager: RemoteConnectionManagerState,
    pub(super) remote_connection_launch: Option<remote_connection_process::RemoteConnectionLaunch>,
    pub(super) remote_tunnel_manager: RemoteTunnelManagerState,
    pub(super) remote_tunnel_host: Option<ProductRemoteTunnelHost>,
    pub(super) ui_dispatch: UiDispatch,
    pub(super) session_runtime: Option<SessionRuntime>,
    pub(super) app_server_client: Option<AppServerRequestHandle>,
    pub(super) app_server_host: AppServerHost,
    pub(super) session_pane: SessionPaneState,
    pub(super) workspace_surface: WorkspaceSurface,
    pub(super) workspace_context: WorkspaceContext,
    pub(super) text_layout: TextInputLayoutEngine,
    pub(super) caret_blink: CaretBlinkController,
    pub(super) code_editor_style: CodeEditorStyle,
    pub(super) event_proxy: zui::app::AppProxy<ProductEvent>,
    pub(super) clipboard: ClipboardHandle,
    pub(super) cursor_position: Option<Point>,
    pub(super) command_registry: command_dispatch::ProductCommandRegistry,
    pub(super) keybindings: keybindings::ProductKeybindings,
    pub(super) keybindings_resource: KeybindingsResource,
    pub(super) settings: SettingsState,
    pub(super) modifiers: ModifiersState,
    pub(super) pending_focus: Option<ElementId>,
    pub(super) physical_extent: PhysicalExtent,
    pub(super) scale_factor: f64,
    pub(super) failed: bool,
    pub(super) palette: UiTheme,
    pub(super) theme_scheme: ColorScheme,
    pub(super) theme_follows_system: bool,
}

impl ProductApp {
    pub(super) fn new(application: ApplicationHandle<ProductEvent>, launch: AppLaunch) -> Self {
        let event_proxy = application.proxy();
        let clipboard = application.clipboard();
        let local_workspace_context = WorkspaceContext::capture_current();
        let app_server_host = launch.app_server_host(local_workspace_context.working_directory());
        let remote_tunnel_host = app_server_host
            .ssh_transport()
            .map(|(host, ssh_executable)| {
                ProductRemoteTunnelHost::new(host.clone(), ssh_executable.to_path_buf())
            });
        let workspace_context = if app_server_host.is_remote() {
            WorkspaceContext::capture_remote(app_server_host.workspace_root().to_path_buf())
        } else {
            local_workspace_context
        };
        let mut files = FilesState::default();
        files.set_workspace_root(workspace_context.working_directory().to_path_buf());
        let mut scm = ScmState::default();
        scm.replace_diffs(
            workspace_context
                .diffs()
                .iter()
                .map(|diff| ScmDiff::new(diff.path(), diff.document().clone())),
        );
        let mut keybindings = keybindings::ProductKeybindings::default();
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
        let session_pane =
            SessionPaneState::for_working_directory(workspace_context.working_directory());
        let language_events = Arc::new(language_service_adapter::ProductLanguageEventSink::new(
            event_proxy.clone(),
        ));
        let language_service = if launch.is_remote() {
            FileEditorLanguageService::remote(
                workspace_context.working_directory(),
                language_events,
            )
        } else {
            FileEditorLanguageService::start(workspace_context.working_directory(), language_events)
        };
        Self {
            window: None,
            presentation: None,
            frame_scheduler: FrameScheduler::default(),
            retained_runtime: RetainedRuntime::default(),
            files,
            scm,
            file_editor_input: FileEditorInputState::default(),
            file_editor_search: FileEditorSearchState::default(),
            language_service,
            file_editor_host: FileEditorHost::default(),
            session_search: SessionSearchState::default(),
            workbench: WorkbenchHost::new(),
            terminal_workspace: {
                let terminal_event_proxy = event_proxy.clone();
                let terminal_target = app_server_host
                    .remote_connection()
                    .cloned()
                    .map(zeta_terminal_workspace::TerminalRuntimeTarget::remote)
                    .unwrap_or(zeta_terminal_workspace::TerminalRuntimeTarget::Local);
                TerminalWorkspace::new(
                    move |key, size| {
                        let event_proxy = terminal_event_proxy.clone();
                        let event_sink: zeta_terminal_workspace::TerminalEventSink =
                            Arc::new(move |event| event_proxy.send_event(event.into()).is_ok());
                        TerminalSession::spawn_async(
                            key,
                            size,
                            event_sink,
                            terminal_target.clone(),
                            PRODUCT_DISPLAY_NAME.to_owned(),
                        )
                    },
                    TerminalSession::resize,
                )
            },
            terminal_pane_views: TerminalPaneViews::default(),
            git_branch_context_menu: GitBranchContextMenuState::default(),
            workspace_path_picker: WorkspacePathPickerState::default(),
            remote_connection_picker: RemoteConnectionPickerState::default(),
            remote_connection_manager: RemoteConnectionManagerState::default(),
            remote_connection_launch: None,
            remote_tunnel_manager: RemoteTunnelManagerState::default(),
            remote_tunnel_host,
            ui_dispatch: UiDispatch::default(),
            session_runtime: None,
            app_server_client: None,
            app_server_host: app_server_host.clone(),
            session_pane,
            workspace_surface: WorkspaceSurface::default(),
            workspace_context,
            text_layout: TextInputLayoutEngine::new(),
            caret_blink: CaretBlinkController::default(),
            code_editor_style: CodeEditorStyle::light(),
            event_proxy,
            clipboard,
            cursor_position: None,
            command_registry: command_dispatch::builtin_command_registry(),
            keybindings,
            keybindings_resource,
            settings: SettingsState::default(),
            modifiers: ModifiersState::default(),
            pending_focus: None,
            physical_extent: PhysicalExtent::new(0, 0),
            scale_factor: 1.0,
            failed: false,
            palette: DEFAULT_UI_THEME,
            theme_scheme: ColorScheme::Light,
            theme_follows_system: true,
        }
    }
}
