//! Cross-platform sandbox policy and backend coordination.
//!
//! Platform crates implement [`SandboxBackend`]. This crate validates directory-relative command
//! paths, owns the shared policy vocabulary, and coordinates command preparation. On macOS it
//! also provides the native Seatbelt backend.

mod dir;
mod error;
mod manager;
mod model;

#[cfg(target_os = "macos")]
mod macos;

pub use dir::PROTECTED_DIR_METADATA_NAMES;
pub use error::SandboxError;
pub use manager::{SandboxBackend, SandboxManager};
pub use model::{
    FileSystemAccess, NetworkAccess, PreparedCommand, SandboxCommand, SandboxDenialTiming,
    SandboxKind, SandboxPolicy, SandboxProcessDenial, SandboxProcessExitStatus,
};

#[cfg(target_os = "macos")]
pub use macos::MacosSeatbeltSandbox;
