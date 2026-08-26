use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

use crate::devtools::DevToolsRequest;
use crate::devtools::DevToolsRequestSender;
use crate::internal::ActiveEventLoop;
use crate::internal::ApplicationHandler;
use crate::internal::NativeWindowEvent;
use crate::internal::NativeWindowId;
use crate::ui::Point;
use crate::window::WindowEvent;
use crate::window::WindowId;
use crate::window::WindowRole;

use super::App;
use super::AppContext;
use super::AppContextParts;
use super::AppProxy;
use super::ApplicationError;
use super::BackgroundExecutor;
use super::ClipboardHandle;
use super::DiagnosticEventKind;
use super::DiagnosticsHandle;
use super::ExitPolicy;
use super::ProtocolUrl;
use super::RendererFactory;
use super::RuntimeEvent;
use super::Services;
use super::TaskScope;
use super::TimerRegistry;
use super::TimerScheduler;
use super::WindowCommand;
use super::WindowCommandQueue;
use super::WindowContext;
use super::WindowContextParts;
use super::WindowRuntime;

#[path = "host_devtools.rs"]
mod host_devtools;

pub(super) struct ApplicationHost<T: 'static, A> {
    pub(super) app: A,
    windows: HashMap<WindowId, WindowRuntime>,
    renderer_factory: Box<dyn RendererFactory>,
    clipboard: ClipboardHandle,
    services: Services,
    event_proxy: AppProxy<T>,
    pub(super) error: Option<ApplicationError>,
    commands: WindowCommandQueue,
    control_flow: crate::internal::ControlFlow,
    background: BackgroundExecutor<T>,
    timers: TimerScheduler<T>,
    timer_registry: TimerRegistry<T>,
    exit_policy: ExitPolicy,
    launch_urls: Vec<ProtocolUrl>,
    diagnostics: DiagnosticsHandle,
    devtools_requests: Arc<Mutex<VecDeque<DevToolsRequest>>>,
    devtools_request_sender: DevToolsRequestSender,
    cursor_positions: HashMap<WindowId, Point>,
}

pub(super) struct ApplicationResources<T: 'static> {
    pub(super) renderer_factory: Box<dyn RendererFactory>,
    pub(super) clipboard: ClipboardHandle,
    pub(super) services: Services,
    pub(super) event_proxy: AppProxy<T>,
    pub(super) background: BackgroundExecutor<T>,
    pub(super) timers: TimerScheduler<T>,
    pub(super) launch_urls: Vec<ProtocolUrl>,
    pub(super) diagnostics: DiagnosticsHandle,
}

impl<T: 'static, A> ApplicationHost<T, A> {
    pub(super) fn new(app: A, resources: ApplicationResources<T>, exit_policy: ExitPolicy) -> Self
    where
        T: Send,
    {
        let devtools_requests = Arc::new(Mutex::new(VecDeque::new()));
        let request_queue = Arc::clone(&devtools_requests);
        let request_proxy = resources.event_proxy.inner.clone();
        let devtools_request_sender: DevToolsRequestSender = Arc::new(move |request| {
            request_queue
                .lock()
                .expect("devtools request queue lock")
                .push_back(request);
            let _ = request_proxy.send_event(RuntimeEvent::DevToolsWake);
        });
        let runtime_proxy = resources.event_proxy.inner.clone();
        resources
            .services
            .menus()
            .set_event_handler(Some(Arc::new(move |action| {
                let _ = runtime_proxy.send_event(RuntimeEvent::MenuAction(action));
            })));
        let tray_proxy = resources.event_proxy.inner.clone();
        resources
            .services
            .tray()
            .set_event_handler(Some(Arc::new(move |event| {
                let _ = tray_proxy.send_event(RuntimeEvent::Tray(event));
            })));
        let shortcut_proxy = resources.event_proxy.inner.clone();
        resources
            .services
            .global_shortcuts()
            .set_event_handler(Some(Arc::new(move |event| {
                let _ = shortcut_proxy.send_event(RuntimeEvent::GlobalShortcut(event));
            })));
        Self {
            app,
            windows: HashMap::new(),
            renderer_factory: resources.renderer_factory,
            clipboard: resources.clipboard,
            services: resources.services,
            event_proxy: resources.event_proxy,
            error: None,
            commands: WindowCommandQueue::default(),
            control_flow: crate::internal::ControlFlow::Wait,
            background: resources.background,
            timers: resources.timers,
            timer_registry: TimerRegistry::default(),
            exit_policy,
            launch_urls: resources.launch_urls,
            diagnostics: resources.diagnostics,
            devtools_requests,
            devtools_request_sender,
            cursor_positions: HashMap::new(),
        }
    }
}

