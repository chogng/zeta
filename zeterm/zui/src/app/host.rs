use std::collections::HashMap;
use std::time::Instant;

use crate::internal::ActiveEventLoop;
use crate::internal::ApplicationHandler;
use crate::internal::NativeWindowEvent;
use crate::internal::NativeWindowId;
use crate::window::WindowEvent;
use crate::window::WindowId;

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
        let runtime_proxy = resources.event_proxy.inner.clone();
        resources
            .services
            .menus()
            .set_event_handler(Some(std::sync::Arc::new(move |action| {
                let _ = runtime_proxy.send_event(RuntimeEvent::MenuAction(action));
            })));
        let tray_proxy = resources.event_proxy.inner.clone();
        resources
            .services
            .tray()
            .set_event_handler(Some(std::sync::Arc::new(move |event| {
                let _ = tray_proxy.send_event(RuntimeEvent::Tray(event));
            })));
        let shortcut_proxy = resources.event_proxy.inner.clone();
        resources
            .services
            .global_shortcuts()
            .set_event_handler(Some(std::sync::Arc::new(move |event| {
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
        }
    }
}

impl<T, A> ApplicationHost<T, A>
where
    T: 'static,
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
        });
        callback(app, &mut context)
    }

    fn deliver_product_event(&mut self, event_loop: &ActiveEventLoop, event: T) {
        self.with_app_context(event_loop, |app, context| app.user_event(context, event));
    }

    fn process_window_commands(&mut self, event_loop: &ActiveEventLoop) {
        let mut closed_window = false;
        while let Some(command) = self.commands.pop() {
            match command {
                WindowCommand::Opened(window) => {
                    if self.windows.contains_key(&window) {
                        self.with_app_context(event_loop, |app, context| {
                            app.window_opened(context, window)
                        });
                    }
                }
                WindowCommand::Close(window) => {
                    if self.windows.remove(&window).is_some() {
                        closed_window = true;
                        self.background.cancel_window(window);
                        self.timer_registry.cancel_scope(TaskScope::Window(window));
                        self.diagnostics.close_window(window);
                        self.with_app_context(event_loop, |app, context| {
                            app.window_closed(context, window)
                        });
                    }
                }
                WindowCommand::Exit => event_loop.exit(),
            }
        }
        if closed_window
            && self.windows.is_empty()
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
    T: 'static,
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
        let Some(runtime) = self.windows.get_mut(&window_id) else {
            return;
        };
        runtime.process_accessibility_window_event(&event);
        let event = WindowEvent::from_native(event);
        runtime.apply_platform_event(&event);
        self.diagnostics.update_window(window_id, runtime.metrics());
        self.diagnostics
            .record(DiagnosticEventKind::WindowEvent(window_id));
        let destroyed = matches!(event, WindowEvent::Destroyed);
        {
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
        if destroyed {
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
                    .and_then(|runtime| runtime.handle_accessibility_event(event.window_event));
                if let Some(action) = action {
                    self.diagnostics
                        .record(DiagnosticEventKind::AccessibilityAction);
                    self.with_app_context(event_loop, |app, context| {
                        app.accessibility_action(context, action)
                    });
                }
            }
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
