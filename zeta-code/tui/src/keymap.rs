//! Runtime keymap resolution for the Zeta Code TUI.
//!
//! This file is the module entry and runtime owner. Binding declarations,
//! chord lifecycle, and terminal key conversion live in focused submodules.

mod bindings;
mod chords;
mod editor;
mod input;
mod settings;

/// A completed keymap operation delivered to the TUI state owner.
pub(crate) enum Event {
    SettingsReceived(KeymapSettings),
    EditorOpened(KeymapEditorUpdate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    OpenEditor,
    Edit(KeymapEdit),
}

use bindings::AppKeymapCondition;
use chords::PendingChord;
use zeta_keybinding::BindingSet;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::UserBinding;

#[cfg(test)]
use bindings::AppKeybindingSpec;
pub(crate) use bindings::AppKeymapAction;
pub(crate) use bindings::AppKeymapContext;
pub(crate) use bindings::KeymapActionSnapshot;
pub(crate) use bindings::compile_app_user_bindings;
pub(crate) use chords::AppChordMatch;
#[cfg(test)]
use chords::KEY_CHORD_TIMEOUT;
pub(crate) use editor::KeymapChoices;
pub(crate) use editor::KeymapEditor;
pub(crate) use editor::KeymapEditorOutcome;
pub(crate) use editor::KeymapEditorPage;
#[cfg(test)]
pub(crate) use editor::keymap_choices;
pub(crate) use input::compose_config_chord;
pub(crate) use input::key_event_to_config_key;
pub(crate) use settings::KeymapCaptureMode;
pub(crate) use settings::KeymapEdit;
pub(crate) use settings::KeymapEditIntent;
pub(crate) use settings::KeymapEditKind;
pub(crate) use settings::KeymapEditorUpdate;
pub(crate) use settings::KeymapSettings;
pub(crate) use settings::fixed_shortcuts;
pub(crate) use settings::read_keymap;
pub(crate) use settings::set_keymap;
pub(crate) use settings::settings_from_tui;

/// Application-level shortcuts that sit above individual TUI components.
#[derive(Debug)]
pub(super) struct AppKeymap {
    single_bindings: BindingSet<AppKeymapCondition, AppKeymapAction>,
    chord_bindings: BindingSet<AppKeymapCondition, AppKeymapAction>,
    platform: HostPlatform,
    pending: Option<PendingChord>,
    user_bindings: Vec<AppUserBinding>,
}

pub(super) type AppUserBinding = UserBinding<AppKeymapAction, AppKeymapCondition>;

#[cfg(test)]
#[path = "keymap/keymap_tests.rs"]
mod tests;
