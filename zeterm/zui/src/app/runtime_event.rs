use std::error::Error;
use std::fmt;

use crate::internal::NativeEventLoopClosed;
use crate::internal::NativeEventProxy;

use super::GlobalShortcutEvent;
use super::MenuItemId;
use super::ProtocolUrl;
use super::TrayEvent;
use crate::runtime::timer::ScheduledTimer;
use crate::runtime::timer::TimerId;

pub(crate) enum RuntimeEvent<T: 'static> {
    Product(T),
    ScheduleTimer(ScheduledTimer<T>),
    CancelTimer(TimerId),
    MenuAction(MenuItemId),
    Tray(TrayEvent),
    GlobalShortcut(GlobalShortcutEvent),
    OpenUrl(ProtocolUrl),
    Accessibility(accesskit_platform::Event),
    DevToolsWake,
}

impl<T: 'static> From<accesskit_platform::Event> for RuntimeEvent<T> {
    fn from(event: accesskit_platform::Event) -> Self {
        Self::Accessibility(event)
    }
}

/// Cloneable cross-thread capability for delivering application-defined events.
pub struct AppProxy<T: 'static> {
    pub(crate) inner: NativeEventProxy<RuntimeEvent<T>>,
}

impl<T: 'static> Clone for AppProxy<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: 'static> fmt::Debug for AppProxy<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AppProxy { .. }")
    }
}

impl<T: 'static> AppProxy<T> {
    pub(crate) const fn new(inner: NativeEventProxy<RuntimeEvent<T>>) -> Self {
        Self { inner }
    }

    /// Sends `event` to the application's main-thread event loop.
    pub fn send_event(&self, event: T) -> Result<(), AppDisconnected<T>> {
        self.inner
            .send_event(RuntimeEvent::Product(event))
            .map_err(|error| match error {
                NativeEventLoopClosed(RuntimeEvent::Product(event)) => AppDisconnected(event),
                NativeEventLoopClosed(
                    RuntimeEvent::ScheduleTimer(_)
                    | RuntimeEvent::CancelTimer(_)
                    | RuntimeEvent::MenuAction(_)
                    | RuntimeEvent::Tray(_)
                    | RuntimeEvent::GlobalShortcut(_)
                    | RuntimeEvent::OpenUrl(_)
                    | RuntimeEvent::Accessibility(_)
                    | RuntimeEvent::DevToolsWake,
                ) => unreachable!("product event delivery must retain the product event"),
            })
    }

    /// Forwards an application URL to the main-thread lifecycle handler.
    pub fn send_open_url(&self, url: ProtocolUrl) -> Result<(), AppDisconnected<ProtocolUrl>> {
        self.inner
            .send_event(RuntimeEvent::OpenUrl(url))
            .map_err(|error| match error {
                NativeEventLoopClosed(RuntimeEvent::OpenUrl(url)) => AppDisconnected(url),
                NativeEventLoopClosed(
                    RuntimeEvent::Product(_)
                    | RuntimeEvent::ScheduleTimer(_)
                    | RuntimeEvent::CancelTimer(_)
                    | RuntimeEvent::MenuAction(_)
                    | RuntimeEvent::Tray(_)
                    | RuntimeEvent::GlobalShortcut(_)
                    | RuntimeEvent::Accessibility(_)
                    | RuntimeEvent::DevToolsWake,
                ) => unreachable!("application URL delivery must retain the URL"),
            })
    }
}

/// Failed event delivery after the owning application loop has exited.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AppDisconnected<T>(pub T);

impl<T> fmt::Display for AppDisconnected<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("cannot deliver an event to an application that has exited")
    }
}

impl<T: fmt::Debug> Error for AppDisconnected<T> {}
