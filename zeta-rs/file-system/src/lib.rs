//! Workspace-scoped filesystem primitives shared by clients and tools.

mod error;
mod local;
mod service;
mod types;

pub use error::FileSystemError;
pub use local::LocalFileSystem;
pub use service::WorkspaceFileSystem;
pub use types::{DirectoryEntry, FileMetadata, FileType};
