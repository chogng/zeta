use std::process::ExitCode;
use std::time::Instant;

use shell_interaction::{InteractionEffect, PointerFeedback, ShellInteraction};
use shell_scene::{LogicalViewport, ShellPresentation, build_shell_presentation};
use zeta_ui::{
    CaretBlinkAdvance, CaretBlinkController, Point, TextInputCommand, TextInputCompositionCursor,
    TextInputCompositionEvent, TextInputLayoutEngine, TextInputSelectionMode,
};
use zeta_wgpu::{RenderOutcome, WgpuRenderer};
use zeta_winit::{
    ActiveEventLoop, ApplicationHandler, ControlFlow, CursorIcon, ElementState, Ime, ImeCursorArea,
    Key, KeyEvent, LogicalSize, ModifiersState, MouseButton, NamedKey, NativeWindow,
    PhysicalExtent, WindowAttributes, WindowChrome, WindowEvent, WindowId, apply_window_chrome,
    run_application,
};

mod shell_interaction;
mod shell_scene;
mod shell_style;

const WINDOW_TITLE: &str = "Zeta Native";
const INITIAL_WIDTH: f64 = 1_000.0;
const INITIAL_HEIGHT: f64 = 700.0;

fn main() -> ExitCode {
    let mut application = NativeApp::new();
    if let Err(error) = run_application(&mut application) {
        eprintln!("failed to run the native event loop: {error}");
        return ExitCode::FAILURE;
    }
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
    text_layout: TextInputLayoutEngine,
    caret_blink: CaretBlinkController,
    cursor_position: Option<Point>,
    modifiers: ModifiersState,
    physical_extent: PhysicalExtent,
    scale_factor: f64,
    failed: bool,
}

impl NativeApp {
    fn new() -> Self {
        Self {
            window_id: None,
            window: None,
            renderer: None,
            presentation: None,
            interaction: ShellInteraction::default(),
            text_layout: TextInputLayoutEngine::new(),
            caret_blink: CaretBlinkController::default(),
            cursor_position: None,
            modifiers: ModifiersState::default(),
            physical_extent: PhysicalExtent::new(0, 0),
            scale_factor: 1.0,
            failed: false,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: impl std::fmt::Display) {
        eprintln!("zeta-native failed: {message}");
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

    fn rebuild_presentation(&mut self) {
        let viewport = self.logical_viewport();
        let mut presentation = build_shell_presentation(
            viewport,
            &self.interaction,
            &mut self.text_layout,
            self.caret_blink.visibility(),
        );
        if let Some(point) = self.cursor_position
            && self.interaction.pointer_moved(point, &presentation.hit_map)
                == InteractionEffect::Redraw
        {
            presentation = build_shell_presentation(
                viewport,
                &self.interaction,
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
            PointerFeedback::Clickable => CursorIcon::Pointer,
            PointerFeedback::Text => CursorIcon::Text,
        };
        if let Some(window) = self.window.as_ref() {
            window.set_cursor(cursor);
        }
    }

    fn update_ime_cursor_area(&self) {
        if !self.interaction.composer_focused() {
            return;
        }
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
            InteractionEffect::FocusComposer => {
                self.caret_blink.focus(Instant::now());
                if let Some(window) = self.window.as_ref() {
                    window.enable_ime();
                }
                self.rebuild_presentation();
                self.request_redraw();
            }
            InteractionEffect::BlurComposer => {
                self.caret_blink.blur();
                if let Some(window) = self.window.as_ref() {
                    window.disable_ime();
                }
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

    fn keyboard_input(&mut self, event: KeyEvent) {
        if event.state != ElementState::Pressed || !self.interaction.composer_focused() {
            return;
        }
        if event.logical_key == Key::Named(NamedKey::Escape) {
            self.caret_blink.activity(Instant::now());
            let effect = self
                .interaction
                .update_composition(TextInputCompositionEvent::Cancel);
            self.apply_interaction_effect(effect);
            return;
        }
        let selection_mode = if self.modifiers.shift_key() {
            TextInputSelectionMode::Extend
        } else {
            TextInputSelectionMode::Move
        };
        let shortcut = self.modifiers.control_key() || self.modifiers.super_key();
        let command = match &event.logical_key {
            Key::Named(NamedKey::Backspace) => Some(TextInputCommand::Backspace),
            Key::Named(NamedKey::Delete) => Some(TextInputCommand::DeleteForward),
            Key::Named(NamedKey::ArrowLeft) => Some(TextInputCommand::MoveLeft(selection_mode)),
            Key::Named(NamedKey::ArrowRight) => Some(TextInputCommand::MoveRight(selection_mode)),
            Key::Named(NamedKey::Home) => Some(TextInputCommand::MoveToStart(selection_mode)),
            Key::Named(NamedKey::End) => Some(TextInputCommand::MoveToEnd(selection_mode)),
            Key::Character(text) if shortcut && text.eq_ignore_ascii_case("a") => {
                Some(TextInputCommand::SelectAll)
            }
            _ if !shortcut => event
                .text
                .as_ref()
                .map(|text| TextInputCommand::Insert(text.to_string())),
            _ => None,
        };
        if let Some(command) = command {
            self.caret_blink.activity(Instant::now());
            let effect = self.interaction.edit_composer(command);
            self.apply_interaction_effect(effect);
        }
    }

    fn ime_input(&mut self, event: Ime) {
        let event = match event {
            Ime::Enabled => {
                self.update_ime_cursor_area();
                return;
            }
            Ime::Preedit(text, Some((start, end))) => TextInputCompositionEvent::Preedit {
                text,
                cursor: TextInputCompositionCursor::Visible(start..end),
            },
            Ime::Preedit(text, None) => TextInputCompositionEvent::Preedit {
                text,
                cursor: TextInputCompositionCursor::Hidden,
            },
            Ime::Commit(text) => TextInputCompositionEvent::Commit(text),
            Ime::Disabled => TextInputCompositionEvent::Cancel,
        };
        self.caret_blink.activity(Instant::now());
        let effect = self.interaction.update_composition(event);
        self.apply_interaction_effect(effect);
    }
}

impl ApplicationHandler for NativeApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            self.request_redraw();
            return;
        }

        let attributes = apply_window_chrome(
            WindowAttributes::default()
                .with_title(WINDOW_TITLE)
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
        self.rebuild_presentation();
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
                self.physical_extent = PhysicalExtent::new(size.width, size.height);
                self.rebuild_presentation();
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(self.physical_extent);
                }
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
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
                let effect = self.interaction.window_focus_lost();
                self.apply_interaction_effect(effect);
            }
            WindowEvent::Focused(true) => {}
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                self.primary_button_changed(state);
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
