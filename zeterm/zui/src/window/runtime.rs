use crate::accessibility::AccessibilityAction;
use crate::accessibility::AccessibilityBridge;
use crate::app::AppProxy;
use crate::app::ApplicationError;
use crate::devtools::DevToolsRequestSender;
use crate::internal::ActiveEventLoop;
use crate::render::RenderOutcome;
use crate::render::RenderTargetSize;
use crate::render::Renderer;
use crate::render::RendererError;
use crate::render::RendererFactory;
use crate::runtime::AccessibilityNode;
use crate::ui::foundation::Size;
use crate::ui::presentation::UiScene;
use crate::window::LogicalSize;
use crate::window::NativeWindow;
use crate::window::PhysicalExtent;
use crate::window::WindowChrome;
use crate::window::WindowEvent;
use crate::window::WindowHandle;
use crate::window::WindowId;
use crate::window::WindowRole;
use thiserror::Error;

/// Invalid policy supplied while configuring a native window.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum WindowOptionsError {
    /// One configured size was non-finite, zero, or negative.
    #[error("{field} must contain finite, positive dimensions")]
    InvalidSize { field: &'static str },
    /// The minimum size exceeded the maximum along at least one axis.
    #[error("minimum inner size must not exceed maximum inner size")]
    InvalidSizeRange,
}

/// Native window creation policy supplied by an application.
#[derive(Debug)]
pub struct WindowOptions {
    pub(crate) title: String,
    pub(crate) inner_size: Option<LogicalSize>,
    pub(crate) min_inner_size: Option<LogicalSize>,
    pub(crate) max_inner_size: Option<LogicalSize>,
    pub(crate) chrome: WindowChrome,
    pub(crate) visible: bool,
    pub(crate) active: bool,
    pub(crate) resizable: bool,
    pub(crate) maximized: bool,
    pub(crate) fullscreen: bool,
}

impl WindowOptions {
    /// Creates a native window request with platform chrome and the supplied title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            inner_size: None,
            min_inner_size: None,
            max_inner_size: None,
            chrome: WindowChrome::Native,
            visible: true,
            active: true,
            resizable: true,
            maximized: false,
            fullscreen: false,
        }
    }

    /// Replaces the title shown by the native window system.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Selects the relationship between native chrome and product content.
    pub const fn with_chrome(mut self, chrome: WindowChrome) -> Self {
        self.chrome = chrome;
        self
    }

    /// Sets the requested logical inner size.
    pub const fn with_inner_size(mut self, size: LogicalSize) -> Self {
        self.inner_size = Some(size);
        self
    }

    /// Sets the minimum logical content size accepted by the platform.
    pub const fn with_min_inner_size(mut self, size: LogicalSize) -> Self {
        self.min_inner_size = Some(size);
        self
    }

    /// Sets the maximum logical content size accepted by the platform.
    pub const fn with_max_inner_size(mut self, size: LogicalSize) -> Self {
        self.max_inner_size = Some(size);
        self
    }

    /// Selects whether the window is shown after renderer and accessibility setup completes.
    pub const fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Selects whether opening the window should request platform activation.
    pub const fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Selects whether the user can resize the window.
    pub const fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Selects whether the window opens maximized.
    pub const fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    /// Selects whether the window opens in borderless fullscreen mode.
    pub const fn with_fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    /// Validates size invariants before native resources are allocated.
    pub fn validate(&self) -> Result<(), WindowOptionsError> {
        for (field, size) in [
            ("inner size", self.inner_size),
            ("minimum inner size", self.min_inner_size),
            ("maximum inner size", self.max_inner_size),
        ] {
            if size.is_some_and(|size| !size.is_valid()) {
                return Err(WindowOptionsError::InvalidSize { field });
            }
        }
        if let (Some(min), Some(max)) = (self.min_inner_size, self.max_inner_size)
            && (min.width > max.width || min.height > max.height)
        {
            return Err(WindowOptionsError::InvalidSizeRange);
        }
        Ok(())
    }
}

/// Current platform dimensions for one runtime-owned window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowMetrics {
    physical_extent: PhysicalExtent,
    scale_factor: f64,
}

impl WindowMetrics {
    pub(crate) const fn new(physical_extent: PhysicalExtent, scale_factor: f64) -> Self {
        Self {
            physical_extent,
            scale_factor,
        }
    }

    /// Returns the current physical render-target dimensions.
    pub const fn physical_extent(self) -> PhysicalExtent {
        self.physical_extent
    }

