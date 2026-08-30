//! Host keybinding lifecycle independent of command execution.

mod catalog;
mod engine;
mod input;
mod resource;

pub use catalog::KeybindingCatalog;
pub use engine::CHORD_TIMEOUT;
pub use engine::KeybindingResolution;
pub use engine::Keybindings;
pub use engine::UserBinding;
pub use engine::UserBindingTarget;
pub use input::recording_chord;
pub use resource::KeybindingsResource;
pub use resource::KeybindingsResourceError;
pub use resource::KeybindingsResourcePoll;
pub use resource::binding_diagnostics;
pub use resource::compile_user_bindings;

#[cfg(test)]
#[path = "keybindings_tests.rs"]
mod tests;
