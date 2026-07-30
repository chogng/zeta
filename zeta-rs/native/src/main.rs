use std::process::ExitCode;
use std::time::Instant;

use agent_sidebar::AgentSidebarState;
use agent_sidebar_workspace::AgentSidebarWorkspace;
use session_context_menu::SessionContextMenuState;
use session_search::SessionSearch;
use session_sidebar::SessionSidebarState;
use shell_interaction::{COMPOSER, ContextAction, SessionContextMenuAction};
use shell_scene::{
    LogicalViewport, ShellPresentation, ShellPresentationModel, build_shell_presentation,
    terminal_grid_size_for_viewport,
};
use terminal_pointer::TerminalPointer;
use terminal_scrollback::TerminalScroll;
use terminal_selection::TerminalSelection;
use terminal_session::{TerminalSession, TerminalSessionEvent};
use workspace_context::WorkspaceContext;
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

mod agent_sidebar;
mod agent_sidebar_layout;
mod agent_sidebar_workspace;
mod editor_pane;
mod explorer_pane;
mod input_context_toolbar;
mod input_method;
mod session_context_menu;
mod session_search;
mod session_sidebar;
mod session_sidebar_toolbar;
mod session_tab_list;
mod shell_interaction;
mod shell_scene;
mod shell_style;
mod terminal_blocks;
mod terminal_composer;
mod terminal_input;
mod terminal_pointer;
mod terminal_projection;
mod terminal_scrollback;
mod terminal_selection;
mod terminal_session;
mod terminal_workspace_layout;
mod titlebar;
mod workspace_context;

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
    agent_sidebar: AgentSidebarState,
    agent_sidebar_workspace: AgentSidebarWorkspace,
    session_sidebar: SessionSidebarState,
    session_search: SessionSearch,
    session_context_menu: SessionContextMenuState,
    ui_dispatch: UiDispatch,
    terminal: Option<TerminalSession>,
    workspace_context: WorkspaceContext,
    terminal_composer: terminal_composer::TerminalComposer,
    text_layout: TextInputLayoutEngine,
    caret_blink: CaretBlinkController,
    event_proxy: zeta_winit::EventLoopProxy<TerminalSessionEvent>,
    cursor_position: Option<Point>,
    terminal_pointer: TerminalPointer,
    terminal_scroll: TerminalScroll,
    terminal_selection: TerminalSelection,
    modifiers: ModifiersState,
    physical_extent: PhysicalExtent,
    scale_factor: f64,
    failed: bool,
}

