use crate::{DirectoryEntry, FileMetadata, FileSystemError};
use std::path::Path;

/// Workspace-scoped filesystem access used by both client adapters and Agent tools.
///
/// Implementations must resolve every relative input beneath their configured authority root and
/// must reject absolute paths, parent traversal, and symlink escapes before performing I/O.
pub trait WorkspaceFileSystem: Send + Sync {
    /// Reads one existing file, failing if its content exceeds `maximum_bytes`.
    fn read_file(&self, path: &Path, maximum_bytes: usize) -> Result<Vec<u8>, FileSystemError>;

    /// Atomically replaces or creates one file, failing if `content` exceeds `maximum_bytes`.
    fn write_file(
        &self,
        path: &Path,
        content: &[u8],
        maximum_bytes: usize,
    ) -> Result<FileMetadata, FileSystemError>;

    /// Returns metadata for one existing path.
    fn get_metadata(&self, path: &Path) -> Result<FileMetadata, FileSystemError>;

    /// Lists the direct children of one existing directory.
    fn read_directory(&self, path: &Path) -> Result<Vec<DirectoryEntry>, FileSystemError>;
}
