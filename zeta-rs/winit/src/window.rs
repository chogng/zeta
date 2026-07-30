use std::sync::Arc;

use winit::dpi::{LogicalPosition, LogicalSize};
use winit::error::{ExternalError, OsError};
use winit::event_loop::{ActiveEventLoop, OwnedDisplayHandle};
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId};

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
/// Product hosts create this value after `ApplicationHandler::resumed`. Render backends clone the
/// underlying handles through the dedicated integration methods; product state and event routing
/// remain outside this type.
#[derive(Clone)]
pub struct NativeWindow {
    window: Arc<Window>,
    display_handle: OwnedDisplayHandle,
}

impl NativeWindow {
    /// Creates a native window from product-owned attributes.
    pub fn create(
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
    ) -> Result<Self, OsError> {
        let window = Arc::new(event_loop.create_window(attributes)?);
        Ok(Self {
            window,
            display_handle: event_loop.owned_display_handle(),
        })
    }

    /// Returns the stable identity used to route native window events.
    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Returns the current physical pixel extent.
    pub fn inner_extent(&self) -> PhysicalExtent {
        let size = self.window.inner_size();
        PhysicalExtent::new(size.width, size.height)
    }

    /// Returns the current logical-to-physical scale factor.
    pub fn scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }

    /// Schedules a redraw request through the platform event loop.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Begins a platform window drag in response to a primary-button press in product chrome.
    pub fn start_window_drag(&self) -> Result<(), ExternalError> {
        self.window.drag_window()
    }

    /// Updates the pointer cursor requested by product-owned hit testing.
    pub fn set_cursor(&self, cursor: CursorIcon) {
        self.window.set_cursor(cursor);
    }

    /// Updates the platform window title from product-owned session state.
    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    /// Enables platform text input and IME events for the focused editable control.
    pub fn enable_ime(&self) {
        self.window.set_ime_allowed(true);
    }

    /// Disables platform text input and IME events when no editable control is focused.
    pub fn disable_ime(&self) {
        self.window.set_ime_allowed(false);
    }

    /// Updates the logical caret area used to place platform IME candidate UI.
    pub fn set_ime_cursor_area(&self, area: ImeCursorArea) {
        self.window.set_ime_cursor_area(
            LogicalPosition::new(area.x, area.y),
            LogicalSize::new(area.width, area.height),
        );
    }

    /// Notifies the platform immediately before a rendered frame is presented.
    pub fn pre_present_notify(&self) {
        self.window.pre_present_notify();
    }

    /// Clones the window target used to create a graphics surface.
    pub fn surface_target(&self) -> Arc<Window> {
        self.window.clone()
    }

    /// Clones the persistent display handle used to initialize graphics APIs.
    pub fn display_handle(&self) -> OwnedDisplayHandle {
        self.display_handle.clone()
    }
}

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;
