use std::error::Error;
use std::fmt;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event_loop::EventLoop;

/// Policy controlling when the application event loop wakes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlFlow {
    Poll,
    Wait,
    WaitUntil(Instant),
}

impl ControlFlow {
    pub(crate) const fn into_native(self) -> winit::event_loop::ControlFlow {
        match self {
            Self::Poll => winit::event_loop::ControlFlow::Poll,
            Self::Wait => winit::event_loop::ControlFlow::Wait,
            Self::WaitUntil(deadline) => winit::event_loop::ControlFlow::WaitUntil(deadline),
        }
    }
}

/// Internal cross-thread capability used by the application runtime.
pub(crate) struct NativeEventProxy<T: 'static> {
    inner: winit::event_loop::EventLoopProxy<T>,
}

impl<T: 'static> Clone for NativeEventProxy<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: 'static> fmt::Debug for NativeEventProxy<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NativeEventProxy { .. }")
    }
}

impl<T: 'static> NativeEventProxy<T> {
    pub(crate) const fn from_native(inner: winit::event_loop::EventLoopProxy<T>) -> Self {
        Self { inner }
    }

    /// Sends `event` to the application's main-thread event loop.
    pub(crate) fn send_event(&self, event: T) -> Result<(), NativeEventLoopClosed<T>> {
        self.inner
            .send_event(event)
            .map_err(|error| NativeEventLoopClosed(error.0))
    }

    pub(crate) fn native(&self) -> winit::event_loop::EventLoopProxy<T> {
        self.inner.clone()
    }
}

pub(crate) struct NativeEventLoopClosed<T>(pub(crate) T);

/// Failure to create or run the native application event loop.
#[derive(Debug)]
pub struct ApplicationRunError(winit::error::EventLoopError);

impl fmt::Display for ApplicationRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Error for ApplicationRunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

/// Creates a typed platform event loop and constructs a handler with its wakeup proxy.
///
/// The caller owns the user-event type. Returning the handler lets an application runtime inspect
/// its termination state after the event loop exits.
pub fn run_application_with_user_events<T, A, F>(
    create_application: F,
) -> Result<A, ApplicationRunError>
where
    T: 'static,
    A: ApplicationHandler<T>,
    F: FnOnce(NativeEventProxy<T>) -> A,
{
    let event_loop = EventLoop::<T>::with_user_event()
        .build()
        .map_err(ApplicationRunError)?;
    let proxy = NativeEventProxy::from_native(event_loop.create_proxy());
    let mut application = create_application(proxy);
    event_loop
        .run_app(&mut application)
        .map_err(ApplicationRunError)?;
    Ok(application)
}
