use std::process::ExitCode;
use std::time::Instant;

use agent_composer::AgentComposer;
use agent_session::AgentSession;
use agent_session_target::AgentSessionTarget;
use agent_sidebar::AgentSidebarState;
use agent_sidebar_workspace::AgentSidebarWorkspace;
use file_editor_host::FileEditorHost;
use file_editor_input::FileEditorInputState;
use git_branch_context_menu::GitBranchContextMenuState;
use keybindings::{KeybindingsResource, KeybindingsResourcePoll};
use keyboard_shortcuts::KeyboardShortcutsState;
use language_server_settings::LanguageServerSettingsState;
use launch::ZetermLaunch;
use layout_inspector::LayoutInspector;
use native_event::NativeEvent;
use remote_connection_cli::ZetermInvocation;
use remote_connection_manager::RemoteConnectionManagerState;
use remote_connection_picker::RemoteConnectionPickerState;
use remote_tunnel_manager::RemoteTunnelManagerState;
use remote_tunnel_process::RemoteTunnelHost;
use session_context_menu::SessionContextMenuState;
use session_search::SessionSearch;
use session_sidebar::SessionSidebarState;
use session_tab_list::SessionTabState;
use shell_interaction::{COMPOSER, FILE_EDITOR_DOCUMENT};
use shell_scene::{
    ShellPresentation, ShellPresentationModel, build_shell_presentation_with_animation_bindings,
    rebuild_shell_fragment, rebuild_shell_overlays, terminal_grid_size_for_viewport,
};
use shell_style::{SHELL_PALETTE, ShellPalette, code_editor_style};
use terminal_pointer::TerminalPointer;
use terminal_scrollback::TerminalScroll;
use terminal_selection::TerminalSelection;
use terminal_session::{TerminalSession, TerminalSessionEvent, TerminalSessionKey};
use terminal_workspace::{TerminalReadyOutcome, TerminalWorkspace};
use thread_projection::ThreadProjection;
use thread_timeline_scroll::ThreadTimelineScroll;
use workspace_context::WorkspaceContext;
use workspace_path_picker::WorkspacePathPickerState;
use workspace_surface::WorkspaceSurface;
use zeta_agent_sidebar::AgentSidebarAction;
use zeta_editor::CodeEditorStyle;
use zeta_layout::{InspectorPane, LogicalViewport, RootLayout};
use zeta_protocol::SessionId;
use zeta_renderer::{RenderOutcome, RenderTargetSize, Renderer};
use zeta_terminal::{BlockStatus, GridSize, ScreenBuffer};
use zeta_theme::{ColorScheme, ThemeLoadOptions, ThemeLoader, ThemeSurface, default_device_root};
use zeta_ui::{CaretBlinkAdvance, CaretBlinkController, Point, TextInputLayoutEngine};
use zeta_winit::{
    ActiveEventLoop, ApplicationHandler, ControlFlow, CursorIcon, ElementState, LogicalSize,
    ModifiersState, MouseButton, NativeWindow, PhysicalExtent, Theme, WindowAttributes,
    WindowChrome, WindowControlInsets, WindowEvent, WindowId, run_application_with_user_events,
};
use zui::FrameDeadlineSet;
use zui::{CursorFeedback, DispatchInvalidation, DispatchOutcome, ElementId, UiDispatch, UiIntent};
use zui::{FrameInvalidation, FrameSchedule, FrameScheduler, RetainedRuntime};