impl<T, A> ApplicationHost<T, A>
where
    T: Send + 'static,
    A: App<T>,
{
    fn with_app_context<R>(
        &mut self,
        event_loop: &ActiveEventLoop,
        callback: impl FnOnce(&mut A, &mut AppContext<'_, T>) -> R,
    ) -> R {
        let Self {
            app,
            windows,
            renderer_factory,
            clipboard,
            services,
            event_proxy,
            error,
            commands,
            control_flow,
            background,
            timers,
            diagnostics,
            devtools_request_sender,
            ..
        } = self;
        let mut context = AppContext::new(AppContextParts {
            event_loop,
            windows,
            renderer_factory: renderer_factory.as_mut(),
            clipboard,
            services,
            event_proxy,
            error,
            commands,
            control_flow,
            background,
            timers,
            diagnostics,
            devtools_requests: devtools_request_sender.clone(),
        });
        callback(app, &mut context)
    }

    fn deliver_product_event(&mut self, event_loop: &ActiveEventLoop, event: T) {
        self.with_app_context(event_loop, |app, context| app.user_event(context, event));
    }

    fn process_window_commands(&mut self, event_loop: &ActiveEventLoop) {
        self.process_devtools_requests(event_loop);
        let mut closed_product_window = false;
        while let Some(command) = self.commands.pop() {
            match command {
                WindowCommand::Opened(window) => {
                    if self
                        .windows
                        .get(&window)
                        .is_some_and(|runtime| runtime.role() == WindowRole::Product)
                    {
                        self.with_app_context(event_loop, |app, context| {
                            app.window_opened(context, window)
                        });
                    }
                }
                WindowCommand::Close(window) => {
                    let role = self.windows.get(&window).map(WindowRuntime::role);
                    if role == Some(WindowRole::Product) {
                        self.close_devtools_windows(window);
                        self.services.menus().detach_window(window);
                    }
                    if self.windows.remove(&window).is_some() {
                        self.background.cancel_window(window);
                        self.timer_registry.cancel_scope(TaskScope::Window(window));
                        self.cursor_positions.remove(&window);
                        self.diagnostics.close_window(window);
                        if role == Some(WindowRole::Product) {
                            closed_product_window = true;
                            self.with_app_context(event_loop, |app, context| {
                                app.window_closed(context, window)
                            });
                        } else if let Some(WindowRole::DevTools { owner }) = role
                            && let Some(runtime) = self.windows.get(&owner)
                        {
                            runtime.handle().devtools().close_local();
                            runtime.request_redraw();
                        }
                    }
                }
                WindowCommand::Exit => event_loop.exit(),
            }
        }
        if closed_product_window
            && !self.has_product_windows()
            && self.exit_policy == ExitPolicy::OnLastWindowClosed
        {
            event_loop.exit();
        }
    }

    fn apply_control_flow(&self, event_loop: &ActiveEventLoop) {
        let timer_deadline = self.timer_registry.next_deadline();
        let control_flow = match (self.control_flow, timer_deadline) {
            (crate::internal::ControlFlow::Poll, _) => crate::internal::ControlFlow::Poll,
            (crate::internal::ControlFlow::Wait, Some(deadline)) => {
                crate::internal::ControlFlow::WaitUntil(deadline)
            }
            (crate::internal::ControlFlow::WaitUntil(application), Some(timer)) => {
                crate::internal::ControlFlow::WaitUntil(application.min(timer))
            }
            (application, None) => application,
        };
        event_loop.set_control_flow(control_flow.into_native());
    }

    fn refresh_diagnostics(&self) {
        self.diagnostics
            .set_work_counts(self.background.active_count(), self.timer_registry.len());
    }
}

impl<T, A> ApplicationHandler<RuntimeEvent<T>> for ApplicationHost<T, A>
where
    T: Send + 'static,
    A: App<T>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.diagnostics.record(DiagnosticEventKind::Resumed);
        self.with_app_context(event_loop, |app, context| app.resumed(context));
        self.process_window_commands(event_loop);
        for url in std::mem::take(&mut self.launch_urls) {
            self.diagnostics.record(DiagnosticEventKind::OpenUrl);
            self.with_app_context(event_loop, |app, context| app.open_url(context, url));
            self.process_window_commands(event_loop);
        }
        self.apply_control_flow(event_loop);
        self.refresh_diagnostics();
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.diagnostics.record(DiagnosticEventKind::Suspended);
        self.with_app_context(event_loop, |app, context| app.suspended(context));
        self.process_window_commands(event_loop);
        self.apply_control_flow(event_loop);
        self.refresh_diagnostics();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: NativeWindowId,
        event: NativeWindowEvent,
    ) {
        let window_id = WindowId::from_native(window_id);
        let (role, event, destroyed) = {
            let Some(runtime) = self.windows.get_mut(&window_id) else {
                return;
            };
            runtime.process_accessibility_window_event(&event);
            let event = WindowEvent::from_native(event);
            runtime.apply_platform_event(&event);
            self.diagnostics.update_window(window_id, runtime.metrics());
            let destroyed = matches!(event, WindowEvent::Destroyed);
            (runtime.role(), event, destroyed)
        };
        self.diagnostics
            .record(DiagnosticEventKind::WindowEvent(window_id));
        match role {
            WindowRole::Product => {
                if !self.handle_product_devtools_event(window_id, &event) {
                    let Some(runtime) = self.windows.get_mut(&window_id) else {
                        return;
                    };
                    let mut context = WindowContext::new(WindowContextParts {
                        runtime,
                        clipboard: &self.clipboard,
                        services: &self.services,
                        error: &mut self.error,
                        commands: &mut self.commands,
                        background: &self.background,
                        timers: &self.timers,
                        diagnostics: &self.diagnostics,
                    });
                    if matches!(event, WindowEvent::RedrawRequested) {
                        self.app.redraw(&mut context);
                    } else {
                        self.app.window_event(&mut context, event);
                    }
                }
            }
            WindowRole::DevTools { owner } => {
                self.handle_devtools_window_event(window_id, owner, event_loop, &event);
            }
        }
        if destroyed && role == WindowRole::Product {
            self.commands.push(WindowCommand::Close(window_id));
        }
        self.process_window_commands(event_loop);
        self.apply_control_flow(event_loop);
        self.refresh_diagnostics();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeEvent<T>) {
        match event {
            RuntimeEvent::Product(event) => {
                self.diagnostics.record(DiagnosticEventKind::UserEvent);
                self.deliver_product_event(event_loop, event);
            }
            RuntimeEvent::ScheduleTimer(timer) => self.timer_registry.schedule(timer),
            RuntimeEvent::CancelTimer(timer) => self.timer_registry.cancel(timer),
            RuntimeEvent::MenuAction(action) => {
                self.diagnostics.record(DiagnosticEventKind::MenuAction);
                self.with_app_context(event_loop, |app, context| app.menu_action(context, action));
            }
            RuntimeEvent::Tray(event) => {
                self.diagnostics.record(DiagnosticEventKind::TrayEvent);
                self.with_app_context(event_loop, |app, context| app.tray_event(context, event));
            }
            RuntimeEvent::GlobalShortcut(event) => {
                self.diagnostics.record(DiagnosticEventKind::GlobalShortcut);
                self.with_app_context(event_loop, |app, context| {
                    app.global_shortcut(context, event)
                });
            }
            RuntimeEvent::OpenUrl(url) => {
                self.diagnostics.record(DiagnosticEventKind::OpenUrl);
                self.with_app_context(event_loop, |app, context| app.open_url(context, url));
            }
            RuntimeEvent::Accessibility(event) => {
                let window = WindowId::from_native(event.window_id);
                let action = self
                    .windows
                    .get_mut(&window)
                    .filter(|runtime| runtime.role() == WindowRole::Product)
                    .and_then(|runtime| runtime.handle_accessibility_event(event.window_event));
                if let Some(action) = action {
                    self.diagnostics
                        .record(DiagnosticEventKind::AccessibilityAction);
                    self.with_app_context(event_loop, |app, context| {
                        app.accessibility_action(context, action)
                    });
                }
            }
            RuntimeEvent::DevToolsWake => {}
        }
        self.process_window_commands(event_loop);
        self.apply_control_flow(event_loop);
        self.refresh_diagnostics();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        for event in self.timer_registry.take_due(Instant::now()) {
            self.deliver_product_event(event_loop, event);
            self.process_window_commands(event_loop);
        }
        self.with_app_context(event_loop, |app, context| app.about_to_wait(context));
        self.process_window_commands(event_loop);
        self.apply_control_flow(event_loop);
        self.refresh_diagnostics();
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.diagnostics.record(DiagnosticEventKind::Exiting);
        self.background.cancel_all();
        self.services.menus().set_event_handler(None);
        self.services.tray().set_event_handler(None);
        self.services.global_shortcuts().set_event_handler(None);
        let _ = self.services.global_shortcuts().unregister_all();
        self.with_app_context(event_loop, |app, context| app.exiting(context));
    }
}
