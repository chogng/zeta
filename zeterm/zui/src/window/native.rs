use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::sync::Weak;

use raw_window_handle::DisplayHandle;
use raw_window_handle::HandleError;
use raw_window_handle::HasDisplayHandle;
use raw_window_handle::HasWindowHandle;
use raw_window_handle::WindowHandle as RawWindowHandle;
use winit::dpi::LogicalPosition;
use winit::event_loop::ActiveEventLoop;
#[cfg(feature = "wgpu")]
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;
use winit::window::WindowAttributes;

use crate::devtools::DevToolsHandle;
use crate::devtools::DevToolsRequestSender;

use super::Theme;
use super::WindowChrome;
use super::WindowControlInsets;
use super::chrome::apply_window_chrome;
use super::chrome::window_control_insets;

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
    const fn into_native(self) -> winit::window::CursorIcon {
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

/// Failure while applying a live native-window operation.
#[derive(Debug)]
pub struct WindowOperationError {
    operation: &'static str,
    source: Box<dyn Error + Send + Sync>,
}

impl fmt::Display for WindowOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} failed: {}", self.operation, self.source)
    }
}

impl Error for WindowOperationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
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

/// Non-owning platform capability for updating a live native window.
///
/// Application runtimes retain canonical window ownership. Product state may keep this handle to
/// request redraws or forward UI decisions without extending the native window lifecycle.
#[derive(Clone)]
pub struct WindowHandle {
    window: Weak<Window>,
    chrome: WindowChrome,
    devtools: DevToolsHandle,
}

impl WindowHandle {
    /// Returns the stable identity of the live window, if it still exists.
    pub fn id(&self) -> Option<WindowId> {
        self.window
            .upgrade()
            .map(|window| WindowId::from_native(window.id()))
    }

    /// Schedules a redraw when the runtime still owns the window.
    pub fn request_redraw(&self) {
        if let Some(window) = self.window.upgrade() {
            window.request_redraw();
        }
    }

    /// Returns the shared DevTools session capability for this window.
    pub fn devtools(&self) -> DevToolsHandle {
        self.devtools.clone()
    }

    /// Opens the default zui DevTools window for this window and schedules a frame.
    pub fn open_devtools(&self) {
        self.devtools.open();
        self.request_redraw();
    }

    /// Closes the default zui DevTools window for this window and schedules a frame.
    pub fn close_devtools(&self) {
        self.devtools.close();
        self.request_redraw();
    }

    /// Toggles DevTools for this window and returns whether it is now open.
    pub fn toggle_devtools(&self) -> bool {
        let is_open = self.devtools.toggle();
        self.request_redraw();
        is_open
    }

    /// Returns whether DevTools is currently open for this window.
    pub fn is_devtools_open(&self) -> bool {
        self.devtools.is_open()
    }

    /// Begins a platform window drag when the runtime still owns the window.
    pub fn start_window_drag(&self) -> Result<(), WindowOperationError> {
        if let Some(window) = self.window.upgrade() {
            window
                .drag_window()
                .map_err(|source| WindowOperationError {
                    operation: "window drag",
                    source: Box::new(source),
                })?;
        }
        Ok(())
    }

    /// Updates the pointer cursor when the runtime still owns the window.
    pub fn set_cursor(&self, cursor: CursorIcon) {
        if let Some(window) = self.window.upgrade() {
            window.set_cursor(cursor.into_native());
        }
    }

    /// Updates the platform window title when the runtime still owns the window.
    pub fn set_title(&self, title: &str) {
        if let Some(window) = self.window.upgrade() {
            window.set_title(title);
        }
    }

    /// Requests a new logical inner size when the runtime still owns the window.
    pub fn request_inner_logical_size(&self, size: LogicalSize) {
        if let Some(window) = self.window.upgrade() {
            let _ = window.request_inner_size(size.into_native());
        }
    }

    /// Returns the current platform theme preference for a live window.
    pub fn theme(&self) -> Option<Theme> {
        self.window
            .upgrade()
            .and_then(|window| window.theme())
            .map(Theme::from_native)
    }

    /// Applies an explicit platform theme to a live window.
    pub fn set_theme(&self, theme: Option<Theme>) {
        if let Some(window) = self.window.upgrade() {
            window.set_theme(theme.map(Theme::into_native));
        }
    }

    /// Enables platform text input when the runtime still owns the window.
    pub fn enable_ime(&self) {
        if let Some(window) = self.window.upgrade() {
            window.set_ime_allowed(true);
        }
    }

    /// Disables platform text input when the runtime still owns the window.
    pub fn disable_ime(&self) {
        if let Some(window) = self.window.upgrade() {
            window.set_ime_allowed(false);
        }
    }

    /// Updates the IME candidate area when the runtime still owns the window.
    pub fn set_ime_cursor_area(&self, area: ImeCursorArea) {
        if let Some(window) = self.window.upgrade() {
            window.set_ime_cursor_area(
                LogicalPosition::new(area.x, area.y),
                winit::dpi::LogicalSize::new(area.width, area.height),
            );
        }
    }

    /// Returns logical insets occupied by native controls for this window's chrome policy.
    pub fn window_control_insets(&self) -> WindowControlInsets {
        window_control_insets(self.chrome)
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn native_hwnd(&self) -> Option<isize> {
        let window = self.window.upgrade()?;
        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            raw_window_handle::RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
            _ => None,
        }
    }
}

impl NativeWindow {
    /// Creates a native window from ZUI-owned options and a named chrome policy.
    pub(crate) fn create(
        event_loop: &ActiveEventLoop,
        title: String,
        inner_size: Option<LogicalSize>,
        chrome: WindowChrome,
        request_sender: DevToolsRequestSender,
    ) -> Result<Self, winit::error::OsError> {
        let mut attributes = WindowAttributes::default().with_title(title);
        if let Some(inner_size) = inner_size {
            attributes = attributes.with_inner_size(inner_size.into_native());
        }
        let attributes = apply_window_chrome(attributes, chrome).with_visible(false);
        let window = Arc::new(event_loop.create_window(attributes)?);
        let owner = WindowId::from_native(window.id());
        Ok(Self {
            window,
            #[cfg(feature = "wgpu")]
            display_handle: event_loop.owned_display_handle(),
            chrome,
            devtools: DevToolsHandle::with_request(owner, request_sender),
        })
    }

    /// Returns the stable identity used to route native window events.
    pub(crate) fn id(&self) -> WindowId {
        WindowId::from_native(self.window.id())
    }

    /// Creates a non-owning product capability without sharing window lifecycle ownership.
    pub(crate) fn handle(&self) -> WindowHandle {
        WindowHandle {
            window: Arc::downgrade(&self.window),
            chrome: self.chrome,
            devtools: self.devtools.clone(),
        }
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
