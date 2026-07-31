//! Native event-loop and window substrate for Rust products.

mod host;
mod window;
mod window_chrome;

pub use host::{run_application, run_application_with_user_events};
pub use window::{ImeCursorArea, NativeWindow, PhysicalExtent};
pub use window_chrome::{WindowChrome, WindowControlInsets};

pub use winit::application::ApplicationHandler;
pub use winit::dpi::LogicalSize;
pub use winit::dpi::PhysicalPosition;
pub use winit::event::ElementState;
pub use winit::event::Ime;
pub use winit::event::KeyEvent;
pub use winit::event::MouseButton;
pub use winit::event::MouseScrollDelta;
pub use winit::event::WindowEvent;
pub use winit::event_loop::ActiveEventLoop;
pub use winit::event_loop::ControlFlow;
pub use winit::event_loop::EventLoopProxy;
pub use winit::keyboard::{Key, ModifiersState, NamedKey, PhysicalKey};
pub use winit::window::CursorIcon;
pub use winit::window::Theme;
pub use winit::window::WindowAttributes;
pub use winit::window::WindowId;
