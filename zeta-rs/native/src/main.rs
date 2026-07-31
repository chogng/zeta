use std::process::ExitCode;
use std::time::Instant;

use agent_composer::AgentComposer;
use agent_session::AgentSession;
use agent_sidebar::AgentSidebarState;
use agent_sidebar_workspace::AgentSidebarWorkspace;
use git_branch_context_menu::GitBranchContextMenuState;
use keybindings_resource::{KeybindingsResource, KeybindingsResourcePoll};
use keyboard_shortcuts::KeyboardShortcutsState;
use native_event::NativeEvent;
use session_context_menu::SessionContextMenuState;
use session_search::SessionSearch;
use session_sidebar::SessionSidebarState;
use shell_interaction::COMPOSER;
use shell_scene::{
    LogicalViewport, ShellPresentation, ShellPresentationModel, build_shell_presentation,
    terminal_grid_size_for_viewport,
};
use terminal_pointer::TerminalPointer;
use terminal_scrollback::TerminalScroll;
use terminal_selection::TerminalSelection;
use terminal_session::TerminalSession;
use thread_projection::ThreadProjection;
use thread_timeline_scroll::ThreadTimelineScroll;
use workspace_context::WorkspaceContext;
use workspace_path_picker::WorkspacePathPickerState;
use workspace_surface::WorkspaceSurface;
use zeta_terminal::{BlockStatus, ScreenBuffer};
use zeta_ui::{CaretBlinkAdvance, CaretBlinkController, Point, TextInputLayoutEngine};
use zeta_ui_dispatch::{
    CursorFeedback, DispatchInvalidation, DispatchOutcome, UiDispatch, UiIntent,
};
use zeta_wgpu::{RenderOutcome, WgpuRenderer};
use zeta_winit::{
    ActiveEventLoop, ApplicationHandler, ControlFlow, CursorIcon, ElementState, LogicalSize,
    ModifiersState, MouseButton, NativeWindow, PhysicalExtent, Theme, WindowAttributes,
    WindowChrome, WindowControlInsets, WindowEvent, WindowId, run_application_with_user_events,
};

mod agent_composer;
mod agent_session;
mod agent_sidebar;
mod agent_sidebar_layout;
mod agent_sidebar_navigation;
mod agent_sidebar_toolbar;
mod agent_sidebar_workspace;
mod commands;
mod composer_editor;
mod editor_pane;
mod explorer_pane;
mod explorer_tree;
mod git_branch_context_menu;
mod git_branch_context_menu_input;
mod input_context_toolbar;
mod input_method;
mod keybinding_input;
mod keybindings;
mod keybindings_resource;
mod keyboard_shortcuts;
mod native_event;
mod session_context_menu;
mod session_search;
mod session_sidebar;
mod session_sidebar_toolbar;
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
mod terminal_workspace_layout;
mod thread_projection;
mod thread_timeline;
mod thread_timeline_scroll;
mod titlebar;
mod workspace_context;
mod workspace_path_picker;
mod workspace_path_picker_input;
mod workspace_surface;

pub(crate) const PRODUCT_DISPLAY_NAME: &str = "zeterm";
const INITIAL_WIDTH: f64 = 1_000.0;
const INITIAL_HEIGHT: f64 = 700.0;

