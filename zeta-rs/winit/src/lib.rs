//! Native event-loop and window substrate for Rust products.

mod host;
mod window;

pub use host::run_application;
pub use window::NativeWindow;
pub use window::PhysicalExtent;

pub use winit::application::ApplicationHandler;
pub use winit::dpi::LogicalSize;
pub use winit::event::WindowEvent;
pub use winit::event_loop::ActiveEventLoop;
pub use winit::window::WindowAttributes;
pub use winit::window::WindowId;
