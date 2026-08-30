//! Host keybinding lifecycle independent of command execution.

mod catalog;
mod engine;
mod input;
mod settings;

pub use catalog::KeybindingCatalog;
pub use engine::CHORD_TIMEOUT;
pub use engine::KeybindingResolution;
pub use engine::Keybindings;
pub use engine::UserBinding;
pub use engine::UserBindingTarget;
pub use input::recording_chord;
pub use settings::KeybindingsConfigError;
pub use settings::binding_diagnostics;
pub use settings::compile_user_bindings;
pub use settings::edited_user_bindings;

#[cfg(test)]
#[path = "keybindings_tests.rs"]
mod tests;
