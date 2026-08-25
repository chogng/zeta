use winit::event;

use crate::input::KeyEvent;
use crate::input::Modifiers;
use crate::input::ModifiersState;

use super::PhysicalExtent;

/// Pressed or released state shared by keyboard and pointer input.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ElementState {
    Pressed,
    Released,
}

impl ElementState {
    pub(crate) const fn from_native(state: event::ElementState) -> Self {
        match state {
            event::ElementState::Pressed => Self::Pressed,
            event::ElementState::Released => Self::Released,
        }
    }
}

/// Physical coordinates reported by a window input device.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PhysicalPosition {
    pub x: f64,
    pub y: f64,
}

impl PhysicalPosition {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Mouse or pointing-device button identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

impl MouseButton {
    pub(crate) const fn from_native(button: event::MouseButton) -> Self {
        match button {
            event::MouseButton::Left => Self::Left,
            event::MouseButton::Right => Self::Right,
            event::MouseButton::Middle => Self::Middle,
            event::MouseButton::Back => Self::Back,
            event::MouseButton::Forward => Self::Forward,
            event::MouseButton::Other(button) => Self::Other(button),
        }
    }
}

/// Scrolling distance expressed in logical lines or physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MouseScrollDelta {
    LineDelta(f32, f32),
    PixelDelta(PhysicalPosition),
}

impl MouseScrollDelta {
    pub(crate) fn from_native(delta: event::MouseScrollDelta) -> Self {
        match delta {
            event::MouseScrollDelta::LineDelta(x, y) => Self::LineDelta(x, y),
            event::MouseScrollDelta::PixelDelta(position) => {
                Self::PixelDelta(PhysicalPosition::new(position.x, position.y))
            }
        }
    }
}

/// Input-method lifecycle and composition text reported by the platform.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Ime {
    Enabled,
    Preedit(String, Option<(usize, usize)>),
    Commit(String),
    Disabled,
}

impl Ime {
    pub(crate) fn from_native(event: event::Ime) -> Self {
        match event {
            event::Ime::Enabled => Self::Enabled,
            event::Ime::Preedit(text, selection) => Self::Preedit(text, selection),
            event::Ime::Commit(text) => Self::Commit(text),
            event::Ime::Disabled => Self::Disabled,
        }
    }
}

/// System light or dark appearance for a native window.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub(crate) const fn from_native(theme: winit::window::Theme) -> Self {
        match theme {
            winit::window::Theme::Light => Self::Light,
            winit::window::Theme::Dark => Self::Dark,
        }
    }

    pub(crate) const fn into_native(self) -> winit::window::Theme {
        match self {
            Self::Light => winit::window::Theme::Light,
            Self::Dark => winit::window::Theme::Dark,
        }
    }
}

/// Backend-independent window event delivered by [`crate::app::App`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WindowEvent {
    CloseRequested,
    Destroyed,
    Resized(PhysicalExtent),
    ScaleFactorChanged {
        scale_factor: f64,
    },
    ThemeChanged(Theme),
    CursorMoved {
        position: PhysicalPosition,
    },
    CursorLeft,
    ModifiersChanged(Modifiers),
    KeyboardInput {
        event: KeyEvent,
    },
    Ime(Ime),
    Focused(bool),
    MouseInput {
        state: ElementState,
        button: MouseButton,
    },
    MouseWheel {
        delta: MouseScrollDelta,
    },
    Occluded(bool),
    RedrawRequested,
    Other,
}

impl WindowEvent {
    pub(crate) fn from_native(event: event::WindowEvent) -> Self {
        match event {
            event::WindowEvent::CloseRequested => Self::CloseRequested,
            event::WindowEvent::Destroyed => Self::Destroyed,
            event::WindowEvent::Resized(size) => {
                Self::Resized(PhysicalExtent::new(size.width, size.height))
            }
            event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                Self::ScaleFactorChanged { scale_factor }
            }
            event::WindowEvent::ThemeChanged(theme) => {
                Self::ThemeChanged(Theme::from_native(theme))
            }
            event::WindowEvent::CursorMoved { position, .. } => Self::CursorMoved {
                position: PhysicalPosition::new(position.x, position.y),
            },
            event::WindowEvent::CursorLeft { .. } => Self::CursorLeft,
            event::WindowEvent::ModifiersChanged(modifiers) => Self::ModifiersChanged(
                Modifiers::new(ModifiersState::from_native(modifiers.state())),
            ),
            event::WindowEvent::KeyboardInput { event, .. } => Self::KeyboardInput {
                event: KeyEvent::from_native(event),
            },
            event::WindowEvent::Ime(event) => Self::Ime(Ime::from_native(event)),
            event::WindowEvent::Focused(focused) => Self::Focused(focused),
            event::WindowEvent::MouseInput { state, button, .. } => Self::MouseInput {
                state: ElementState::from_native(state),
                button: MouseButton::from_native(button),
            },
            event::WindowEvent::MouseWheel { delta, .. } => Self::MouseWheel {
                delta: MouseScrollDelta::from_native(delta),
            },
            event::WindowEvent::Occluded(occluded) => Self::Occluded(occluded),
            event::WindowEvent::RedrawRequested => Self::RedrawRequested,
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
#[path = "event_tests.rs"]
mod tests;
