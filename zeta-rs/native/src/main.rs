use std::process::ExitCode;

use shell_interaction::{InteractionEffect, PointerFeedback, ShellInteraction};
use shell_scene::{LogicalViewport, ShellPresentation, build_shell_presentation};
use zeta_ui::Point;
use zeta_wgpu::{RenderOutcome, WgpuRenderer};
use zeta_winit::{
    ActiveEventLoop, ApplicationHandler, CursorIcon, ElementState, LogicalSize, MouseButton,
    NativeWindow, PhysicalExtent, WindowAttributes, WindowChrome, WindowEvent, WindowId,
    apply_window_chrome, run_application,
};

mod shell_interaction;
mod shell_scene;
mod shell_theme;

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
    cursor_position: Option<Point>,
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
            cursor_position: None,
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
        let mut presentation = build_shell_presentation(viewport, &self.interaction);
        if let Some(point) = self.cursor_position
            && self.interaction.pointer_moved(point, &presentation.hit_map)
                == InteractionEffect::Redraw
        {
            presentation = build_shell_presentation(viewport, &self.interaction);
        }
        self.presentation = Some(presentation);
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
}
