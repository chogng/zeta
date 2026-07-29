use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event_loop::EventLoop;

/// Creates the platform event loop and runs a product-owned application handler.
///
/// The handler remains responsible for product state, window policy, commands, and rendering.
/// This function owns only the native event-loop bootstrap and returns platform termination
/// failures without translating them into product errors.
pub fn run_application<A>(application: &mut A) -> Result<(), EventLoopError>
where
    A: ApplicationHandler,
{
    EventLoop::new()?.run_app(application)
}
