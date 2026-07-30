use anyhow::Result;
use zeta_terminal::{
    MouseModifiers, MouseTrackingMode, ScreenBuffer, TerminalCore, TerminalMouseButton,
    TerminalMouseButtonState, TerminalMouseEvent, TerminalMousePosition,
};
use zeta_winit::{ElementState, ModifiersState, MouseButton, MouseScrollDelta};

use crate::NativeApp;
use crate::shell_scene::terminal_mouse_position_for_viewport;
use crate::terminal_session::TerminalSession;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PointerInput {
    NotApplicable,
    Consumed(Vec<u8>),
}

#[derive(Default)]
pub(crate) struct TerminalPointer {
    held_button: TerminalMouseButtonState,
    last_position: Option<TerminalMousePosition>,
}

impl TerminalPointer {
    pub(crate) fn route_moved(
        &mut self,
        terminal: &mut TerminalSession,
        position: Option<TerminalMousePosition>,
        modifiers: ModifiersState,
    ) -> Result<bool> {
        let input = self.moved(terminal.core(), position, modifiers);
        send_pointer_input(terminal, input)
    }

    pub(crate) fn route_button(
        &mut self,
        terminal: &mut TerminalSession,
        position: Option<TerminalMousePosition>,
        button: MouseButton,
        state: ElementState,
        modifiers: ModifiersState,
    ) -> Result<bool> {
        let input = self.button_changed(terminal.core(), position, button, state, modifiers);
        send_pointer_input(terminal, input)
    }

    pub(crate) fn route_wheel(
        &mut self,
        terminal: &mut TerminalSession,
        position: Option<TerminalMousePosition>,
        delta: MouseScrollDelta,
        modifiers: ModifiersState,
    ) -> Result<bool> {
        let input = self.wheel(terminal.core(), position, delta, modifiers);
        send_pointer_input(terminal, input)
    }

    pub(crate) fn moved(
        &mut self,
        terminal: &TerminalCore,
        position: Option<TerminalMousePosition>,
        modifiers: ModifiersState,
    ) -> PointerInput {
        if !captures_pointer(terminal) {
            self.held_button = TerminalMouseButtonState::None;
            return PointerInput::NotApplicable;
        }
        let Some(position) = position else {
            return if self.held_button == TerminalMouseButtonState::None {
                PointerInput::NotApplicable
            } else {
                PointerInput::Consumed(Vec::new())
            };
        };
        self.last_position = Some(position);
        let event = TerminalMouseEvent::motion(self.held_button, position)
            .with_modifiers(mouse_modifiers(modifiers));
        PointerInput::Consumed(terminal.encode_mouse(event))
    }

    pub(crate) fn button_changed(
        &mut self,
        terminal: &TerminalCore,
        position: Option<TerminalMousePosition>,
        button: MouseButton,
        state: ElementState,
        modifiers: ModifiersState,
    ) -> PointerInput {
        let Some(button) = terminal_button(button) else {
            return PointerInput::NotApplicable;
        };
        if !captures_pointer(terminal) {
            self.held_button = TerminalMouseButtonState::None;
            return PointerInput::NotApplicable;
        }
        let event = match state {
            ElementState::Pressed => {
                let Some(position) = position else {
                    return PointerInput::NotApplicable;
                };
                self.last_position = Some(position);
                self.held_button = button_state(button);
                TerminalMouseEvent::press(button, position)
            }
            ElementState::Released => {
                let position = match position {
                    Some(position) => position,
                    None if self.held_button != TerminalMouseButtonState::None => {
                        let Some(position) = self.last_position else {
                            return PointerInput::NotApplicable;
                        };
                        position
                    }
                    None => return PointerInput::NotApplicable,
                };
                self.last_position = Some(position);
                self.held_button = TerminalMouseButtonState::None;
                TerminalMouseEvent::release(button, position)
            }
        }
        .with_modifiers(mouse_modifiers(modifiers));
        PointerInput::Consumed(terminal.encode_mouse(event))
    }

