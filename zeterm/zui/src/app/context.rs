use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use std::time::Instant;

use crate::devtools::DevToolsHandle;
use crate::devtools::DevToolsRequestSender;
use crate::internal::ActiveEventLoop;
use crate::internal::ControlFlow;
use crate::render::RenderOutcome;
use crate::render::RendererError;
use crate::runtime::AccessibilityNode;
use crate::ui::presentation::UiScene;
use crate::window::WindowHandle;
use crate::window::WindowId;

use super::AppProxy;
use super::ApplicationError;
use super::BackgroundExecutor;
use super::ClipboardHandle;
use super::DiagnosticsHandle;
use super::OpenedWindow;
use super::RendererFactory;
use super::Services;
use super::Task;
use super::TaskScope;
use super::TaskSpawnError;
use super::Timer;
use super::TimerScheduleError;
use super::TimerScheduler;
use super::WindowMetrics;
use super::WindowOptions;
use super::lifecycle::WindowCommand;
use super::lifecycle::WindowCommandQueue;
use crate::window::WindowRole;
use crate::window::WindowRuntime;

/// Main-thread application capabilities available outside a specific window callback.
pub struct AppContext<'a, T: 'static> {
    event_loop: &'a ActiveEventLoop,
    windows: &'a mut HashMap<WindowId, WindowRuntime>,
    renderer_factory: &'a mut dyn RendererFactory,
    clipboard: &'a ClipboardHandle,
    services: &'a Services,
    event_proxy: &'a AppProxy<T>,
    error: &'a mut Option<ApplicationError>,
    commands: &'a mut WindowCommandQueue,
    control_flow: &'a mut ControlFlow,
    background: &'a BackgroundExecutor<T>,
    timers: &'a TimerScheduler<T>,
    diagnostics: &'a DiagnosticsHandle,
    devtools_requests: DevToolsRequestSender,
}

pub(super) struct AppContextParts<'a, T: 'static> {
    pub(super) event_loop: &'a ActiveEventLoop,
    pub(super) windows: &'a mut HashMap<WindowId, WindowRuntime>,
    pub(super) renderer_factory: &'a mut dyn RendererFactory,
    pub(super) clipboard: &'a ClipboardHandle,
    pub(super) services: &'a Services,
    pub(super) event_proxy: &'a AppProxy<T>,
    pub(super) error: &'a mut Option<ApplicationError>,
    pub(super) commands: &'a mut WindowCommandQueue,
    pub(super) control_flow: &'a mut ControlFlow,
    pub(super) background: &'a BackgroundExecutor<T>,
    pub(super) timers: &'a TimerScheduler<T>,
    pub(super) diagnostics: &'a DiagnosticsHandle,
    pub(super) devtools_requests: DevToolsRequestSender,
}

