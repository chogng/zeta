use super::is_clipboard_shortcut;
use zeta_winit::{Key, ModifiersState};

#[test]
fn terminal_clipboard_shortcuts_preserve_unshifted_control_keys() {
    let key = Key::Character("v".into());

    assert!(!is_clipboard_shortcut(
        &key,
        "v",
        ModifiersState::CONTROL,
        true,
    ));
    assert!(is_clipboard_shortcut(
        &key,
        "v",
        ModifiersState::CONTROL | ModifiersState::SHIFT,
        true,
    ));
    assert!(is_clipboard_shortcut(
        &key,
        "v",
        ModifiersState::SUPER,
        true,
    ));
    assert!(is_clipboard_shortcut(
        &key,
        "v",
        ModifiersState::CONTROL,
        false,
    ));
}
