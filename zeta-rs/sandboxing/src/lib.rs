//! Cross-platform sandbox policy and backend coordination.
//!
//! Platform crates implement [`SandboxBackend`]. This crate validates workspace-relative command
//! paths, owns the shared policy vocabulary, and coordinates command preparation. On macOS it
//! also provides the native Seatbelt backend.

mod error;
mod manager;
mod model;
mod workspace;

#[cfg(target_os = "macos")]
mod macos;

pub use error::SandboxError;
pub use manager::{SandboxBackend, SandboxManager};
pub use model::{
    FileSystemAccess, NetworkAccess, PreparedCommand, SandboxCommand, SandboxKind, SandboxPolicy,
};
pub use workspace::WorkspaceRoot;

#[cfg(target_os = "macos")]
pub use macos::MacosSeatbeltSandbox;