impl NativeApp {
    fn new(event_proxy: zeta_winit::EventLoopProxy<TerminalSessionEvent>) -> Self {
        Self {
            window_id: None,
            window: None,
            renderer: None,
            presentation: None,
            agent_sidebar: AgentSidebarState::default(),
            agent_sidebar_workspace: AgentSidebarWorkspace::default(),
            session_sidebar: SessionSidebarState::default(),
            session_search: SessionSearch::default(),
            session_context_menu: SessionContextMenuState::default(),
            ui_dispatch: UiDispatch::default(),
            terminal: None,
            workspace_context: WorkspaceContext::capture_current(),
            terminal_composer: terminal_composer::TerminalComposer::default(),
            text_layout: TextInputLayoutEngine::new(),
            caret_blink: CaretBlinkController::default(),
            event_proxy,
            cursor_position: None,
            terminal_pointer: TerminalPointer::default(),
            terminal_scroll: TerminalScroll::default(),
            terminal_selection: TerminalSelection::default(),
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
                terminal_selection: self.terminal_selection.range(),
                workspace_context: &self.workspace_context,
                composer: self.terminal_composer.input(),
                session_search: &self.session_search,
                caret_visibility: self.caret_blink.visibility(),
                dispatch: &self.ui_dispatch,
                session_sidebar: self.session_sidebar,
                agent_sidebar: self.agent_sidebar,
                agent_sidebar_workspace: &self.agent_sidebar_workspace,
                session_context_menu: self.session_context_menu,
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
                    terminal_selection: self.terminal_selection.range(),
                    workspace_context: &self.workspace_context,
                    composer: self.terminal_composer.input(),
                    session_search: &self.session_search,
                    caret_visibility: self.caret_blink.visibility(),
                    dispatch: &self.ui_dispatch,
                    session_sidebar: self.session_sidebar,
                    agent_sidebar: self.agent_sidebar,
                    agent_sidebar_workspace: &self.agent_sidebar_workspace,
                    session_context_menu: self.session_context_menu,
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
                    terminal_selection: self.terminal_selection.range(),
                    workspace_context: &self.workspace_context,
                    composer: self.terminal_composer.input(),
                    session_search: &self.session_search,
                    caret_visibility: self.caret_blink.visibility(),
                    dispatch: &self.ui_dispatch,
                    session_sidebar: self.session_sidebar,
                    agent_sidebar: self.agent_sidebar,
                    agent_sidebar_workspace: &self.agent_sidebar_workspace,
                    session_context_menu: self.session_context_menu,
                    window_control_insets,
                },
                &mut self.text_layout,
            );
        }
        self.presentation = Some(presentation);
        self.update_ime_cursor_area();
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
        if id == shell_interaction::SESSION_SIDEBAR_TOGGLE {
            self.session_sidebar.toggle();
            return;
        }
        if id == shell_interaction::AGENT_SIDEBAR_TOGGLE {
            self.agent_sidebar.toggle();
            return;
        }
        if id == shell_interaction::ACTIVE_SESSION_TAB {
            return;
        }
        if id == shell_interaction::ADD_SESSION {
            // The action is intentionally routed at the product boundary now. Creating a second
            // tab requires the future multi-Session runtime to own distinct PTYs and active state.
            return;
        }
        if let Some(action) = SessionContextMenuAction::from_element_id(id) {
            let _target_session = self.session_context_menu.target_session();
            self.dismiss_session_context_menu();
            match action {
                SessionContextMenuAction::Pin
                | SessionContextMenuAction::Close
                | SessionContextMenuAction::Rename
                | SessionContextMenuAction::Fork => {
                    // The menu owns only command presentation. These transitions require the
                    // future multi-Session runtime rather than mutating the single PTY preview.
                }
            }
            return;
        }
        let Some(action) = ContextAction::from_element_id(id) else {
            return;
        };
        match action {
            ContextAction::Location
            | ContextAction::WorkingDirectory
            | ContextAction::GitBranch
            | ContextAction::Diff => {
                // Pickers are product commands layered above the dispatch foundation.
            }
        }
    }

    fn pointer_moved(&mut self, physical_x: f64, physical_y: f64) {
        let point = self.logical_pointer_position(physical_x, physical_y);
        self.cursor_position = Some(point);
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
    }

    fn mouse_button_changed(&mut self, state: ElementState, button: MouseButton) {
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
            self.rebuild_presentation();
            self.request_redraw();
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
            self.rebuild_presentation();
            self.request_redraw();
        }
        outcome.handled
    }
}

impl ApplicationHandler<TerminalSessionEvent> for NativeApp {
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
                self.terminal_pointer.cancel();
                self.cancel_session_sidebar_resize();
                self.agent_sidebar_workspace.cancel_multi_diff_scrollbar();
                self.session_context_menu.dismiss();
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

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: TerminalSessionEvent) {
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
        let caret_changed = matches!(
            self.caret_blink.advance(now),
            CaretBlinkAdvance::VisibilityChanged(_)
        );
        let scrollbar_changed = self
            .agent_sidebar_workspace
            .advance_multi_diff_scrollbar(now);
        if caret_changed || scrollbar_changed {
            self.rebuild_presentation();
            self.request_redraw();
        }
        let next_deadline = [
            self.caret_blink.next_deadline(),
            self.agent_sidebar_workspace.multi_diff_scrollbar_deadline(),
        ]
        .into_iter()
        .flatten()
        .min();
        let control_flow = match next_deadline {
            Some(deadline) => ControlFlow::WaitUntil(deadline),
            None => ControlFlow::Wait,
        };
        event_loop.set_control_flow(control_flow);
    }
}