fn main() -> ExitCode {
    let application = match run_application_with_user_events(NativeApp::new) {
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
    renderer: Option<WgpuRenderer>,
    presentation: Option<ShellPresentation>,
    presentation_rebuild_pending: bool,
    agent_sidebar: AgentSidebarState,
    agent_sidebar_workspace: AgentSidebarWorkspace,
    session_sidebar: SessionSidebarState,
    session_search: SessionSearch,
    session_context_menu: SessionContextMenuState,
    git_branch_context_menu: GitBranchContextMenuState,
    workspace_path_picker: WorkspacePathPickerState,
    ui_dispatch: UiDispatch,
    agent_session: Option<AgentSession>,
    thread_projection: ThreadProjection,
    thread_timeline_scroll: ThreadTimelineScroll,
    workspace_surface: WorkspaceSurface,
    terminal: Option<TerminalSession>,
    workspace_context: WorkspaceContext,
    composer: AgentComposer,
    text_layout: TextInputLayoutEngine,
    caret_blink: CaretBlinkController,
    event_proxy: zeta_winit::EventLoopProxy<NativeEvent>,
    cursor_position: Option<Point>,
    terminal_pointer: TerminalPointer,
    terminal_scroll: TerminalScroll,
    terminal_selection: TerminalSelection,
    keybindings: keybindings::NativeKeybindings,
    keybindings_resource: KeybindingsResource,
    keyboard_shortcuts: KeyboardShortcutsState,
    modifiers: ModifiersState,
    physical_extent: PhysicalExtent,
    scale_factor: f64,
    failed: bool,
}

impl NativeApp {
    fn new(event_proxy: zeta_winit::EventLoopProxy<NativeEvent>) -> Self {
        let workspace_context = WorkspaceContext::capture_current();
        let agent_sidebar_workspace = AgentSidebarWorkspace::new(&workspace_context);
        let mut keybindings = keybindings::NativeKeybindings::default();
        let mut keybindings_resource = KeybindingsResource::for_workspace(
            workspace_context.working_directory(),
            zeta_keybinding::HostPlatform::current(),
        );
        if let KeybindingsResourcePoll::Rejected(error) =
            keybindings_resource.poll(Instant::now(), &mut keybindings)
        {
            eprintln!("{error}");
        }
        Self {
            window_id: None,
            window: None,
            renderer: None,
            presentation: None,
            presentation_rebuild_pending: false,
            agent_sidebar: AgentSidebarState::default(),
            agent_sidebar_workspace,
            session_sidebar: SessionSidebarState::default(),
            session_search: SessionSearch::default(),
            session_context_menu: SessionContextMenuState::default(),
            git_branch_context_menu: GitBranchContextMenuState::default(),
            workspace_path_picker: WorkspacePathPickerState::default(),
            ui_dispatch: UiDispatch::default(),
            agent_session: None,
            thread_projection: ThreadProjection::default(),
            thread_timeline_scroll: ThreadTimelineScroll::default(),
            workspace_surface: WorkspaceSurface::default(),
            terminal: None,
            workspace_context,
            composer: AgentComposer::default(),
            text_layout: TextInputLayoutEngine::new(),
            caret_blink: CaretBlinkController::default(),
            event_proxy,
            cursor_position: None,
            terminal_pointer: TerminalPointer::default(),
            terminal_scroll: TerminalScroll::default(),
            terminal_selection: TerminalSelection::default(),
            keybindings,
            keybindings_resource,
            keyboard_shortcuts: KeyboardShortcutsState::default(),
            modifiers: ModifiersState::default(),
            physical_extent: PhysicalExtent::new(0, 0),
            scale_factor: 1.0,
            failed: false,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: impl std::fmt::Display) {
        eprintln!("{PRODUCT_DISPLAY_NAME} failed: {message}");
        self.failed = true;
        event_loop.exit();
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        if self.presentation_rebuild_pending {
            self.rebuild_presentation();
        }
        let Some(presentation) = self.presentation.as_ref() else {
            return;
        };
        debug_assert!(!presentation.accessibility_nodes.is_empty());
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        match renderer.render_scene(&presentation.scene) {
            Ok(RenderOutcome::Presented | RenderOutcome::Skipped) => {}
            Ok(RenderOutcome::Retry) => self.request_redraw(),
            Err(error) => self.fail(event_loop, error),
        }
    }

    fn logical_viewport(&self) -> LogicalViewport {
        LogicalViewport::from_physical(
            self.physical_extent.width,
            self.physical_extent.height,
            self.scale_factor,
        )
    }

    fn active_screen(&self) -> ScreenBuffer {
        if self.workspace_surface.is_terminal() {
            return ScreenBuffer::Alternate;
        }
        self.terminal
            .as_ref()
            .map(|terminal| terminal.core().active_screen())
            .unwrap_or_default()
    }

    fn rebuild_presentation(&mut self) {
        let viewport = self.logical_viewport();
        let active_screen = self.active_screen();
        let terminal_size = terminal_grid_size_for_viewport(
            viewport,
            active_screen,
            self.session_sidebar,
            self.agent_sidebar,
        );
        if let Some(terminal) = self.terminal.as_mut()
            && terminal.core().grid().size() != terminal_size
            && let Err(error) = terminal.resize(terminal_size)
        {
            eprintln!("could not resize terminal: {error}");
        }
        let scroll_limit = self.terminal_scroll_limit();
        self.terminal_scroll.clamp(scroll_limit);
        let window_control_insets = self
            .window
            .as_ref()
            .map(NativeWindow::window_control_insets)
            .unwrap_or(WindowControlInsets::NONE);
        let mut presentation = build_shell_presentation(
            viewport,
            ShellPresentationModel {
                terminal: self.terminal.as_ref().map(TerminalSession::core),
                terminal_scroll_offset: self.terminal_scroll.offset(),
                terminal_scrollbar_presentation: self.terminal_scroll.scrollbar_presentation(),
                terminal_selection: self.terminal_selection.range(),
                terminal_surface: self.workspace_surface.is_terminal(),
                thread_projection: &self.thread_projection,
                thread_timeline_scroll_offset: self.thread_timeline_scroll.offset(),
                workspace_context: &self.workspace_context,
                composer: self.composer.editor(),
                composer_mode: self.composer.mode(),
                session_search: &self.session_search,
                caret_visibility: self.caret_blink.visibility(),
                dispatch: &self.ui_dispatch,
                session_sidebar: self.session_sidebar,
                agent_sidebar: self.agent_sidebar,
                agent_sidebar_workspace: &self.agent_sidebar_workspace,
                session_context_menu: self.session_context_menu,
                git_branch_context_menu: &self.git_branch_context_menu,
                workspace_path_picker: &self.workspace_path_picker,
                keybindings: &self.keybindings,
                keyboard_shortcuts: &self.keyboard_shortcuts,
                keybinding_diagnostics: self.keybindings_resource.diagnostics(),
                window_control_insets,
            },
            &mut self.text_layout,
        );
        let focus_outcome = self
            .ui_dispatch
            .reconcile_focus(&presentation.interaction_frame, COMPOSER);
        if focus_outcome.invalidation == DispatchInvalidation::Paint {
            presentation = build_shell_presentation(
                viewport,
                ShellPresentationModel {
                    terminal: self.terminal.as_ref().map(TerminalSession::core),
                    terminal_scroll_offset: self.terminal_scroll.offset(),
                    terminal_scrollbar_presentation: self.terminal_scroll.scrollbar_presentation(),
                    terminal_selection: self.terminal_selection.range(),
                    terminal_surface: self.workspace_surface.is_terminal(),
                    thread_projection: &self.thread_projection,
                    thread_timeline_scroll_offset: self.thread_timeline_scroll.offset(),
                    workspace_context: &self.workspace_context,
                    composer: self.composer.editor(),
                    composer_mode: self.composer.mode(),
                    session_search: &self.session_search,
                    caret_visibility: self.caret_blink.visibility(),
                    dispatch: &self.ui_dispatch,
                    session_sidebar: self.session_sidebar,
                    agent_sidebar: self.agent_sidebar,
                    agent_sidebar_workspace: &self.agent_sidebar_workspace,
                    session_context_menu: self.session_context_menu,
                    git_branch_context_menu: &self.git_branch_context_menu,
                    workspace_path_picker: &self.workspace_path_picker,
                    keybindings: &self.keybindings,
                    keyboard_shortcuts: &self.keyboard_shortcuts,
                    keybinding_diagnostics: self.keybindings_resource.diagnostics(),
                    window_control_insets,
                },
                &mut self.text_layout,
            );
        }
        if let Some(point) = self.cursor_position
            && self
                .ui_dispatch
                .pointer_moved(point, &presentation.interaction_frame)
                .invalidation
                == DispatchInvalidation::Paint
        {
            presentation = build_shell_presentation(
                viewport,
                ShellPresentationModel {
                    terminal: self.terminal.as_ref().map(TerminalSession::core),
                    terminal_scroll_offset: self.terminal_scroll.offset(),
                    terminal_scrollbar_presentation: self.terminal_scroll.scrollbar_presentation(),
                    terminal_selection: self.terminal_selection.range(),
                    terminal_surface: self.workspace_surface.is_terminal(),
                    thread_projection: &self.thread_projection,
                    thread_timeline_scroll_offset: self.thread_timeline_scroll.offset(),
                    workspace_context: &self.workspace_context,
                    composer: self.composer.editor(),
                    composer_mode: self.composer.mode(),
                    session_search: &self.session_search,
                    caret_visibility: self.caret_blink.visibility(),
                    dispatch: &self.ui_dispatch,
                    session_sidebar: self.session_sidebar,
                    agent_sidebar: self.agent_sidebar,
                    agent_sidebar_workspace: &self.agent_sidebar_workspace,
                    session_context_menu: self.session_context_menu,
                    git_branch_context_menu: &self.git_branch_context_menu,
                    workspace_path_picker: &self.workspace_path_picker,
                    keybindings: &self.keybindings,
                    keyboard_shortcuts: &self.keyboard_shortcuts,
                    keybinding_diagnostics: self.keybindings_resource.diagnostics(),
                    window_control_insets,
                },
                &mut self.text_layout,
            );
        }
        self.presentation = Some(presentation);
        self.presentation_rebuild_pending = false;
        self.update_ime_cursor_area();
    }

    fn rebuild_presentation_on_next_redraw(&mut self) {
        self.presentation_rebuild_pending = true;
        self.request_redraw();
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
                    .pointer_feedback(&presentation.interaction_frame)
            })
            .unwrap_or_default();
        let cursor = if self.session_sidebar.is_resizing() {
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
        if let Some(intent) = outcome.intent {
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
        if outcome.invalidation == DispatchInvalidation::Paint {
            self.sync_input_focus();
            self.rebuild_presentation();
            self.request_redraw();
        }
    }

    fn activate_shell_element(&mut self, id: zeta_ui_dispatch::ElementId) {
        if self.agent_sidebar_workspace.activate_file_tree_element(id) {
            return;
        }
        if self.agent_sidebar_workspace.toggle_multi_diff_fold(id) {
            return;
        }
        if self.activate_keyboard_shortcuts_element(id) {
            return;
        }
        if self.activate_git_branch_context_menu_element(id) {
            return;
        }
        if self.activate_workspace_path_picker_element(id) {
            return;
        }
        if let Some(command) = commands::command_for_element(id) {
            self.execute_native_command(command);
        }
    }

    fn pointer_moved(&mut self, physical_x: f64, physical_y: f64) {
        let point = self.logical_pointer_position(physical_x, physical_y);
        self.cursor_position = Some(point);
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
                    .pointer_moved(point, &presentation.interaction_frame)
            })
            .unwrap_or_default();
        self.update_cursor();
        self.apply_dispatch_outcome(outcome);
    }

    fn pointer_left(&mut self) {
        self.cursor_position = None;
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
                (presentation.interaction_frame.target_at(point) == Some(COMPOSER))
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
                .press_primary(&presentation.interaction_frame),
            ElementState::Released => {
                let point = self.cursor_position.unwrap_or(Point::new(-1.0, -1.0));
                self.ui_dispatch
                    .release_primary(point, &presentation.interaction_frame)
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

impl ApplicationHandler<NativeEvent> for NativeApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            self.request_redraw();
            return;
        }

        let attributes = WindowAttributes::default()
            .with_title(PRODUCT_DISPLAY_NAME)
            .with_theme(Some(Theme::Light))
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
        self.window_id = Some(window.id());
        self.physical_extent = window.inner_extent();
        self.scale_factor = window.scale_factor();
        let terminal_size = terminal_grid_size_for_viewport(
            self.logical_viewport(),
            ScreenBuffer::Primary,
            self.session_sidebar,
            self.agent_sidebar,
        );
        self.terminal = match TerminalSession::spawn(terminal_size, self.event_proxy.clone()) {
            Ok(terminal) => Some(terminal),
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
        self.agent_session = match AgentSession::spawn(
            self.event_proxy.clone(),
            self.workspace_context.working_directory().to_path_buf(),
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
        let renderer = match WgpuRenderer::initialize(window) {
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
                self.rebuild_presentation();
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(self.physical_extent);
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
                self.cancel_session_sidebar_resize();
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
            WindowEvent::MouseWheel { delta, .. } => self.mouse_wheel(delta),
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
        let event = match event {
            NativeEvent::Agent(event) => {
                self.handle_agent_session_event(event);
                return;
            }
            NativeEvent::Terminal(event) => event,
        };
        let previous_scroll_limit = self.terminal_scroll_limit();
        let previous_block_status = self
            .terminal
            .as_ref()
            .and_then(|terminal| terminal.core().block_list().blocks().last())
            .map(|block| block.status());
        let active_screen = if let Some(terminal) = self.terminal.as_mut() {
            if let Err(error) = terminal.handle_event(event) {
                eprintln!("could not reply to terminal query: {error}");
            }
            if let Some(window) = self.window.as_ref() {
                window.set_title(terminal.core().title().unwrap_or(PRODUCT_DISPLAY_NAME));
            }
            terminal.core().active_screen()
        } else {
            return;
        };
        let current_block_status = self
            .terminal
            .as_ref()
            .and_then(|terminal| terminal.core().block_list().blocks().last())
            .map(|block| block.status());
        if previous_block_status == Some(BlockStatus::Running)
            && current_block_status != Some(BlockStatus::Running)
        {
            self.workspace_context.refresh_repository();
            self.agent_sidebar_workspace
                .sync_repository(&self.workspace_context);
            self.agent_sidebar_workspace.refresh_files();
        }
        if active_screen == ScreenBuffer::Alternate || self.terminal_scroll.offset() == 0 {
            self.terminal_selection.clear();
        }
        let scroll_limit = self.terminal_scroll_limit();
        self.terminal_scroll.preserve_view_after_growth(
            scroll_limit.saturating_sub(previous_scroll_limit),
            scroll_limit,
        );
        self.sync_input_focus();
        self.rebuild_presentation();
        self.request_redraw();
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
        let file_search_changed = self.agent_sidebar_workspace.poll_file_search();
        if caret_changed || scrollbar_changed || terminal_scrollbar_changed || file_search_changed {
            self.rebuild_presentation();
            self.request_redraw();
        }
        let mut next_deadline = [
            self.caret_blink.next_deadline(),
            self.agent_sidebar_workspace.multi_diff_scrollbar_deadline(),
            self.terminal_scroll.scrollbar_deadline(),
            self.keybindings.chord_deadline(),
            self.keyboard_shortcuts_deadline(),
            Some(self.keybindings_resource.next_deadline()),
        ]
        .into_iter()
        .flatten()
        .min();
        if self.agent_sidebar_workspace.file_search_pending() {
            let search_deadline = now + std::time::Duration::from_millis(50);
            next_deadline = Some(
                next_deadline
                    .map(|deadline| deadline.min(search_deadline))
                    .unwrap_or(search_deadline),
            );
        }
        let control_flow = match next_deadline {
            Some(deadline) => ControlFlow::WaitUntil(deadline),
            None => ControlFlow::Wait,
        };
        event_loop.set_control_flow(control_flow);
    }
}
