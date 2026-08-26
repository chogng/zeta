use std::error::Error;
use std::future::Future;
use std::time::Duration;
use std::time::Instant;

pub use crate::accessibility::AccessibilityAction;
pub use crate::accessibility::AccessibilityActionKind;
use crate::devtools::DiagnosticEventKind;
use crate::devtools::DiagnosticsHandle;
use crate::devtools::DiagnosticsSink;
use crate::render::RendererFactory;
#[cfg(feature = "wgpu")]
use crate::render::WgpuRendererFactory;
use crate::runtime::BackgroundExecutor;
use crate::runtime::Task;
use crate::runtime::TaskScope;
use crate::runtime::TaskSpawnError;
use crate::runtime::Timer;
use crate::runtime::TimerRegistry;
use crate::runtime::TimerScheduleError;
use crate::runtime::TimerScheduler;
use crate::services::Clipboard;
use crate::services::ClipboardHandle;
use crate::services::FileDialogService;
use crate::services::GlobalShortcutEvent;
use crate::services::GlobalShortcutService;
use crate::services::MenuItemId;
use crate::services::MenuService;
use crate::services::NotificationService;
use crate::services::OpenerService;
use crate::services::ProcessService;
use crate::services::ResourceService;
use crate::services::Services;
use crate::services::SystemClipboard;
use crate::services::SystemResourceLocator;
use crate::services::TrayEvent;
use crate::services::TrayService;
use crate::services::UpdateService;
use crate::window::OpenedWindow;
use crate::window::WindowEvent;
use crate::window::WindowId;
use crate::window::WindowMetrics;
use crate::window::WindowOptions;
use crate::window::WindowRuntime;
use thiserror::Error;

mod builder;
mod context;
mod host;
mod lifecycle;
pub(crate) mod native_host;
mod protocol;
pub(crate) mod runtime_event;

pub use builder::ApplicationBuilder;
pub use context::AppContext;
use context::AppContextParts;
pub use context::WindowContext;
use context::WindowContextParts;
pub use lifecycle::ExitPolicy;
use lifecycle::WindowCommand;
use lifecycle::WindowCommandQueue;
pub use native_host::ApplicationRunError;
pub use native_host::ControlFlow;
pub use protocol::ProtocolScheme;
pub use protocol::ProtocolSchemeError;
pub use protocol::ProtocolUrl;
pub use protocol::ProtocolUrlError;
pub use runtime_event::AppDisconnected;
pub use runtime_event::AppProxy;
use runtime_event::RuntimeEvent;

/// Fatal failure crossing the reusable native application runtime boundary.
#[derive(Debug, Error)]
#[error("{operation} failed: {source}")]
pub struct ApplicationError {
    operation: &'static str,
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl ApplicationError {
    pub(crate) fn window(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            operation: "native window creation",
            source: Box::new(source),
        }
    }

    pub(crate) fn renderer(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            operation: "renderer initialization",
            source: Box::new(source),
        }
    }

    /// Wraps a fatal product or platform-service failure with a stable operation label.
    pub fn product(operation: &'static str, source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            operation,
            source: Box::new(source),
        }
    }
}

