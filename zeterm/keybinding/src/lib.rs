//! Shortcut values, rule resolution, recording lifecycle, and settings presentation.

mod binding;
mod context;
mod key;
mod parser;
mod recording;
mod resolver;
mod settings;

pub use binding::{BindingPriority, BindingSet, BindingSource};
pub use context::{ContextExpression, ContextExpressionError, ContextValue};
pub use key::{
    Chord, HostPlatform, KeyIdentity, KeySequence, KeySequenceError, KeyStroke, LogicalKey,
    MAX_CHORDS, Modifiers, PhysicalKey, ShortcutModifiers,
};
pub use parser::{
    KeybindingParseError, format_key_sequence, keycap_labels, parse_key_sequence,
    serialize_key_sequence,
};
pub use recording::{KeyboardShortcutsState, ShortcutCommit};
pub use resolver::{KeybindingResolver, ResolveResult};
pub use settings::{
    KeyboardShortcutRow, KeyboardShortcuts, KeyboardShortcutsIds, KeyboardShortcutsStyle,
    paint_chord_hint,
};

#[cfg(test)]
#[path = "keybinding_tests.rs"]
mod tests;