mod agent_composer;
mod agent_session;
mod agent_session_target;
#[cfg(test)]
#[path = "agent_session_target_tests.rs"]
mod agent_session_target_tests;
mod agent_sidebar;
mod agent_sidebar_workspace;
mod command_dispatch;
#[cfg(test)]
#[path = "component_composition_tests.rs"]
mod component_composition_tests;
mod composer_classification;
mod composer_editor;
mod composer_interaction;
mod composer_interaction_pane;
mod composer_panel;
mod file_editor_auto_scroll;
mod file_editor_diagnostics;
mod file_editor_host;
mod file_editor_input;
mod file_editor_language_features;
mod file_editor_pane;
mod file_editor_search;
mod git_branch_context_menu;
mod git_branch_context_menu_input;
mod input_context_toolbar;
mod input_method;
mod keybindings;
mod keyboard_shortcuts;
mod language_server_settings;
mod language_server_settings_input;
mod language_service_host;
mod launch;
#[cfg(test)]
#[path = "launch_profile_tests.rs"]
mod launch_profile_tests;
mod launch_progress;
#[cfg(test)]
#[path = "launch_progress_tests.rs"]
mod launch_progress_tests;
#[cfg(test)]
#[path = "launch_test_support.rs"]
mod launch_test_support;
#[cfg(test)]
#[path = "launch_tests.rs"]
mod launch_tests;
mod layout_inspector;
mod native_event;
mod remote_connection_cli;
#[cfg(test)]
#[path = "remote_connection_cli_tests.rs"]
mod remote_connection_cli_tests;
mod remote_connection_launch_input;
mod remote_connection_manager;
mod remote_connection_manager_input;
mod remote_connection_manager_view;
mod remote_connection_picker;
mod remote_connection_picker_input;
mod remote_connection_process;
mod remote_connection_tunnel;
#[cfg(test)]
#[path = "remote_connection_tunnel_tests.rs"]
mod remote_connection_tunnel_tests;
mod remote_tunnel_manager;
mod remote_tunnel_manager_input;
mod remote_tunnel_manager_view;
mod remote_tunnel_process;
mod remote_tunnel_readiness;
mod renderer_backend;
mod session_context_menu;
mod session_search;
mod session_sidebar;
mod session_sidebar_toolbar;
mod session_switch_trace;
mod session_tab_list;
mod shell_interaction;
mod shell_scene;
mod shell_style;
mod terminal_blocks;
mod terminal_input;
mod terminal_output_scroll_view;
mod terminal_pointer;
mod terminal_projection;
mod terminal_scrollback;
mod terminal_selection;
mod terminal_session;
mod terminal_workspace;
mod thread_projection;
mod thread_timeline;
mod thread_timeline_scroll;
mod titlebar;
mod workspace_context;
mod workspace_path_picker;
mod workspace_path_picker_input;
mod workspace_surface;

