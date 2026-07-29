use std::sync::Arc;

use winit::error::OsError;
use winit::event_loop::{ActiveEventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowAttributes, WindowId};

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
