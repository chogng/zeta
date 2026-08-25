use zeta_keybinding::Chord;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::KeyIdentity;
use zeta_keybinding::KeyStroke;
use zeta_keybinding::LogicalKey;
use zeta_keybinding::Modifiers;
use zeta_keybinding::PhysicalKey;
use zeta_keybinding::ShortcutModifiers;
use zui::input::Key;
use zui::input::KeyEvent;
use zui::input::ModifiersState;
use zui::input::NamedKey;

pub(crate) fn key_stroke(event: &KeyEvent, modifiers: ModifiersState) -> Option<KeyStroke> {
    let logical_key = match &event.logical_key {
        Key::Character(text) => LogicalKey::new(text.as_str()),
        Key::Named(key) => LogicalKey::new(format!("{key:?}")),
        Key::Dead(character) => {
            character.and_then(|character| LogicalKey::new(character.to_string()))
        }
        Key::Unidentified(_) => None,
    }?;
    let physical_key = match &event.physical_key {
        zui::input::PhysicalKey::Code(code) => PhysicalKey::new(format!("{code:?}")),
        zui::input::PhysicalKey::Unidentified(_) => None,
    };
    Some(KeyStroke::new(
        logical_key,
        physical_key,
        shortcut_modifiers(modifiers),
    ))
}

/// Converts a non-modifier platform key event into a portable chord for shortcut recording.
pub fn recording_chord(
    event: &KeyEvent,
    modifiers: ModifiersState,
    platform: HostPlatform,
) -> Option<Chord> {
    if matches!(
        event.logical_key,
        Key::Named(
            NamedKey::Alt
                | NamedKey::AltGraph
                | NamedKey::Control
                | NamedKey::Fn
                | NamedKey::FnLock
                | NamedKey::Meta
                | NamedKey::Shift
                | NamedKey::Super
        )
    ) {
        return None;
    }
    let key = match &event.logical_key {
        Key::Character(text) => LogicalKey::new(text.as_str()),
        Key::Named(key) => LogicalKey::new(format!("{key:?}")),
        Key::Dead(character) => {
            character.and_then(|character| LogicalKey::new(character.to_string()))
        }
        Key::Unidentified(_) => None,
    }?;
    let mut shortcut = ShortcutModifiers::none();
    if modifiers.control_key() {
        shortcut = if platform == HostPlatform::MacOs {
            shortcut.with_control()
        } else {
            shortcut.with_primary()
        };
    }
    if modifiers.shift_key() {
        shortcut = shortcut.with_shift();
    }
    if modifiers.alt_key() {
        shortcut = shortcut.with_alt();
    }
    if modifiers.super_key() {
        shortcut = if platform == HostPlatform::MacOs {
            shortcut.with_primary()
        } else {
            shortcut.with_meta()
        };
    }
    Some(Chord::new(KeyIdentity::Logical(key), shortcut))
}

fn shortcut_modifiers(modifiers: ModifiersState) -> Modifiers {
    let mut shortcut = Modifiers::none();
    if modifiers.control_key() {
        shortcut = shortcut.with_control();
    }
    if modifiers.shift_key() {
        shortcut = shortcut.with_shift();
    }
    if modifiers.alt_key() {
        shortcut = shortcut.with_alt();
    }
    if modifiers.super_key() {
        shortcut = shortcut.with_meta();
    }
    shortcut
}