pub(crate) const PRODUCT_DISPLAY_NAME: &str = "zeterm";
const DEFAULT_THEME_ENTRY: &str = "zeterm";
const INITIAL_WIDTH: f64 = 1_000.0;
const INITIAL_HEIGHT: f64 = 700.0;

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|command| command == "app-server")
    {
        return match zeta_server_host::run_app_server(arguments.into_iter().skip(1)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("zeterm App Server host: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let invocation = match ZetermInvocation::parse(arguments) {
        Ok(invocation) => invocation,
        Err(error) => {
            eprintln!("{error}");
            return if error.is_help_requested() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
    };
    let mut launch = match invocation.resolve() {
        Ok(Some(launch)) => launch,
        Ok(None) => return ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = launch_progress::prepare_remote_launch(&mut launch) {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    let application = match run_application_with_user_events(move |event_proxy| {
        NativeApp::new(event_proxy, launch)
    }) {
        Ok(application) => application,
        Err(error) => {
            eprintln!("failed to run the native event loop: {error}");
            return ExitCode::FAILURE;
        }
    };
    if application.failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

struct NativeApp {
    window_id: Option<WindowId>,
    window: Option<NativeWindow>,
    renderer: Option<Box<dyn Renderer>>,
    presentation: Option<ShellPresentation>,
    frame_scheduler: FrameScheduler,
    retained_runtime: RetainedRuntime,
    agent_sidebar: AgentSidebarState,
    agent_sidebar_workspace: AgentSidebarWorkspace,
    file_editor_host: FileEditorHost,
    file_editor_input: FileEditorInputState,
    file_editor_search: file_editor_search::FileEditorSearchState,
    language_service: language_service_host::NativeLanguageService,
    session_sidebar: SessionSidebarState,
    session_search: SessionSearch,
    session_tabs: Vec<SessionTabState>,
    selected_session_tab: ElementId,
    session_context_menu: SessionContextMenuState,
    git_branch_context_menu: GitBranchContextMenuState,
    workspace_path_picker: WorkspacePathPickerState,
    remote_connection_picker: RemoteConnectionPickerState,
    remote_connection_manager: RemoteConnectionManagerState,
    remote_connection_launch: Option<remote_connection_process::RemoteConnectionLaunch>,
    remote_tunnel_manager: RemoteTunnelManagerState,
    remote_tunnel_host: Option<RemoteTunnelHost>,
    ui_dispatch: UiDispatch,
    agent_session: Option<AgentSession>,
    agent_session_target: AgentSessionTarget,
    thread_projection: ThreadProjection,
    thread_timeline_scroll: ThreadTimelineScroll,
    workspace_surface: WorkspaceSurface,
    terminal_workspace: TerminalWorkspace,
    workspace_context: WorkspaceContext,
    composer: AgentComposer,
    composer_interaction: composer_interaction::ComposerInteractionModel,
    composer_interaction_pane: composer_interaction_pane::ComposerInteractionPaneState,
    text_layout: TextInputLayoutEngine,
    caret_blink: CaretBlinkController,
    code_editor_style: CodeEditorStyle,
    event_proxy: zeta_winit::EventLoopProxy<NativeEvent>,
    cursor_position: Option<Point>,
    command_registry: command_dispatch::NativeCommandRegistry,
    terminal_pointer: TerminalPointer,
    terminal_scroll: TerminalScroll,
    terminal_selection: TerminalSelection,
    keybindings: keybindings::NativeKeybindings,
    keybindings_resource: KeybindingsResource,
    keyboard_shortcuts: KeyboardShortcutsState,
    language_server_settings: LanguageServerSettingsState,
    layout_inspector: LayoutInspector,
    modifiers: ModifiersState,
    pending_focus: Option<ElementId>,
    physical_extent: PhysicalExtent,
    scale_factor: f64,
    failed: bool,
    palette: ShellPalette,
    theme_scheme: ColorScheme,
    theme_follows_system: bool,
}

impl NativeApp {
    fn new(event_proxy: zeta_winit::EventLoopProxy<NativeEvent>, launch: ZetermLaunch) -> Self {
        let local_workspace_context = WorkspaceContext::capture_current();
        let agent_session_target =
            launch.agent_session_target(local_workspace_context.working_directory());
        let remote_tunnel_host =
            agent_session_target
                .ssh_transport()
                .map(|(host, ssh_executable)| {
                    RemoteTunnelHost::new(host.clone(), ssh_executable.to_path_buf())
                });
        let workspace_context = if agent_session_target.is_remote() {
            WorkspaceContext::capture_remote(agent_session_target.workspace_root().to_path_buf())
        } else {
            local_workspace_context
        };
        let agent_sidebar_workspace = AgentSidebarWorkspace::new(&workspace_context);
        let mut keybindings = keybindings::NativeKeybindings::default();
        let mut keybindings_resource = KeybindingsResource::new(
            zeta_app_server_client::local_profile_root().join("keybindings.json"),
            zeta_keybinding::HostPlatform::current(),
            Instant::now(),
        );
        if let KeybindingsResourcePoll::Rejected(error) =
            keybindings_resource.poll(Instant::now(), &mut keybindings)
        {
            eprintln!("{error}");
        }
        let composer = AgentComposer::for_working_directory(workspace_context.working_directory());
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
            window_id: None,
            window: None,
            renderer: None,
            presentation: None,
            frame_scheduler: FrameScheduler::default(),
            retained_runtime: RetainedRuntime::default(),
            agent_sidebar: AgentSidebarState::default(),
            agent_sidebar_workspace,
            file_editor_input: FileEditorInputState::default(),
            file_editor_search: file_editor_search::FileEditorSearchState::default(),
            language_service,
            file_editor_host: FileEditorHost::default(),
            session_sidebar: SessionSidebarState::default(),
            session_search: SessionSearch::default(),
            session_tabs: Vec::new(),
            selected_session_tab: shell_interaction::ACTIVE_SESSION_TAB,
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
            agent_session_target: agent_session_target.clone(),
            thread_projection: ThreadProjection::default(),
            thread_timeline_scroll: ThreadTimelineScroll::default(),
            workspace_surface: WorkspaceSurface::default(),
            terminal_workspace: TerminalWorkspace::new(event_proxy.clone(), agent_session_target),
            composer,
            workspace_context,
            composer_interaction: composer_interaction::ComposerInteractionModel::new(),
            composer_interaction_pane:
                composer_interaction_pane::ComposerInteractionPaneState::default(),
            text_layout: TextInputLayoutEngine::new(),
            caret_blink: CaretBlinkController::default(),
            code_editor_style: CodeEditorStyle::light(),
            event_proxy,
            cursor_position: None,
            command_registry: command_dispatch::builtin_command_registry(),
            terminal_pointer: TerminalPointer::default(),
            terminal_scroll: TerminalScroll::default(),
            terminal_selection: TerminalSelection::default(),
            keybindings,
            keybindings_resource,
            keyboard_shortcuts: KeyboardShortcutsState::default(),
            language_server_settings: LanguageServerSettingsState::default(),
            layout_inspector: LayoutInspector::default(),
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

    fn reload_theme(&mut self, system_scheme: ColorScheme) {
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
        self.composer.set_editor_style(editor_style.clone());
        self.code_editor_style = editor_style;
        self.agent_sidebar_workspace
            .set_editor_style(palette.multi_diff_editor_style());
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: impl std::fmt::Display) {
        eprintln!("{PRODUCT_DISPLAY_NAME} failed: {message}");
        self.failed = true;
        event_loop.exit();
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
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
                if !retained_cleanup_failed {
                    presentation.accessibility_nodes = presentation
                        .interaction_frame()
                        .accessibility_nodes(&self.ui_dispatch);
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
        debug_assert!(!presentation.accessibility_nodes.is_empty());
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let _render_trace = session_switch_trace::Span::frame("renderer.render_scene");
        match renderer.render_scene(presentation.scene()) {
            Ok(RenderOutcome::Presented | RenderOutcome::Skipped) => {}
            Ok(RenderOutcome::Retry) => self.request_redraw(),
            Err(error) => self.fail(event_loop, error),
        }
    }

    fn window_viewport(&self) -> LogicalViewport {
        LogicalViewport::from_physical(
            self.physical_extent.width,
            self.physical_extent.height,
            self.scale_factor,
        )
    }

    fn logical_viewport(&self) -> LogicalViewport {
        self.layout_inspector
            .content_viewport(self.window_viewport())
    }

    fn active_screen(&self) -> ScreenBuffer {
        if self.workspace_surface.is_terminal() {
            ScreenBuffer::Alternate
        } else {
            ScreenBuffer::Primary
        }
    }

    fn terminal_size(&self) -> GridSize {
        terminal_grid_size_for_viewport(
            self.logical_viewport(),
            self.active_screen(),
            self.session_sidebar,
            self.agent_sidebar,
        )
    }

    pub(crate) fn ensure_terminal_for_session(&mut self, session_id: &SessionId) -> bool {
        match self
            .terminal_workspace
            .ensure_for_session(session_id, self.terminal_size())
        {
            Ok(()) => true,
            Err(error) => {
                eprintln!("could not start terminal for session: {error}");
                false
            }
        }
    }

    pub(crate) fn activate_terminal_for_session(&mut self, session_id: &SessionId) -> bool {
        if !self.terminal_workspace.activate_for_session(session_id) {
            return false;
        }
        self.terminal_scroll.reset();
        self.terminal_selection.clear();
        self.terminal_pointer.cancel();
        if let Some(window) = self.window.as_ref()
            && let Some(terminal) = self.active_terminal()
        {
            window.set_title(terminal.core().title().unwrap_or(PRODUCT_DISPLAY_NAME));
        }
        true
    }

    fn update_terminal_status(&mut self, key: TerminalSessionKey, status: &str) {
        let Some(session_id) = self.terminal_workspace.session_id_for_key(key) else {
            return;
        };
        if let Some(tab) = self
            .session_tabs
            .iter_mut()
            .find(|tab| tab.session_id() == &session_id)
        {
            tab.update_status(status);
        }
    }

    pub(crate) fn active_terminal(&self) -> Option<&TerminalSession> {
        self.terminal_workspace.active()
    }

    pub(crate) fn active_terminal_mut(&mut self) -> Option<&mut TerminalSession> {
        self.terminal_workspace.active_mut()
    }

    fn rebuild_presentation(&mut self) {
        let _trace = session_switch_trace::Span::new(None, "rebuild_presentation");
        let window_viewport = self.window_viewport();
        let viewport = self.logical_viewport();
        let root_layout = RootLayout::for_viewports(
            window_viewport,
            viewport,
            if self.layout_inspector.is_enabled() {
                InspectorPane::visible(layout_inspector::PANEL_WIDTH)
            } else {
                InspectorPane::Hidden
            },
        );
        let active_screen = self.active_screen();
        let terminal_size = terminal_grid_size_for_viewport(
            viewport,
            active_screen,
            self.session_sidebar,
            self.agent_sidebar,
        );
        self.terminal_workspace.resize_all(terminal_size);
        let scroll_limit = self.terminal_scroll_limit();
        self.terminal_scroll.clamp(scroll_limit);
        let window_control_insets = self
            .window
            .as_ref()
            .map(NativeWindow::window_control_insets)
            .unwrap_or(WindowControlInsets::NONE);
        let mut presentation = with_shell_presentation_model(
            self,
            window_control_insets,
            |model, text_layout, animation_bindings| {
                build_shell_presentation_with_animation_bindings(
                    viewport,
                    model,
                    text_layout,
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
            presentation = with_shell_presentation_model(
                self,
                window_control_insets,
                |model, text_layout, animation_bindings| {
                    build_shell_presentation_with_animation_bindings(
                        viewport,
                        model,
                        text_layout,
                        animation_bindings,
                    )
                },
            );
        }
        if let Some(point) = self.cursor_position
            && self
                .ui_dispatch
                .pointer_moved(point, presentation.interaction_frame())
                .invalidation
                != DispatchInvalidation::None
        {
            presentation = with_shell_presentation_model(
                self,
                window_control_insets,
                |model, text_layout, animation_bindings| {
                    build_shell_presentation_with_animation_bindings(
                        viewport,
                        model,
                        text_layout,
                        animation_bindings,
                    )
                },
            );
        }
        self.layout_inspector
            .compose(presentation.scene_mut(), root_layout, self.cursor_position);
        self.mount_shell_fragments(&mut presentation);
        self.presentation = Some(presentation);
        self.frame_scheduler.clear();
        if requested_focus.is_some() {
            self.sync_input_focus();
        }
        self.update_ime_cursor_area();
    }

    fn rebuild_overlay_presentation(&mut self) {
        let window_viewport = self.window_viewport();
        let viewport = self.logical_viewport();
        let root_layout = RootLayout::for_viewports(
            window_viewport,
            viewport,
            if self.layout_inspector.is_enabled() {
                InspectorPane::visible(layout_inspector::PANEL_WIDTH)
            } else {
                InspectorPane::Hidden
            },
        );
        let window_control_insets = self
            .window
            .as_ref()
            .map(NativeWindow::window_control_insets)
            .unwrap_or(WindowControlInsets::NONE);
        let Some(mut presentation) = self.presentation.take() else {
            self.rebuild_presentation();
            return;
        };
        let rebuilt = with_shell_presentation_model(
            self,
            window_control_insets,
            |model, text_layout, animation_bindings| {
                rebuild_shell_overlays(
                    &mut presentation,
                    viewport,
                    model,
                    text_layout,
                    Some(animation_bindings),
                )
            },
        );
        if !rebuilt {
            self.presentation = Some(presentation);
            self.rebuild_presentation();
            return;
        }
        self.layout_inspector
            .compose(presentation.scene_mut(), root_layout, self.cursor_position);
        self.mount_shell_fragments(&mut presentation);
        self.presentation = Some(presentation);
        self.frame_scheduler.clear();
        self.update_ime_cursor_area();
    }

    fn rebuild_presentation_on_next_redraw(&mut self) {
        if self.frame_scheduler.request(FrameInvalidation::Rebuild) == FrameSchedule::RequestFrame {
            self.request_redraw();
        }
    }

    fn rebuild_overlay_on_next_redraw(&mut self) {
        if self.frame_scheduler.request(FrameInvalidation::Fragment) == FrameSchedule::RequestFrame
        {
            self.request_redraw();
        }
    }

    fn rebuild_fragment_on_next_redraw(&mut self, id: ElementId) {
        if self.frame_scheduler.request_fragment(id) == FrameSchedule::RequestFrame {
            self.request_redraw();
        }
    }

    fn mount_shell_fragments(&mut self, presentation: &mut ShellPresentation) {
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

    fn rebuild_shell_fragments(&mut self, ids: Vec<ElementId>) {
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

    fn logical_pointer_position(&self, physical_x: f64, physical_y: f64) -> Point {
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

    fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn update_cursor(&self) {
        let feedback = self
            .presentation
            .as_ref()
            .map(|presentation| {
                self.ui_dispatch
                    .pointer_feedback(presentation.interaction_frame())
            })
            .unwrap_or_default();
        let cursor = if self
            .layout_inspector
            .uses_inspection_cursor(self.cursor_position)
        {
            CursorIcon::Crosshair
        } else if self
            .layout_inspector
            .uses_panel_action_cursor(self.cursor_position)
        {
            CursorIcon::Pointer
        } else if self
            .layout_inspector
            .pointer_is_over_panel(self.cursor_position)
        {
            CursorIcon::Default
        } else if self.session_sidebar.is_resizing() || self.agent_sidebar.is_resizing() {
            CursorIcon::ColResize
        } else {
            match feedback {
                CursorFeedback::Default => CursorIcon::Default,
                CursorFeedback::Text => CursorIcon::Text,
                CursorFeedback::Pointer => CursorIcon::Pointer,
                CursorFeedback::ResizeHorizontal => CursorIcon::ColResize,
            }
        };
        if let Some(window) = self.window.as_ref() {
            window.set_cursor(cursor);
        }
    }

    fn apply_dispatch_outcome(&mut self, outcome: DispatchOutcome) {
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
    }

    fn activate_shell_element(&mut self, id: zui::ElementId) {
        if self.activate_language_server_settings_element(id) {
            return;
        }
        if self.activate_file_editor_element(id) {
            return;
        }
        let interaction_item_count = self
            .composer_interaction
            .view()
            .map(|view| view.items().len())
            .unwrap_or(0);
        if let Some(index) =
            shell_interaction::composer_interaction_item_index(id, 0..interaction_item_count)
        {
            self.activate_composer_interaction_item(index);
            return;
        }
        if let Some(action) = self.agent_sidebar_workspace.activate_file_tree_element(id) {
            match action {
                AgentSidebarAction::OpenFile { path } => self.open_workspace_file(path),
                AgentSidebarAction::LoadChildren { element, path } => {
                    self.load_file_tree_directory(element, path);
                }
                AgentSidebarAction::Handled
                | AgentSidebarAction::StateChanged
                | AgentSidebarAction::Focus(_) => {}
            }
            return;
        }
        if self.agent_sidebar_workspace.toggle_multi_diff_fold(id) {
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
        if let Some(index) = shell_interaction::session_tab_index(id, 0..self.session_tabs.len()) {
            session_switch_trace::event(
                None,
                "session-tab-hit",
                format_args!(
                    "element={id:?} index={index} tab_count={}",
                    self.session_tabs.len()
                ),
            );
            self.activate_session_tab(index);
            return;
        }
        if let Some(request) = command_dispatch::command_request_for_element(id) {
            self.dispatch_command(request);
        }
    }

    fn pointer_moved(&mut self, physical_x: f64, physical_y: f64) {
        let point = self.logical_pointer_position(physical_x, physical_y);
        self.cursor_position = Some(point);
        if self.route_layout_inspector_pointer_move() {
            return;
        }
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
        if self.route_session_sidebar_resize_move(point) {
            return;
        }
        if self.route_agent_sidebar_resize_move(point) {
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

    fn pointer_left(&mut self) {
        self.cursor_position = None;
        self.file_editor_input.cancel_pointer();
        if self.route_layout_inspector_pointer_left() {
            return;
        }
        if self
            .agent_sidebar_workspace
            .leave_multi_diff_scrollbar(Instant::now())
        {
            self.rebuild_presentation();
            self.request_redraw();
        }
        let outcome = self.ui_dispatch.pointer_left();
        self.update_cursor();
        self.apply_dispatch_outcome(outcome);
    }

    fn primary_button_changed(&mut self, state: ElementState) {
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
                    .accessibility_nodes
                    .iter()
                    .find(|node| node.id == COMPOSER)
                    .map(|node| (point, node.bounds))
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

    fn mouse_button_changed(&mut self, state: ElementState, button: MouseButton) {
        if self.route_layout_inspector_button(state, button) {
            return;
        }
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
        if button == MouseButton::Left && self.route_session_sidebar_resize_button(state) {
            return;
        }
        if button == MouseButton::Left && self.route_agent_sidebar_resize_button(state) {
            return;
        }
        if button == MouseButton::Left && self.route_multi_diff_scrollbar_button(state) {
            return;
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

    fn multi_diff_bounds(&self) -> Option<zeta_ui::Rect> {
        self.presentation
            .as_ref()?
            .accessibility_nodes
            .iter()
            .find(|node| node.id == shell_interaction::MULTI_DIFF_EDITOR)
            .map(|node| node.bounds)
    }

    fn route_multi_diff_scrollbar_move(&mut self, point: Point) -> bool {
        let Some(bounds) = self.multi_diff_bounds() else {
            return false;
        };
        let outcome =
            self.agent_sidebar_workspace
                .move_multi_diff_scrollbar(point, bounds, Instant::now());
        if outcome.presentation_changed {
            self.rebuild_presentation_on_next_redraw();
        }
        outcome.handled
    }

    fn route_multi_diff_scrollbar_button(&mut self, state: ElementState) -> bool {
        let Some(bounds) = self.multi_diff_bounds() else {
            return false;
        };
        let point = self.cursor_position.unwrap_or(Point::new(-1.0, -1.0));
        let now = Instant::now();
        let outcome = match state {
            ElementState::Pressed => self
                .agent_sidebar_workspace
                .press_multi_diff_scrollbar(point, bounds, now),
            ElementState::Released => self
                .agent_sidebar_workspace
                .release_multi_diff_scrollbar(point, bounds, now),
        };
        if outcome.presentation_changed {
            self.rebuild_presentation_on_next_redraw();
        }
        outcome.handled
    }
}

fn with_shell_presentation_model<R>(
    app: &mut NativeApp,
    window_control_insets: WindowControlInsets,
    operation: impl FnOnce(
        ShellPresentationModel<'_>,
        &mut TextInputLayoutEngine,
        &mut dyn zui::AnimationBinding,
    ) -> R,
) -> R {
    let NativeApp {
        palette,
        retained_runtime,
        terminal_workspace,
        terminal_scroll,
        terminal_selection,
        workspace_surface,
        thread_projection,
        thread_timeline_scroll,
        workspace_context,
        composer,
        composer_interaction,
        composer_interaction_pane,
        session_search,
        session_tabs,
        selected_session_tab,
        caret_blink,
        ui_dispatch,
        session_sidebar,
        agent_sidebar,
        agent_sidebar_workspace,
        file_editor_host,
        file_editor_input,
        file_editor_search,
        language_service,
        code_editor_style,
        session_context_menu,
        git_branch_context_menu,
        workspace_path_picker,
        remote_connection_picker,
        remote_connection_manager,
        remote_tunnel_manager,
        keybindings,
        keyboard_shortcuts,
        language_server_settings,
        cursor_position,
        keybindings_resource,
        text_layout,
        ..
    } = app;
    let file_editor_diagnostics = language_service.active_editor_diagnostics(file_editor_host);
    let language_hover = language_service.active_hover(file_editor_host);
    let language_completions = language_service.active_completions(file_editor_host);
    let language_server_runtime_state =
        language_service.server_state(language_server_settings.selected_server().server_id());
    operation(
        ShellPresentationModel {
            palette: *palette,
            terminal: terminal_workspace.active().map(TerminalSession::core),
            terminal_scroll_offset: terminal_scroll.offset(),
            terminal_scrollbar_presentation: terminal_scroll.scrollbar_presentation(),
            terminal_selection: terminal_selection.range(),
            workspace_surface: workspace_surface.active(),
            file_editor_host,
            file_editor_prompt: file_editor_input.prompt(),
            file_editor_search,
            file_editor_diagnostics,
            language_hover,
            language_completions,
            completion_selection: file_editor_input.completion_selection(),
            code_editor_style,
            thread_projection,
            thread_timeline_scroll_offset: thread_timeline_scroll.offset(),
            workspace_context,
            composer: composer.editor(),
            composer_interaction,
            composer_interaction_pane,
            composer_mode: composer.mode(),
            session_search,
            session_tabs,
            selected_session_tab: *selected_session_tab,
            caret_visibility: caret_blink.visibility(),
            dispatch: ui_dispatch,
            session_sidebar: *session_sidebar,
            agent_sidebar: *agent_sidebar,
            agent_sidebar_workspace,
            session_context_menu: *session_context_menu,
            git_branch_context_menu,
            workspace_path_picker,
            remote_connection_picker,
            remote_connection_manager,
            remote_tunnel_manager,
            keybindings,
            keyboard_shortcuts,
            language_server_settings,
            language_server_runtime_state,
            keybinding_diagnostics: keybindings_resource.diagnostics(),
            window_control_insets,
            pointer_position: *cursor_position,
        },
        text_layout,
        retained_runtime.animation_registry_mut(),
    )
}

fn handle_terminal_event(
    app: &mut NativeApp,
    key: TerminalSessionKey,
    event: TerminalSessionEvent,
) {
    let terminal_exited = matches!(&event, TerminalSessionEvent::Exited(_));
    if app.terminal_workspace.is_pending(key) {
        app.terminal_workspace.buffer_event_if_pending(key, event);
        session_switch_trace::event(None, "terminal-event-buffered", format_args!("key={key:?}"));
        return;
    }
    if app.terminal_workspace.active_key() != Some(key) {
        {
            let Some(terminal) = app.terminal_workspace.inactive_mut(key) else {
                return;
            };
            if let Err(error) = terminal.handle_event(event) {
                eprintln!("could not reply to inactive terminal query: {error}");
            }
        }
        if terminal_exited {
            app.update_terminal_status(key, "Exited");
            app.rebuild_presentation_on_next_redraw();
        }
        return;
    }

    let previous_scroll_limit = app.terminal_scroll_limit();
    let previous_block_status = app
        .active_terminal()
        .and_then(|terminal| terminal.core().block_list().blocks().last())
        .map(|block| block.status());
    let (active_screen, title) = if let Some(terminal) = app.active_terminal_mut() {
        if let Err(error) = terminal.handle_event(event) {
            eprintln!("could not reply to terminal query: {error}");
        }
        (
            terminal.core().active_screen(),
            terminal
                .core()
                .title()
                .unwrap_or(PRODUCT_DISPLAY_NAME)
                .to_owned(),
        )
    } else {
        return;
    };
    if let Some(window) = app.window.as_ref() {
        window.set_title(&title);
    }
    if terminal_exited {
        app.update_terminal_status(key, "Exited");
    }
    let current_block_status = app
        .active_terminal()
        .and_then(|terminal| terminal.core().block_list().blocks().last())
        .map(|block| block.status());
    if previous_block_status == Some(BlockStatus::Running)
        && current_block_status != Some(BlockStatus::Running)
    {
        if let Some(session) = app.agent_session.as_ref()
            && let Err(error) = session.refresh_git()
        {
            eprintln!("could not refresh Git projection: {error}");
        }
        app.refresh_files_from_app_server();
    }
    if active_screen == ScreenBuffer::Alternate || app.terminal_scroll.offset() == 0 {
        app.terminal_selection.clear();
    }
    let scroll_limit = app.terminal_scroll_limit();
    app.terminal_scroll.preserve_view_after_growth(
        scroll_limit.saturating_sub(previous_scroll_limit),
        scroll_limit,
    );
    app.sync_input_focus();
    app.rebuild_presentation_on_next_redraw();
}

impl ApplicationHandler<NativeEvent> for NativeApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            self.request_redraw();
            return;
        }

        let attributes = WindowAttributes::default()
            .with_title(PRODUCT_DISPLAY_NAME)
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT));
        let window = match NativeWindow::create(
            event_loop,
            attributes,
            WindowChrome::ContentUnderTitlebar,
        ) {
            Ok(window) => window,
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
        let system_scheme = match window.theme() {
            Some(Theme::Dark) => ColorScheme::Dark,
            Some(Theme::Light) | None => ColorScheme::Light,
        };
        self.reload_theme(system_scheme);
        window.set_theme(
            (!self.theme_follows_system).then_some(match self.theme_scheme {
                ColorScheme::Dark | ColorScheme::HighContrastDark => Theme::Dark,
                ColorScheme::Light | ColorScheme::HighContrastLight => Theme::Light,
            }),
        );
        self.window_id = Some(window.id());
        self.physical_extent = window.inner_extent();
        self.scale_factor = window.scale_factor();
        let terminal_size = terminal_grid_size_for_viewport(
            self.logical_viewport(),
            ScreenBuffer::Primary,
            self.session_sidebar,
            self.agent_sidebar,
        );
        if let Err(error) = self.terminal_workspace.spawn_initial(terminal_size) {
            self.fail(event_loop, error);
            return;
        }
        if self.agent_session_target.is_remote()
            && let Err(error) = self
                .language_service
                .start_remote(self.event_proxy.clone(), self.agent_session_target.clone())
        {
            self.fail(event_loop, error);
            return;
        }
        self.agent_session = match AgentSession::spawn(
            self.event_proxy.clone(),
            self.agent_session_target.clone(),
        ) {
            Ok(session) => Some(session),
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
        self.window = Some(window.clone());
        self.rebuild_presentation();
        self.sync_input_focus();
        let renderer = match renderer_backend::create(window) {
            Ok(renderer) => renderer,
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
        self.renderer = Some(renderer);
        self.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window_id != Some(window_id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.terminal_selection.clear();
                self.physical_extent = PhysicalExtent::new(size.width, size.height);
                self.layout_inspector.window_resized(self.window_viewport());
                self.rebuild_presentation();
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(RenderTargetSize::new(
                        self.physical_extent.width,
                        self.physical_extent.height,
                    ));
                }
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.terminal_selection.clear();
                self.scale_factor = scale_factor;
                self.rebuild_presentation();
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.set_scale_factor(scale_factor);
                }
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
            WindowEvent::Ime(event) => {
                if !self.layout_inspector.is_picking() {
                    self.ime_input(event);
                }
            }
            WindowEvent::Focused(false) => {
                self.modifiers = ModifiersState::default();
                self.keybindings.cancel_chord();
                self.keyboard_shortcuts.window_blurred();
                self.terminal_pointer.cancel();
                self.file_editor_input.cancel_pointer();
                self.cancel_session_sidebar_resize();
                self.cancel_agent_sidebar_resize();
                self.agent_sidebar_workspace.cancel_multi_diff_scrollbar();
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
            WindowEvent::MouseWheel { delta, .. } => {
                if !self.layout_inspector.is_picking()
                    && !self
                        .layout_inspector
                        .pointer_is_over_panel(self.cursor_position)
                {
                    self.mouse_wheel(delta);
                }
            }
            WindowEvent::Occluded(false) => {
                // macOS can reject initial surface acquisition while the new window activates.
                // The visible transition is the next reliable opportunity to present that frame.
                self.request_redraw();
            }
            WindowEvent::Occluded(true) => {}
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: NativeEvent) {
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
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
            .agent_sidebar_workspace
            .advance_multi_diff_scrollbar(now);
        let terminal_scrollbar_changed = self.terminal_scroll.advance_scrollbar(now);
        let retained_runtime_due = self
            .retained_runtime
            .next_deadline()
            .is_some_and(|deadline| deadline <= now);
        let file_search_changed = self.agent_sidebar_workspace.poll_file_search();
        let file_editor_auto_scrolled = self.advance_file_editor_auto_scroll(now);
        if caret_changed
            || scrollbar_changed
            || terminal_scrollbar_changed
            || file_search_changed
            || file_editor_auto_scrolled
        {
            self.rebuild_presentation_on_next_redraw();
        } else if retained_runtime_due {
            self.request_redraw();
        }
        let mut deadlines = FrameDeadlineSet::default();
        for deadline in [
            self.caret_blink.next_deadline(),
            self.agent_sidebar_workspace.multi_diff_scrollbar_deadline(),
            self.terminal_scroll.scrollbar_deadline(),
            self.retained_runtime.next_deadline(),
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
        if self.agent_sidebar_workspace.file_search_pending() {
            deadlines.include(now + std::time::Duration::from_millis(50));
        }
        let control_flow = match deadlines.next_deadline() {
            Some(deadline) => ControlFlow::WaitUntil(deadline),
            None => ControlFlow::Wait,
        };
        event_loop.set_control_flow(control_flow);
    }
}
