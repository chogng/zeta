use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event_loop::{EventLoop, EventLoopProxy};

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

/// Creates a typed platform event loop and constructs a product handler with its wakeup proxy.
///
/// The product owns the user-event type and decides how background work is projected into its
/// main-thread state. Returning the handler lets the executable inspect product termination state
/// after the event loop exits.
pub fn run_application_with_user_events<T, A, F>(create_application: F) -> Result<A, EventLoopError>
where
    T: 'static,
    A: ApplicationHandler<T>,
    F: FnOnce(EventLoopProxy<T>) -> A,
{
    let event_loop = EventLoop::<T>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut application = create_application(proxy);
    event_loop.run_app(&mut application)?;
    Ok(application)
}
