use std::sync::Arc;

use raw_window_handle::DisplayHandle;
use raw_window_handle::HandleError;
use raw_window_handle::HasDisplayHandle;
use raw_window_handle::HasWindowHandle;
use raw_window_handle::WindowHandle as RawWindowHandle;
use winit::event_loop::ActiveEventLoop;
#[cfg(feature = "wgpu")]
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;
use winit::window::WindowAttributes;

use crate::devtools::DevToolsHandle;
use crate::devtools::DevToolsRequestSender;

use super::WindowChrome;
use super::WindowHandle;
use super::WindowOptions;
use super::chrome::apply_window_chrome;

/// Logical dimensions used to configure and resize native windows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalSize {
    pub width: f64,
    pub height: f64,
}

impl LogicalSize {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Returns whether both dimensions are finite and strictly positive.
    pub fn is_valid(self) -> bool {
        self.width.is_finite() && self.width > 0.0 && self.height.is_finite() && self.height > 0.0
    }

    pub(crate) const fn into_native(self) -> winit::dpi::LogicalSize<f64> {
        winit::dpi::LogicalSize::new(self.width, self.height)
    }
}

/// Stable runtime identity for one ZUI-owned window.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WindowId(u64);

impl WindowId {
    /// Creates a stable identity from its packed platform representation.
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the packed platform representation.
    pub const fn into_raw(self) -> u64 {
        self.0
    }

    pub(crate) fn from_native(id: winit::window::WindowId) -> Self {
        Self(id.into())
    }
}

/// Pointer cursor selected by application-owned hit testing.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CursorIcon {
    #[default]
    Default,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    EResize,
    NResize,
    NeResize,
    NwResize,
    SResize,
    SeResize,
    SwResize,
    WResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
    AllScroll,
    ZoomIn,
    ZoomOut,
}

impl CursorIcon {
    pub(crate) const fn into_native(self) -> winit::window::CursorIcon {
        match self {
            Self::Default => winit::window::CursorIcon::Default,
            Self::ContextMenu => winit::window::CursorIcon::ContextMenu,
            Self::Help => winit::window::CursorIcon::Help,
            Self::Pointer => winit::window::CursorIcon::Pointer,
            Self::Progress => winit::window::CursorIcon::Progress,
            Self::Wait => winit::window::CursorIcon::Wait,
            Self::Cell => winit::window::CursorIcon::Cell,
            Self::Crosshair => winit::window::CursorIcon::Crosshair,
            Self::Text => winit::window::CursorIcon::Text,
            Self::VerticalText => winit::window::CursorIcon::VerticalText,
            Self::Alias => winit::window::CursorIcon::Alias,
            Self::Copy => winit::window::CursorIcon::Copy,
            Self::Move => winit::window::CursorIcon::Move,
            Self::NoDrop => winit::window::CursorIcon::NoDrop,
            Self::NotAllowed => winit::window::CursorIcon::NotAllowed,
            Self::Grab => winit::window::CursorIcon::Grab,
            Self::Grabbing => winit::window::CursorIcon::Grabbing,
            Self::EResize => winit::window::CursorIcon::EResize,
            Self::NResize => winit::window::CursorIcon::NResize,
            Self::NeResize => winit::window::CursorIcon::NeResize,
            Self::NwResize => winit::window::CursorIcon::NwResize,
            Self::SResize => winit::window::CursorIcon::SResize,
            Self::SeResize => winit::window::CursorIcon::SeResize,
            Self::SwResize => winit::window::CursorIcon::SwResize,
            Self::WResize => winit::window::CursorIcon::WResize,
            Self::EwResize => winit::window::CursorIcon::EwResize,
            Self::NsResize => winit::window::CursorIcon::NsResize,
            Self::NeswResize => winit::window::CursorIcon::NeswResize,
            Self::NwseResize => winit::window::CursorIcon::NwseResize,
            Self::ColResize => winit::window::CursorIcon::ColResize,
            Self::RowResize => winit::window::CursorIcon::RowResize,
            Self::AllScroll => winit::window::CursorIcon::AllScroll,
            Self::ZoomIn => winit::window::CursorIcon::ZoomIn,
            Self::ZoomOut => winit::window::CursorIcon::ZoomOut,
        }
    }
}

/// Physical pixel extent reported by the native window system.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalExtent {
    pub width: u32,
    pub height: u32,
}