    /// Returns the validated logical-to-physical scale factor.
    pub fn scale_factor(self) -> f64 {
        if self.scale_factor.is_finite() && self.scale_factor > 0.0 {
            self.scale_factor
        } else {
            1.0
        }
    }

    /// Returns the logical content size derived from the current physical dimensions.
    pub fn logical_size(self) -> Size {
        let scale_factor = self.scale_factor() as f32;
        Size::new(
            self.physical_extent.width as f32 / scale_factor,
            self.physical_extent.height as f32 / scale_factor,
        )
    }
}

/// Identity, non-owning handle, and initial metrics returned after opening a window.
#[derive(Clone)]
pub struct OpenedWindow {
    id: WindowId,
    handle: WindowHandle,
    metrics: WindowMetrics,
}

impl OpenedWindow {
    /// Returns the stable platform window identity.
    pub const fn id(&self) -> WindowId {
        self.id
    }

    /// Returns a non-owning capability for product-directed platform updates.
    pub fn handle(&self) -> WindowHandle {
        self.handle.clone()
    }

    /// Returns metrics captured when the window was opened.
    pub const fn metrics(&self) -> WindowMetrics {
        self.metrics
    }
}

pub(crate) struct WindowRuntime {
    window: NativeWindow,
    renderer: Box<dyn Renderer>,
    metrics: WindowMetrics,
    accessibility: AccessibilityBridge,
    role: WindowRole,
}

impl WindowRuntime {
    pub(crate) fn open<T: Send + 'static>(
        event_loop: &ActiveEventLoop,
        renderer_factory: &mut dyn RendererFactory,
        proxy: &AppProxy<T>,
        options: WindowOptions,
        role: WindowRole,
        request_sender: DevToolsRequestSender,
    ) -> Result<Self, ApplicationError> {
        options
            .validate()
            .map_err(ApplicationError::window_options)?;
        let title = options.title.clone();
        let initially_visible = options.visible;
        let window = NativeWindow::create(event_loop, options, request_sender)
            .map_err(ApplicationError::window)?;
        let metrics = WindowMetrics::new(window.inner_extent(), window.scale_factor());
        let accessibility =
            AccessibilityBridge::new(event_loop, &window, proxy, title, metrics.scale_factor());
        let mut renderer = renderer_factory
            .create(window.render_window())
            .map_err(ApplicationError::renderer)?;
        renderer.resize(RenderTargetSize::new(
            metrics.physical_extent.width,
            metrics.physical_extent.height,
        ));
        renderer.set_scale_factor(metrics.scale_factor());
        if initially_visible {
            window.show();
        }
        Ok(Self {
            window,
            renderer,
            metrics,
            accessibility,
            role,
        })
    }

    pub(crate) fn opened_window(&self) -> OpenedWindow {
        OpenedWindow {
            id: self.window.id(),
            handle: self.window.handle(),
            metrics: self.metrics,
        }
    }

    pub(crate) fn id(&self) -> WindowId {
        self.window.id()
    }

    pub(crate) fn metrics(&self) -> WindowMetrics {
        self.metrics
    }

    pub(crate) fn handle(&self) -> WindowHandle {
        self.window.handle()
    }

    pub(crate) fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub(crate) const fn role(&self) -> WindowRole {
        self.role
    }

    pub(crate) fn apply_platform_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Resized(size) => {
                self.metrics.physical_extent = PhysicalExtent::new(size.width, size.height);
                self.renderer
                    .resize(RenderTargetSize::new(size.width, size.height));
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.metrics.scale_factor = *scale_factor;
                self.renderer.set_scale_factor(*scale_factor);
            }
            _ => {}
        }
    }

    pub(crate) fn process_accessibility_window_event(
        &mut self,
        event: &crate::internal::NativeWindowEvent,
    ) {
        self.accessibility.process_window_event(&self.window, event);
    }

    pub(crate) fn handle_accessibility_event(
        &mut self,
        event: accesskit_platform::WindowEvent,
    ) -> Option<AccessibilityAction> {
        self.accessibility.handle_event(self.id(), event)
    }

    pub(crate) fn update_accessibility(&mut self, nodes: &[AccessibilityNode]) {
        self.accessibility
            .update(nodes, self.metrics.scale_factor());
    }

    pub(crate) fn render_scene(&mut self, scene: &UiScene) -> Result<RenderOutcome, RendererError> {
        let outcome = self.renderer.render_scene(scene)?;
        if outcome == RenderOutcome::Retry {
            self.window.request_redraw();
        }
        Ok(outcome)
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