impl<'a, T: 'static> AppContext<'a, T> {
    pub(super) fn new(parts: AppContextParts<'a, T>) -> Self {
        Self {
            event_loop: parts.event_loop,
            windows: parts.windows,
            renderer_factory: parts.renderer_factory,
            clipboard: parts.clipboard,
            services: parts.services,
            event_proxy: parts.event_proxy,
            error: parts.error,
            commands: parts.commands,
            control_flow: parts.control_flow,
            background: parts.background,
            timers: parts.timers,
            diagnostics: parts.diagnostics,
            devtools_requests: parts.devtools_requests,
        }
    }

    /// Opens a runtime-owned native window and initializes its renderer.
    pub fn open_window(&mut self, options: WindowOptions) -> Result<OpenedWindow, ApplicationError>
    where
        T: Send,
    {
        let runtime = WindowRuntime::open(
            self.event_loop,
            self.renderer_factory,
            self.event_proxy,
            options,
            WindowRole::Product,
            self.devtools_requests.clone(),
        )?;
        let opened = runtime.opened_window();
        self.services
            .menus()
            .attach_window(opened.handle())
            .map_err(|source| {
                ApplicationError::product("native application menu attachment", source)
            })?;
        self.windows.insert(opened.id(), runtime);
        self.diagnostics.open_window(opened.id(), opened.metrics());
        self.commands.push(WindowCommand::Opened(opened.id()));
        Ok(opened)
    }

    /// Returns whether the runtime currently owns a window with `id`.
    pub fn contains_window(&self, id: WindowId) -> bool {
        self.windows.contains_key(&id)
    }

    /// Returns the current metrics for a runtime-owned window.
    pub fn window_metrics(&self, id: WindowId) -> Option<WindowMetrics> {
        self.windows.get(&id).map(WindowRuntime::metrics)
    }

    /// Returns a non-owning platform capability for a runtime-owned window.
    pub fn window_handle(&self, id: WindowId) -> Option<WindowHandle> {
        self.windows.get(&id).map(WindowRuntime::handle)
    }

    /// Requests another frame for a runtime-owned window.
    pub fn request_redraw(&self, id: WindowId) {
        if let Some(window) = self.windows.get(&id) {
            window.handle().request_redraw();
        }
    }

    /// Returns the application-wide text clipboard capability.
    pub fn clipboard(&self) -> ClipboardHandle {
        self.clipboard.clone()
    }

    /// Returns the typed operating-system service capabilities.
    pub fn services(&self) -> Services {
        self.services.clone()
    }

    /// Returns a bounded snapshot and trace capability for this application runtime.
    pub fn diagnostics(&self) -> DiagnosticsHandle {
        self.diagnostics.clone()
    }

    /// Queues one window for closing after the current application callback returns.
    pub fn close_window(&mut self, id: WindowId) {
        self.commands.push(WindowCommand::Close(id));
    }

    /// Sets the native event-loop wakeup policy selected by the application.
    pub fn set_control_flow(&mut self, control_flow: ControlFlow) {
        *self.control_flow = control_flow;
    }

    /// Queues a normal application exit after the current callback returns.
    pub fn exit(&mut self) {
        self.commands.push(WindowCommand::Exit);
    }

    /// Records a fatal runtime error and exits the native application.
    pub fn exit_with_error(&mut self, error: ApplicationError) {
        if self.error.is_none() {
            *self.error = Some(error);
        }
        self.commands.push(WindowCommand::Exit);
    }

    /// Returns the cloneable application-wide background executor.
    pub fn background_executor(&self) -> BackgroundExecutor<T> {
        self.background.clone()
    }

    /// Starts application-scoped background work and delivers its output as a user event.
    pub fn spawn<F>(&self, future: F) -> Result<Task, TaskSpawnError>
    where
        T: Send,
        F: Future<Output = T> + Send + 'static,
    {
        self.background.spawn(TaskScope::Application, future)
    }

    /// Schedules an application-scoped user event after `delay`.
    pub fn schedule_after(&self, delay: Duration, event: T) -> Result<Timer, TimerScheduleError<T>>
    where
        T: Send,
    {
        self.timers.schedule_after(delay, event)
    }

    /// Schedules an application-scoped user event at `deadline`.
    pub fn schedule_at(&self, deadline: Instant, event: T) -> Result<Timer, TimerScheduleError<T>>
    where
        T: Send,
    {
        self.timers.schedule_at(deadline, event)
    }
}

/// Runtime and platform capabilities scoped to one native window callback.
pub struct WindowContext<'a, T: 'static> {
    runtime: &'a mut WindowRuntime,
    clipboard: &'a ClipboardHandle,
    services: &'a Services,
    error: &'a mut Option<ApplicationError>,
    commands: &'a mut WindowCommandQueue,
    background: &'a BackgroundExecutor<T>,
    timers: &'a TimerScheduler<T>,
    diagnostics: &'a DiagnosticsHandle,
}

pub(super) struct WindowContextParts<'a, T: 'static> {
    pub(super) runtime: &'a mut WindowRuntime,
    pub(super) clipboard: &'a ClipboardHandle,
    pub(super) services: &'a Services,
    pub(super) error: &'a mut Option<ApplicationError>,
    pub(super) commands: &'a mut WindowCommandQueue,
    pub(super) background: &'a BackgroundExecutor<T>,
    pub(super) timers: &'a TimerScheduler<T>,
    pub(super) diagnostics: &'a DiagnosticsHandle,
}

