//! Native window configuration, identity, events, and live capabilities.

mod chrome;
mod event;
mod native;
mod runtime;

pub use chrome::WindowChrome;
pub use chrome::WindowControlInsets;
pub use event::ElementState;
pub use event::Ime;
pub use event::MouseButton;
pub use event::MouseScrollDelta;
pub use event::PhysicalPosition;
pub use event::Theme;
pub use event::WindowEvent;
pub use native::CursorIcon;
pub use native::ImeCursorArea;
pub use native::LogicalSize;
pub use native::PhysicalExtent;
pub use native::RenderWindow;
pub use native::WindowHandle;
pub use native::WindowId;
pub use native::WindowOperationError;
pub use runtime::OpenedWindow;
pub use runtime::WindowMetrics;
pub use runtime::WindowOptions;

pub(crate) use native::NativeWindow;
pub(crate) use runtime::WindowRuntime;

/// Internal ownership role used to keep framework-owned utility windows out of product
/// lifecycle callbacks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WindowRole {
    Product,
    DevTools { owner: WindowId },
}
