use zeta_keybinding::{Chord, HostPlatform, KeyIdentity, LogicalKey, ShortcutModifiers};
use zeta_winit::{Key, KeyEvent, ModifiersState, NamedKey};

pub(super) fn recording_chord(
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
