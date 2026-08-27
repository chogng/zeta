use zeta_commands::AppCommandId;

use super::SettingsActivation;
use super::SettingsState;
use crate::SETTINGS_NAV_APPEARANCE;
use crate::SETTINGS_NAV_GENERAL;
use crate::SettingsPageSection;
use crate::keyboard_shortcut_row_element;

#[test]
fn opening_shortcuts_selects_the_section_and_owns_the_recorder() {
    let mut settings = SettingsState::default();

    settings.open_keyboard_shortcuts();

    assert_eq!(settings.section(), SettingsPageSection::Keybindings);
    assert!(settings.keyboard_shortcuts().is_visible());
}

#[test]
fn activation_changes_owned_state_and_returns_host_effects() {
    let mut settings = SettingsState::default();
    assert_eq!(settings.section(), SettingsPageSection::General);
    assert_eq!(
        settings.activate(SETTINGS_NAV_APPEARANCE),
        SettingsActivation::Changed
    );
    assert_eq!(settings.section(), SettingsPageSection::Appearance);
    assert_eq!(
        settings.activate(SETTINGS_NAV_GENERAL),
        SettingsActivation::Changed
    );
    assert_eq!(settings.section(), SettingsPageSection::General);
}

#[test]
fn shortcut_rows_are_inactive_outside_the_keybindings_section() {
    let mut settings = SettingsState::default();

    assert_eq!(
        settings.activate(keyboard_shortcut_row_element(AppCommandId::Copy)),
        SettingsActivation::Ignored
    );
}
