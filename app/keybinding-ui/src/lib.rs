//! App shortcut recording state and native settings presentation.

mod recording;
mod settings;

pub use recording::KeyboardShortcutsState;
pub use recording::ShortcutCommit;
pub use settings::KeyboardShortcutRow;
pub use settings::KeyboardShortcuts;
pub use settings::KeyboardShortcutsIds;
pub use settings::KeyboardShortcutsStyle;
pub use settings::paint_chord_hint;