    pub(crate) fn wheel(
        &mut self,
        terminal: &TerminalCore,
        position: Option<TerminalMousePosition>,
        delta: MouseScrollDelta,
        modifiers: ModifiersState,
    ) -> PointerInput {
        if !captures_pointer(terminal) {
            self.held_button = TerminalMouseButtonState::None;
            return PointerInput::NotApplicable;
        }
        let Some(position) = position else {
            return PointerInput::NotApplicable;
        };
        let Some(button) = wheel_button(delta) else {
            return PointerInput::Consumed(Vec::new());
        };
        self.last_position = Some(position);
        let event =
            TerminalMouseEvent::press(button, position).with_modifiers(mouse_modifiers(modifiers));
        PointerInput::Consumed(terminal.encode_mouse(event))
    }

    pub(crate) fn cancel(&mut self) {
        self.held_button = TerminalMouseButtonState::None;
        self.last_position = None;
    }
}

impl NativeApp {
    pub(crate) fn terminal_mouse_position(
        &self,
        point: zeta_ui::Point,
    ) -> Option<TerminalMousePosition> {
        terminal_mouse_position_for_viewport(
            self.logical_viewport(),
            self.active_screen(),
            self.session_sidebar,
            point,
        )
    }

    pub(super) fn route_terminal_pointer_move(
        &mut self,
        position: Option<TerminalMousePosition>,
    ) -> bool {
        let Some(terminal) = self.terminal.as_mut() else {
            return false;
        };
        match self
            .terminal_pointer
            .route_moved(terminal, position, self.modifiers)
        {
            Ok(captured) => captured,
            Err(error) => {
                eprintln!("could not send terminal pointer input: {error}");
                true
            }
        }
    }

    pub(super) fn route_terminal_pointer_button(
        &mut self,
        position: Option<TerminalMousePosition>,
        button: MouseButton,
        state: ElementState,
    ) -> bool {
        let Some(terminal) = self.terminal.as_mut() else {
            return false;
        };
        match self
            .terminal_pointer
            .route_button(terminal, position, button, state, self.modifiers)
        {
            Ok(captured) => captured,
            Err(error) => {
                eprintln!("could not send terminal pointer input: {error}");
                true
            }
        }
    }
}

fn send_pointer_input(terminal: &mut TerminalSession, input: PointerInput) -> Result<bool> {
    let PointerInput::Consumed(input) = input else {
        return Ok(false);
    };
    terminal.send_input(input)?;
    Ok(true)
}

fn captures_pointer(terminal: &TerminalCore) -> bool {
    terminal.active_screen() == ScreenBuffer::Alternate
        && terminal.modes().mouse_tracking() != MouseTrackingMode::Disabled
}

fn terminal_button(button: MouseButton) -> Option<TerminalMouseButton> {
    match button {
        MouseButton::Left => Some(TerminalMouseButton::Left),
        MouseButton::Middle => Some(TerminalMouseButton::Middle),
        MouseButton::Right => Some(TerminalMouseButton::Right),
        MouseButton::Back | MouseButton::Forward | MouseButton::Other(_) => None,
    }
}

fn button_state(button: TerminalMouseButton) -> TerminalMouseButtonState {
    match button {
        TerminalMouseButton::Left => TerminalMouseButtonState::Left,
        TerminalMouseButton::Middle => TerminalMouseButtonState::Middle,
        TerminalMouseButton::Right => TerminalMouseButtonState::Right,
        TerminalMouseButton::WheelUp | TerminalMouseButton::WheelDown => {
            TerminalMouseButtonState::None
        }
    }
}

fn wheel_button(delta: MouseScrollDelta) -> Option<TerminalMouseButton> {
    let vertical = match delta {
        MouseScrollDelta::LineDelta(_, vertical) => vertical as f64,
        MouseScrollDelta::PixelDelta(position) => position.y,
    };
    if vertical > 0.0 {
        Some(TerminalMouseButton::WheelUp)
    } else if vertical < 0.0 {
        Some(TerminalMouseButton::WheelDown)
    } else {
        None
    }
}

fn mouse_modifiers(modifiers: ModifiersState) -> MouseModifiers {
    let mut terminal = MouseModifiers::NONE;
    if modifiers.shift_key() {
        terminal = terminal.with_shift();
    }
    if modifiers.alt_key() {
        terminal = terminal.with_alt();
    }
    if modifiers.control_key() {
        terminal = terminal.with_control();
    }
    terminal
}

#[cfg(test)]
#[path = "terminal_pointer_tests.rs"]
mod tests;