/// Product-owned lifecycle and event handling executed by [`Application`].
///
/// Implementations retain domain state and build UI frames. The runtime owns native windows,
/// renderer instances, platform resize synchronization, and event-loop dispatch.
pub trait App<T: 'static> {
    /// Creates initial windows and starts product resources after the native loop resumes.
    fn resumed(&mut self, context: &mut AppContext<'_, T>);

    /// Observes platform suspension after the runtime has stopped receiving active window work.
    fn suspended(&mut self, _context: &mut AppContext<'_, T>) {}

    /// Observes a window after its native resources and renderer enter the runtime registry.
    fn window_opened(&mut self, _context: &mut AppContext<'_, T>, _window: WindowId) {}

    /// Handles a non-redraw event for one runtime-owned window.
    fn window_event(&mut self, context: &mut WindowContext<'_, T>, event: WindowEvent) {
        if matches!(event, WindowEvent::CloseRequested) {
            context.close();
        }
    }

    /// Builds and presents one frame for a runtime-owned window.
    fn redraw(&mut self, _context: &mut WindowContext<'_, T>) {}

    /// Observes a window after its renderer and native resources leave the runtime registry.
    fn window_closed(&mut self, _context: &mut AppContext<'_, T>, _window: WindowId) {}

    /// Handles an action selected from the application menu.
    fn menu_action(&mut self, _context: &mut AppContext<'_, T>, _action: MenuItemId) {}

    /// Handles pointer interaction with a runtime-owned system-tray item.
    fn tray_event(&mut self, _context: &mut AppContext<'_, T>, _event: TrayEvent) {}

    /// Handles a registered system-wide keyboard shortcut.
    fn global_shortcut(&mut self, _context: &mut AppContext<'_, T>, _event: GlobalShortcutEvent) {}

    /// Handles a launch or forwarded URL accepted by the application builder.
    fn open_url(&mut self, _context: &mut AppContext<'_, T>, _url: ProtocolUrl) {}

    /// Handles a focus or activation request from operating-system assistive technology.
    fn accessibility_action(
        &mut self,
        _context: &mut AppContext<'_, T>,
        _action: AccessibilityAction,
    ) {
    }

    /// Projects a background or application-defined event into product state.
    fn user_event(&mut self, _context: &mut AppContext<'_, T>, _event: T) {}

    /// Advances product deadlines before the native event loop sleeps.
    fn about_to_wait(&mut self, _context: &mut AppContext<'_, T>) {}

    /// Releases product resources while the event loop is exiting.
    fn exiting(&mut self, _context: &mut AppContext<'_, T>) {}
}

/// Completed application state and any fatal runtime failure recorded before exit.
pub struct ApplicationExit<A> {
    app: A,
    error: Option<ApplicationError>,
}

impl<A> ApplicationExit<A> {
    /// Returns the final product state after the native event loop exits.
    pub const fn app(&self) -> &A {
        &self.app
    }

    /// Returns the fatal runtime failure that caused termination, if any.
    pub const fn error(&self) -> Option<&ApplicationError> {
        self.error.as_ref()
    }

    /// Consumes the exit report and returns the final product state and runtime failure.
    pub fn into_parts(self) -> (A, Option<ApplicationError>) {
        (self.app, self.error)
    }
}

/// Entry point for running reusable native applications on the default or an injected renderer.
pub struct Application;

impl Application {
    /// Creates a configurable application builder using the default wgpu renderer backend.
    #[cfg(feature = "wgpu")]
    pub fn builder() -> ApplicationBuilder {
        ApplicationBuilder::new(WgpuRendererFactory)
    }

    /// Creates a configurable application builder using an explicitly selected renderer factory.
    pub fn with_renderer(renderer_factory: impl RendererFactory + 'static) -> ApplicationBuilder {
        ApplicationBuilder::new(renderer_factory)
    }

    /// Runs an application using the default wgpu renderer backend.
    #[cfg(feature = "wgpu")]
    pub fn run<T, A, C>(create: C) -> Result<ApplicationExit<A>, ApplicationRunError>
    where
        T: Send + 'static,
        A: App<T> + 'static,
        C: FnOnce(ApplicationHandle<T>) -> A,
    {
        Self::builder().run(create)
    }

    /// Runs an application using an explicitly selected renderer factory.
    pub fn run_with_renderer<T, A, C>(
        renderer_factory: impl RendererFactory + 'static,
        create: C,
    ) -> Result<ApplicationExit<A>, ApplicationRunError>
    where
        T: Send + 'static,
        A: App<T> + 'static,
        C: FnOnce(ApplicationHandle<T>) -> A,
    {
        Self::with_renderer(renderer_factory).run(create)
    }
}

/// Cloneable application-wide capabilities supplied while product state is constructed.
#[derive(Clone)]
pub struct ApplicationHandle<T: 'static> {
    event_proxy: AppProxy<T>,
    clipboard: ClipboardHandle,
    services: Services,
    background: BackgroundExecutor<T>,
    timers: TimerScheduler<T>,
    diagnostics: DiagnosticsHandle,
}

impl<T: 'static> ApplicationHandle<T> {
    /// Returns the typed main-thread wakeup proxy for background product work.
    pub fn event_proxy(&self) -> AppProxy<T> {
        self.event_proxy.clone()
    }

    /// Returns the typed main-thread application proxy for background work.
    pub fn proxy(&self) -> AppProxy<T> {
        self.event_proxy.clone()
    }

    /// Returns the runtime-owned text clipboard capability.
    pub fn clipboard(&self) -> ClipboardHandle {
        self.clipboard.clone()
    }

    /// Returns all typed operating-system service capabilities.
    pub fn services(&self) -> Services {
        self.services.clone()
    }

    /// Returns the application-wide background executor.
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

    /// Returns the application-wide event-loop timer scheduler.
    pub fn timers(&self) -> TimerScheduler<T> {
        self.timers.clone()
    }

    /// Returns the application-wide bounded runtime diagnostics capability.
    pub fn diagnostics(&self) -> DiagnosticsHandle {
        self.diagnostics.clone()
    }

    /// Schedules an application event relative to the current monotonic time.
    pub fn schedule_after(&self, delay: Duration, event: T) -> Result<Timer, TimerScheduleError<T>>
    where
        T: Send,
    {
        self.timers.schedule_after(delay, event)
    }

    /// Schedules an application event at an explicit monotonic deadline.
    pub fn schedule_at(&self, deadline: Instant, event: T) -> Result<Timer, TimerScheduleError<T>>
    where
        T: Send,
    {
        self.timers.schedule_at(deadline, event)
    }
}