impl PhysicalExtent {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Logical window coordinates used to position the platform IME candidate UI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImeCursorArea {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ImeCursorArea {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Owns a native window together with the persistent display handle required by render backends.
///
/// Application runtimes create this value after `ApplicationHandler::resumed`. Render backends
/// clone the underlying handles through dedicated integration methods; product state and event
/// routing remain outside this type.
#[derive(Clone)]
pub(crate) struct NativeWindow {
    window: Arc<Window>,
    #[cfg(feature = "wgpu")]
    display_handle: OwnedDisplayHandle,
    chrome: WindowChrome,
    devtools: DevToolsHandle,
}

impl NativeWindow {
    /// Creates a native window from ZUI-owned options and a named chrome policy.
    pub(crate) fn create(
        event_loop: &ActiveEventLoop,
        options: WindowOptions,
        request_sender: DevToolsRequestSender,
    ) -> Result<Self, winit::error::OsError> {
        let mut attributes = WindowAttributes::default()
            .with_title(options.title)
            .with_active(options.active)
            .with_resizable(options.resizable)
            .with_maximized(options.maximized)
            .with_fullscreen(
                options
                    .fullscreen
                    .then_some(winit::window::Fullscreen::Borderless(None)),
            );
        if let Some(inner_size) = options.inner_size {
            attributes = attributes.with_inner_size(inner_size.into_native());
        }
        if let Some(min_inner_size) = options.min_inner_size {
            attributes = attributes.with_min_inner_size(min_inner_size.into_native());
        }
        if let Some(max_inner_size) = options.max_inner_size {
            attributes = attributes.with_max_inner_size(max_inner_size.into_native());
        }
        let attributes = apply_window_chrome(attributes, options.chrome).with_visible(false);
        let window = Arc::new(event_loop.create_window(attributes)?);
        let owner = WindowId::from_native(window.id());
        Ok(Self {
            window,
            #[cfg(feature = "wgpu")]
            display_handle: event_loop.owned_display_handle(),
            chrome: options.chrome,
            devtools: DevToolsHandle::with_request(owner, request_sender),
        })
    }

    /// Returns the stable identity used to route native window events.
    pub(crate) fn id(&self) -> WindowId {
        WindowId::from_native(self.window.id())
    }

    /// Creates a non-owning product capability without sharing window lifecycle ownership.
    pub(crate) fn handle(&self) -> WindowHandle {
        WindowHandle::new(
            self.id(),
            Arc::downgrade(&self.window),
            self.chrome,
            self.devtools.clone(),
        )
    }

    /// Returns the current physical pixel extent.
    pub(crate) fn inner_extent(&self) -> PhysicalExtent {
        let size = self.window.inner_size();
        PhysicalExtent::new(size.width, size.height)
    }

    /// Requests a new logical inner size while leaving product layout policy with the host.
    /// Returns the current logical-to-physical scale factor.
    pub(crate) fn scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }

    /// Returns the platform's current light or dark window preference when available.
    /// Schedules a redraw request through the platform event loop.
    pub(crate) fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub(crate) fn show(&self) {
        self.window.set_visible(true);
    }

    pub(crate) fn accessibility_window(&self) -> &Window {
        &self.window
    }

    /// Begins a platform window drag in response to a primary-button press in product chrome.
    /// Notifies the platform immediately before a rendered frame is presented.
    #[cfg(feature = "wgpu")]
    pub(crate) fn pre_present_notify(&self) {
        self.window.pre_present_notify();
    }

    /// Clones the window target used to create a graphics surface.
    #[cfg(feature = "wgpu")]
    pub(crate) fn surface_target(&self) -> Arc<Window> {
        self.window.clone()
    }

    /// Clones the persistent display handle used to initialize graphics APIs.
    #[cfg(feature = "wgpu")]
    pub(crate) fn display_handle(&self) -> OwnedDisplayHandle {
        self.display_handle.clone()
    }

    pub(crate) fn render_window(&self) -> RenderWindow {
        RenderWindow {
            window: self.clone(),
        }
    }
}

/// Opaque native presentation target passed to custom renderer factories.
///
/// Graphics backends use the standard [`HasWindowHandle`] and [`HasDisplayHandle`] contracts;
/// application code never receives the concrete window-system implementation.
#[derive(Clone)]
pub struct RenderWindow {
    window: NativeWindow,
}

impl RenderWindow {
    #[cfg(feature = "wgpu")]
    pub(crate) fn native(&self) -> &NativeWindow {
        &self.window
    }
}

impl HasWindowHandle for RenderWindow {
    fn window_handle(&self) -> Result<RawWindowHandle<'_>, HandleError> {
        self.window.window.window_handle()
    }
}

impl HasDisplayHandle for RenderWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.window.window.display_handle()
    }
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