impl<'a, T: 'static> WindowContext<'a, T> {
    pub(super) fn new(parts: WindowContextParts<'a, T>) -> Self {
        Self {
            runtime: parts.runtime,
            clipboard: parts.clipboard,
            services: parts.services,
            error: parts.error,
            commands: parts.commands,
            background: parts.background,
            timers: parts.timers,
            diagnostics: parts.diagnostics,
        }
    }

    /// Returns the stable platform identity of this window.
    pub fn id(&self) -> WindowId {
        self.runtime.id()
    }

    /// Returns the current physical and logical window metrics.
    pub fn metrics(&self) -> WindowMetrics {
        self.runtime.metrics()
    }

    /// Returns a non-owning capability for product-directed platform updates.
    pub fn window_handle(&self) -> WindowHandle {
        self.runtime.handle()
    }

    /// Returns the shared DevTools session capability for this window.
    pub fn devtools(&self) -> DevToolsHandle {
        self.runtime.handle().devtools()
    }

    /// Opens the default zui DevTools window for this window and schedules a frame.
    pub fn open_devtools(&self) {
        self.runtime.handle().open_devtools();
    }

    /// Closes the default zui DevTools window for this window and schedules a frame.
    pub fn close_devtools(&self) {
        self.runtime.handle().close_devtools();
    }

    /// Toggles DevTools for this window and returns whether it is now open.
    pub fn toggle_devtools(&self) -> bool {
        self.runtime.handle().toggle_devtools()
    }

    /// Returns whether DevTools is currently open for this window.
    pub fn is_devtools_open(&self) -> bool {
        self.runtime.handle().is_devtools_open()
    }

    /// Schedules another frame for this window.
    pub fn request_redraw(&self) {
        self.runtime.handle().request_redraw();
    }

    /// Returns the application-wide text clipboard capability.
    pub fn clipboard(&self) -> ClipboardHandle {
        self.clipboard.clone()
    }

    /// Returns the typed operating-system service capabilities.
    pub fn services(&self) -> Services {
        self.services.clone()
    }

    /// Returns a bounded snapshot and trace capability for this application runtime.
    pub fn diagnostics(&self) -> DiagnosticsHandle {
        self.diagnostics.clone()
    }

    /// Submits one immutable UI scene through this window's renderer.
    pub fn render_scene(&mut self, scene: &UiScene) -> Result<RenderOutcome, RendererError> {
        let devtools = self.runtime.handle().devtools();
        devtools.set_inspection(scene.inspection().clone());
        let decorated = devtools
            .is_open()
            .then(|| crate::devtools::view::decorate_product_scene(scene, &devtools))
            .flatten();
        let scene = decorated.as_ref().unwrap_or(scene);
        let outcome = self.runtime.render_scene(scene)?;
        self.diagnostics.present(
            self.id(),
            self.metrics(),
            self.diagnostics.scene_diagnostics(scene, 0),
            outcome,
        );
        Ok(outcome)
    }

    /// Synchronizes the operating-system accessibility tree and renders one immutable scene.
    pub fn present_scene(
        &mut self,
        scene: &UiScene,
        accessibility: &[AccessibilityNode],
    ) -> Result<RenderOutcome, RendererError> {
        let devtools = self.runtime.handle().devtools();
        devtools.set_inspection(scene.inspection().clone());
        let decorated = devtools
            .is_open()
            .then(|| crate::devtools::view::decorate_product_scene(scene, &devtools))
            .flatten();
        let scene = decorated.as_ref().unwrap_or(scene);
        self.runtime.update_accessibility(accessibility);
        let outcome = self.runtime.render_scene(scene)?;
        self.diagnostics.present(
            self.id(),
            self.metrics(),
            self.diagnostics
                .scene_diagnostics(scene, accessibility.len()),
            outcome,
        );
        Ok(outcome)
    }

    /// Closes this window after the current callback returns.
    pub fn close(&mut self) {
        self.commands.push(WindowCommand::Close(self.id()));
    }

    /// Queues a normal application exit after the current callback returns.
    pub fn exit(&mut self) {
        self.commands.push(WindowCommand::Exit);
    }

    /// Records a fatal runtime error and exits the complete native application.
    pub fn exit_with_error(&mut self, error: ApplicationError) {
        if self.error.is_none() {
            *self.error = Some(error);
        }
        self.commands.push(WindowCommand::Exit);
    }

    /// Starts background work cancelled automatically when this window closes.
    pub fn spawn<F>(&self, future: F) -> Result<Task, TaskSpawnError>
    where
        T: Send,
        F: Future<Output = T> + Send + 'static,
    {
        self.background.spawn(TaskScope::Window(self.id()), future)
    }

    /// Schedules a user event cancelled automatically when this window closes.
    pub fn schedule_after(&self, delay: Duration, event: T) -> Result<Timer, TimerScheduleError<T>>
    where
        T: Send,
    {
        self.timers
            .schedule_after_in_scope(TaskScope::Window(self.id()), delay, event)
    }
}
