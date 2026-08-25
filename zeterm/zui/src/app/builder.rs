use crate::internal::ApplicationRunError;
use crate::internal::run_application_with_user_events;

use super::App;
use super::AppProxy;
use super::ApplicationExit;
use super::ApplicationHandle;
use super::BackgroundExecutor;
use super::Clipboard;
use super::ClipboardHandle;
use super::DiagnosticsHandle;
use super::DiagnosticsSink;
use super::ExitPolicy;
use super::FileDialogService;
use super::GlobalShortcutService;
use super::MenuService;
use super::NotificationService;
use super::OpenerService;
use super::ProcessService;
use super::ProtocolScheme;
use super::ProtocolUrl;
use super::RendererFactory;
use super::ResourceService;
use super::Services;
use super::SystemClipboard;
use super::SystemResourceLocator;
use super::TimerScheduler;
use super::TrayService;
use super::UpdateService;
use super::host::ApplicationHost;
use super::host::ApplicationResources;
use super::protocol;

/// Configures shared renderer and platform capabilities before running an application.
pub struct ApplicationBuilder {
    renderer_factory: Box<dyn RendererFactory>,
    clipboard: ClipboardHandle,
    services: Services,
    exit_policy: ExitPolicy,
    protocol_schemes: Vec<ProtocolScheme>,
    launch_urls: Vec<ProtocolUrl>,
    diagnostics_capacity: usize,
    diagnostics_sink: Option<Arc<dyn DiagnosticsSink>>,
}

impl ApplicationBuilder {
    pub(super) fn new(renderer_factory: impl RendererFactory + 'static) -> Self {
        Self {
            renderer_factory: Box::new(renderer_factory),
            clipboard: ClipboardHandle::new(SystemClipboard),
            services: Services::system(),
            exit_policy: ExitPolicy::default(),
            protocol_schemes: Vec::new(),
            launch_urls: Vec::new(),
            diagnostics_capacity: 512,
            diagnostics_sink: None,
        }
    }

    /// Replaces the renderer factory used for subsequently opened windows.
    pub fn with_renderer(mut self, renderer_factory: impl RendererFactory + 'static) -> Self {
        self.renderer_factory = Box::new(renderer_factory);
        self
    }

    /// Replaces the application-wide text clipboard capability.
    pub fn with_clipboard(mut self, clipboard: impl Clipboard + 'static) -> Self {
        self.clipboard = ClipboardHandle::new(clipboard);
        self
    }

    /// Replaces the native file-dialog service exposed to application code.
    pub fn with_file_dialogs(mut self, service: impl FileDialogService + 'static) -> Self {
        self.services.replace_file_dialogs(service);
        self
    }

    /// Replaces the operating-system opener used for paths and external URLs.
    pub fn with_opener(mut self, service: impl OpenerService + 'static) -> Self {
        self.services.replace_opener(service);
        self
    }

    /// Replaces the desktop-notification service exposed to application code.
    pub fn with_notifications(mut self, service: impl NotificationService + 'static) -> Self {
        self.services.replace_notifications(service);
        self
    }

    /// Replaces the native application-menu service and event source.
    pub fn with_menus(mut self, service: impl MenuService + 'static) -> Self {
        self.services.replace_menus(service);
        self
    }

    /// Replaces the system-tray service and event source.
    pub fn with_tray(mut self, service: impl TrayService + 'static) -> Self {
        self.services.replace_tray(service);
        self
    }

    /// Replaces the global-shortcut service and event source.
    pub fn with_global_shortcuts(mut self, service: impl GlobalShortcutService + 'static) -> Self {
        self.services.replace_global_shortcuts(service);
        self
    }

    /// Replaces packaged-resource discovery for development, tests, or custom bundles.
    pub fn with_resources(mut self, service: impl ResourceService + 'static) -> Self {
        self.services.replace_resources(service);
        self
    }

    /// Uses an explicit packaged-resource root.
    pub fn with_resource_root(mut self, root: impl Into<std::path::PathBuf>) -> Self {
        self.services
            .replace_resources(SystemResourceLocator::from_root(root));
        self
    }

    /// Replaces shell-free child-process spawning for policy enforcement or tests.
    pub fn with_processes(mut self, service: impl ProcessService + 'static) -> Self {
        self.services.replace_processes(service);
        self
    }

    /// Replaces application update checking, download verification, and installer handoff.
    pub fn with_updates(mut self, service: impl UpdateService + 'static) -> Self {
        self.services.replace_updates(service);
        self
    }

    /// Sets the number of latest runtime events retained for diagnostics snapshots.
    pub const fn with_diagnostics_capacity(mut self, capacity: usize) -> Self {
        self.diagnostics_capacity = capacity;
        self
    }

    /// Streams diagnostic events to an application-provided observer.
    pub fn with_diagnostics_sink(mut self, sink: impl DiagnosticsSink + 'static) -> Self {
        self.diagnostics_sink = Some(Arc::new(sink));
        self
    }

    /// Selects whether closing the last window exits the event loop automatically.
    pub const fn with_exit_policy(mut self, exit_policy: ExitPolicy) -> Self {
        self.exit_policy = exit_policy;
        self
    }

    /// Accepts command-line launch URLs using one custom protocol scheme.
    pub fn with_protocol_scheme(mut self, scheme: ProtocolScheme) -> Self {
        if !self.protocol_schemes.contains(&scheme) {
            self.protocol_schemes.push(scheme);
        }
        self
    }

    /// Adds an explicit launch URL for platform forwarding or deterministic tests.
    pub fn with_launch_url(mut self, url: ProtocolUrl) -> Self {
        self.launch_urls.push(url);
        self
    }

    /// Runs the configured native application until the platform event loop exits.
    pub fn run<T, A, C>(self, create: C) -> Result<ApplicationExit<A>, ApplicationRunError>
    where
        T: Send + 'static,
        A: App<T> + 'static,
        C: FnOnce(ApplicationHandle<T>) -> A,
    {
        let renderer_factory = self.renderer_factory;
        let clipboard = self.clipboard;
        let services = self.services;
        let exit_policy = self.exit_policy;
        let mut launch_urls = self.launch_urls;
        let diagnostics = DiagnosticsHandle::new(self.diagnostics_capacity, self.diagnostics_sink);
        launch_urls.extend(protocol::urls_from_arguments(
            &self.protocol_schemes,
            std::env::args_os().skip(1),
        ));
        let host = run_application_with_user_events(move |runtime_proxy| {
            let event_proxy = AppProxy::new(runtime_proxy);
            let background = BackgroundExecutor::new(event_proxy.clone());
            let timers = TimerScheduler::new(event_proxy.clone());
            let handle = ApplicationHandle {
                event_proxy: event_proxy.clone(),
                clipboard: clipboard.clone(),
                services: services.clone(),
                background: background.clone(),
                timers: timers.clone(),
                diagnostics: diagnostics.clone(),
            };
            let resources = ApplicationResources {
                renderer_factory,
                clipboard,
                services,
                event_proxy,
                background,
                timers,
                launch_urls,
                diagnostics,
            };
            ApplicationHost::new(create(handle), resources, exit_policy)
        })?;
        Ok(ApplicationExit {
            app: host.app,
            error: host.error,
        })
    }
}
use std::sync::Arc;
