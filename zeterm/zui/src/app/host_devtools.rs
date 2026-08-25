use crate::devtools::DevToolsRequest;
use crate::input::ElementState;
use crate::input::Key;
use crate::input::NamedKey;
use crate::internal::ActiveEventLoop;
use crate::ui::Point;
use crate::ui::Rect;
use crate::window::MouseButton;
use crate::window::PhysicalPosition;
use crate::window::WindowChrome;
use crate::window::WindowEvent;
use crate::window::WindowId;
use crate::window::WindowOptions;
use crate::window::WindowRole;

use super::ApplicationError;
use super::ApplicationHost;
use super::WindowCommand;
use super::WindowRuntime;

impl<T, A> ApplicationHost<T, A>
where
    T: Send + 'static,
{
    pub(super) fn process_devtools_requests(&mut self, event_loop: &ActiveEventLoop) {
        let requests = self
            .devtools_requests
            .lock()
            .expect("devtools request queue lock")
            .drain(..)
            .collect::<Vec<_>>();
        for request in requests {
            match request {
                DevToolsRequest::SetOpen { owner, open: true } => {
                    self.open_devtools_window(event_loop, owner);
                }
                DevToolsRequest::SetOpen { owner, open: false } => {
                    self.close_devtools_windows(owner);
                }
            }
        }
    }

    fn open_devtools_window(&mut self, event_loop: &ActiveEventLoop, owner: WindowId) {
        let Some(owner_runtime) = self.windows.get(&owner) else {
            return;
        };
        if owner_runtime.role() != WindowRole::Product {
            return;
        }
        owner_runtime.handle().request_redraw();
        if let Some(window) = self.devtools_window_for(owner) {
            if let Some(runtime) = self.windows.get(&window) {
                runtime.handle().request_redraw();
            }
            return;
        }

        let options = WindowOptions::new("ZUI DevTools")
            .with_inner_size(crate::window::LogicalSize::new(440.0, 720.0))
            .with_chrome(WindowChrome::Native);
        let runtime = match WindowRuntime::open(
            event_loop,
            self.renderer_factory.as_mut(),
            &self.event_proxy,
            options,
            WindowRole::DevTools { owner },
            self.devtools_request_sender.clone(),
        ) {
            Ok(runtime) => runtime,
            Err(error) => {
                if self.error.is_none() {
                    self.error = Some(error);
                }
                event_loop.exit();
                return;
            }
        };
        let id = runtime.id();
        let metrics = runtime.metrics();
        runtime.handle().request_redraw();
        self.windows.insert(id, runtime);
        self.diagnostics.open_window(id, metrics);
    }

    pub(super) fn close_devtools_windows(&mut self, owner: WindowId) {
        if let Some(runtime) = self.windows.get(&owner) {
            runtime.handle().devtools().close_local();
            runtime.handle().request_redraw();
        }
        let ids = self
            .windows
            .iter()
            .filter_map(|(id, runtime)| {
                (runtime.role() == WindowRole::DevTools { owner }).then_some(*id)
            })
            .collect::<Vec<_>>();
        for id in ids {
            self.cursor_positions.remove(&id);
            self.windows.remove(&id);
            self.diagnostics.close_window(id);
        }
    }

    fn devtools_window_for(&self, owner: WindowId) -> Option<WindowId> {
        self.windows.iter().find_map(|(id, runtime)| {
            (runtime.role() == WindowRole::DevTools { owner }).then_some(*id)
        })
    }

    fn request_devtools_redraw(&self, owner: WindowId) {
        if let Some(window) = self.devtools_window_for(owner)
            && let Some(runtime) = self.windows.get(&window)
        {
            runtime.handle().request_redraw();
        }
    }

    pub(super) fn has_product_windows(&self) -> bool {
        self.windows
            .values()
            .any(|runtime| runtime.role() == WindowRole::Product)
    }

    fn logical_point(&self, window: WindowId, position: PhysicalPosition) -> Point {
        let scale_factor = self
            .windows
            .get(&window)
            .map(|runtime| runtime.metrics().scale_factor())
            .unwrap_or(1.0);
        Point::new(
            position.x as f32 / scale_factor as f32,
            position.y as f32 / scale_factor as f32,
        )
    }

    pub(super) fn handle_product_devtools_event(
        &mut self,
        window: WindowId,
        event: &WindowEvent,
    ) -> bool {
        let Some(runtime) = self.windows.get(&window) else {
            return false;
        };
        let devtools = runtime.handle().devtools();
        if !devtools.is_open() {
            return false;
        }
        match event {
            WindowEvent::CursorMoved { position } => {
                let point = self.logical_point(window, *position);
                self.cursor_positions.insert(window, point);
                if devtools.is_picking() {
                    devtools.hover_at(point);
                    runtime.handle().request_redraw();
                    self.request_devtools_redraw(window);
                    return true;
                }
            }
            WindowEvent::CursorLeft => {
                self.cursor_positions.remove(&window);
                if devtools.is_picking() {
                    devtools.set_hovered(None);
                    runtime.handle().request_redraw();
                    self.request_devtools_redraw(window);
                    return true;
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
            } if devtools.is_picking() => {
                if *state == ElementState::Released {
                    devtools.select_hovered();
                    runtime.handle().request_redraw();
                }
                self.request_devtools_redraw(window);
                return true;
            }
            WindowEvent::KeyboardInput { event } if devtools.is_picking() => {
                if event.state == ElementState::Pressed
                    && event.logical_key == Key::Named(NamedKey::Escape)
                {
                    devtools.stop_picking_or_close();
                    runtime.handle().request_redraw();
                    self.request_devtools_redraw(window);
                    return true;
                }
            }
            WindowEvent::MouseWheel { .. } if devtools.is_picking() => return true,
            _ => {}
        }
        false
    }

    pub(super) fn handle_devtools_window_event(
        &mut self,
        window: WindowId,
        owner: WindowId,
        event_loop: &ActiveEventLoop,
        event: &WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                if let Some(runtime) = self.windows.get(&owner) {
                    runtime.handle().devtools().close_local();
                    runtime.handle().request_redraw();
                }
                self.commands.push(WindowCommand::Close(window));
            }
            WindowEvent::RedrawRequested => self.render_devtools_window(window, owner, event_loop),
            WindowEvent::CursorMoved { position } => {
                self.cursor_positions
                    .insert(window, self.logical_point(window, *position));
            }
            WindowEvent::CursorLeft => {
                self.cursor_positions.remove(&window);
            }
            WindowEvent::KeyboardInput { event } => {
                if event.state == ElementState::Pressed
                    && event.logical_key == Key::Named(NamedKey::Escape)
                {
                    if let Some(runtime) = self.windows.get(&owner) {
                        runtime.handle().devtools().close_local();
                        runtime.handle().request_redraw();
                    }
                    self.commands.push(WindowCommand::Close(window));
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
            } => self.handle_devtools_click(window, owner),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(runtime) = self.windows.get(&window) {
                    runtime.handle().request_redraw();
                }
            }
            _ => {}
        }
    }

    fn handle_devtools_click(&mut self, window: WindowId, owner: WindowId) {
        let Some(point) = self.cursor_positions.get(&window).copied() else {
            return;
        };
        let Some(size) = self
            .windows
            .get(&window)
            .map(|runtime| runtime.metrics().logical_size())
        else {
            return;
        };
        let bounds = Rect::from_xywh(0.0, 0.0, size.width, size.height);
        let Some(owner_runtime) = self.windows.get(&owner) else {
            self.commands.push(WindowCommand::Close(window));
            return;
        };
        let devtools = owner_runtime.handle().devtools();
        match crate::devtools::view::toolbar_action_at(bounds, point) {
            Some(crate::devtools::view::ToolbarAction::Pick) => {
                devtools.toggle_picking();
                owner_runtime.handle().request_redraw();
                if let Some(runtime) = self.windows.get(&window) {
                    runtime.handle().request_redraw();
                }
            }
            Some(crate::devtools::view::ToolbarAction::Close) => {
                devtools.close_local();
                owner_runtime.handle().request_redraw();
                self.commands.push(WindowCommand::Close(window));
            }
            None => {
                if let Some(selection) = devtools.selection()
                    && let Some(index) =
                        crate::devtools::view::row_index_at(bounds, point, selection.path().len())
                {
                    devtools.select_index(index);
                    owner_runtime.handle().request_redraw();
                    if let Some(runtime) = self.windows.get(&window) {
                        runtime.handle().request_redraw();
                    }
                }
            }
        }
    }

    fn render_devtools_window(
        &mut self,
        window: WindowId,
        owner: WindowId,
        event_loop: &ActiveEventLoop,
    ) {
        let Some(owner_runtime) = self.windows.get(&owner) else {
            self.commands.push(WindowCommand::Close(window));
            return;
        };
        let devtools = owner_runtime.handle().devtools();
        if !devtools.is_open() {
            self.commands.push(WindowCommand::Close(window));
            return;
        }
        let Some(size) = self
            .windows
            .get(&window)
            .map(|runtime| runtime.metrics().logical_size())
        else {
            return;
        };
        let frame = devtools.inspection();
        let scene = crate::devtools::view::compose(size, frame.as_ref(), &devtools);
        let Some(runtime) = self.windows.get_mut(&window) else {
            return;
        };
        let metrics = runtime.metrics();
        match runtime.render_scene(&scene) {
            Ok(outcome) => self.diagnostics.present(
                window,
                metrics,
                self.diagnostics.scene_diagnostics(&scene, 0),
                outcome,
            ),
            Err(error) => {
                if self.error.is_none() {
                    self.error = Some(ApplicationError::product("zui DevTools rendering", error));
                }
                event_loop.exit();
            }
        }
    }
}
