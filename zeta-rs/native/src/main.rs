use std::process::ExitCode;
use std::time::Instant;

use shell_interaction::{InteractionEffect, PointerFeedback, ShellInteraction};
use shell_scene::{
    LogicalViewport, ShellPresentation, build_shell_presentation, terminal_grid_size_for_viewport,
};
use terminal_pointer::TerminalPointer;
use terminal_scrollback::TerminalScroll;
use terminal_selection::TerminalSelection;
use terminal_session::{TerminalSession, TerminalSessionEvent};
use zeta_terminal::ScreenBuffer;
use zeta_ui::{CaretBlinkAdvance, CaretBlinkController, Point, TextInputLayoutEngine};
use zeta_wgpu::{RenderOutcome, WgpuRenderer};
use zeta_winit::{
    ActiveEventLoop, ApplicationHandler, ControlFlow, CursorIcon, ElementState, ImeCursorArea,
    LogicalSize, ModifiersState, MouseButton, NativeWindow, PhysicalExtent, WindowAttributes,
    WindowChrome, WindowEvent, WindowId, apply_window_chrome, run_application_with_user_events,
};

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
mod titlebar;

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
    interaction: ShellInteraction,
    terminal: Option<TerminalSession>,
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
            interaction: ShellInteraction::default(),
            terminal: None,
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
        let terminal_size = terminal_grid_size_for_viewport(viewport, active_screen);
        if let Some(terminal) = self.terminal.as_mut()
            && let Err(error) = terminal.resize(terminal_size)
        {
            eprintln!("could not resize terminal: {error}");
        }
        let scroll_limit = self.terminal_scroll_limit();
        self.terminal_scroll.clamp(scroll_limit);
        let mut presentation = build_shell_presentation(
            viewport,
            self.terminal.as_ref().map(TerminalSession::core),
            self.terminal_scroll.offset(),
            self.terminal_selection.range(),
            self.terminal_composer.input(),
            &mut self.text_layout,
            self.caret_blink.visibility(),
        );
        if let Some(point) = self.cursor_position
            && self.interaction.pointer_moved(point, &presentation.hit_map)
                == InteractionEffect::Redraw
        {
            presentation = build_shell_presentation(
                viewport,
                self.terminal.as_ref().map(TerminalSession::core),
                self.terminal_scroll.offset(),
                self.terminal_selection.range(),
                self.terminal_composer.input(),
                &mut self.text_layout,
                self.caret_blink.visibility(),
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
        let cursor = match self.interaction.pointer_feedback() {
            PointerFeedback::Default => CursorIcon::Default,
            PointerFeedback::Text => CursorIcon::Text,
        };
        if let Some(window) = self.window.as_ref() {
            window.set_cursor(cursor);
        }
    }

    fn update_ime_cursor_area(&self) {
        let Some(bounds) = self
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.ime_cursor_area)
        else {
            return;
        };
        if let Some(window) = self.window.as_ref() {
            window.set_ime_cursor_area(ImeCursorArea::new(
                bounds.origin.x as f64,
                bounds.origin.y as f64,
                bounds.size.width as f64,
                bounds.size.height as f64,
            ));
        }
    }

    fn apply_interaction_effect(&mut self, effect: InteractionEffect) {
        match effect {
            InteractionEffect::None => {}
            InteractionEffect::Redraw => {
                self.rebuild_presentation();
                self.request_redraw();
            }
            InteractionEffect::StartWindowDrag => {
                if let Some(window) = self.window.as_ref()
                    && let Err(error) = window.start_window_drag()
                {
                    eprintln!("could not begin native window drag: {error}");
                }
            }
        }
    }

    fn pointer_moved(&mut self, physical_x: f64, physical_y: f64) {
        let point = self.logical_pointer_position(physical_x, physical_y);
        self.cursor_position = Some(point);
        let terminal_position = self.terminal_mouse_position(point);
        let terminal_captured = self.route_terminal_pointer_move(terminal_position);
        if !terminal_captured && self.route_terminal_selection_move(terminal_position) {
            return;
        }
        let effect = self
            .presentation
            .as_ref()
            .map(|presentation| self.interaction.pointer_moved(point, &presentation.hit_map))
            .unwrap_or(InteractionEffect::None);
        self.update_cursor();
        self.apply_interaction_effect(effect);
    }

    fn pointer_left(&mut self) {
        self.cursor_position = None;
        let effect = self.interaction.pointer_left();
        self.update_cursor();
        self.apply_interaction_effect(effect);
    }

    fn primary_button_changed(&mut self, state: ElementState) {
        let effect = match state {
            ElementState::Pressed => self.interaction.press_primary(),
            ElementState::Released => self.interaction.release_primary(),
        };
        self.apply_interaction_effect(effect);
    }

    fn mouse_button_changed(&mut self, state: ElementState, button: MouseButton) {
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
}

impl ApplicationHandler<TerminalSessionEvent> for NativeApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            self.request_redraw();
            return;
        }

        let attributes = apply_window_chrome(
            WindowAttributes::default()
                .with_title(PRODUCT_DISPLAY_NAME)
                .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT)),
            WindowChrome::ContentUnderTitlebar,
        );
        let window = match NativeWindow::create(event_loop, attributes) {
            Ok(window) => window,
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
        self.window_id = Some(window.id());
        self.physical_extent = window.inner_extent();
        self.scale_factor = window.scale_factor();
        let terminal_size =
            terminal_grid_size_for_viewport(self.logical_viewport(), ScreenBuffer::Primary);
        self.terminal = match TerminalSession::spawn(terminal_size, self.event_proxy.clone()) {
            Ok(terminal) => Some(terminal),
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
        self.caret_blink.focus(Instant::now());
        self.rebuild_presentation();
        window.enable_ime();
        self.window = Some(window.clone());
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
                self.terminal_composer.cancel_composition();
                self.caret_blink.blur();
                if let Some(window) = self.window.as_ref() {
                    window.disable_ime();
                }
                self.rebuild_presentation();
                self.request_redraw();
            }
            WindowEvent::Focused(true) => {
                if self.active_screen() == ScreenBuffer::Primary {
                    self.caret_blink.focus(Instant::now());
                }
                if let Some(window) = self.window.as_ref() {
                    window.enable_ime();
                }
                self.rebuild_presentation();
                self.update_ime_cursor_area();
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
        let previous_screen = self.active_screen();
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
        if active_screen == ScreenBuffer::Alternate || self.terminal_scroll.offset() == 0 {
            self.terminal_selection.clear();
        }
        let scroll_limit = self.terminal_scroll_limit();
        self.terminal_scroll.preserve_view_after_growth(
            scroll_limit.saturating_sub(previous_scroll_limit),
            scroll_limit,
        );
        if active_screen != previous_screen {
            if active_screen == ScreenBuffer::Primary {
                self.caret_blink.focus(Instant::now());
            } else {
                self.terminal_composer.cancel_composition();
                self.caret_blink.blur();
            }
        }
        self.rebuild_presentation();
        self.request_redraw();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let CaretBlinkAdvance::VisibilityChanged(_) = self.caret_blink.advance(Instant::now()) {
            self.rebuild_presentation();
            self.request_redraw();
        }
        let control_flow = match self.caret_blink.next_deadline() {
            Some(deadline) => ControlFlow::WaitUntil(deadline),
            None => ControlFlow::Wait,
        };
        event_loop.set_control_flow(control_flow);
    }
}
