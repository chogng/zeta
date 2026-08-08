use crate::file_revision;
use crate::DirectoryEntry;
use crate::FileContent;
use crate::FileMetadata;
use crate::FileSystemError;
use crate::FileWriteCondition;
use std::path::Path;

/// Workspace-scoped filesystem access used by both client adapters and Agent tools.
///
/// Implementations must resolve every relative input beneath their configured authority root and
/// must reject absolute paths, parent traversal, and symlink escapes before performing I/O.
pub trait WorkspaceFileSystem: Send + Sync {
    /// Reads one existing file, failing if its content exceeds `maximum_bytes`.
    fn read_file(&self, path: &Path, maximum_bytes: usize) -> Result<Vec<u8>, FileSystemError>;

    /// Reads file bytes with the opaque revision required by a conditional write.
    fn read_file_with_revision(
        &self,
        path: &Path,
        maximum_bytes: usize,
    ) -> Result<FileContent, FileSystemError> {
        let bytes = self.read_file(path, maximum_bytes)?;
        Ok(FileContent {
            revision: file_revision(&bytes),
            bytes,
        })
    }

    /// Atomically replaces or creates one file, failing if `content` exceeds `maximum_bytes`.
    fn write_file(
        &self,
        path: &Path,
        content: &[u8],
        maximum_bytes: usize,
    ) -> Result<FileMetadata, FileSystemError>;

    /// Writes only when the condition still matches the current file bytes.
    ///
    /// Implementations that can serialize a revision check with replacement should override this
    /// default so concurrent clients cannot pass the same stale condition.
    fn write_file_with_condition(
        &self,
        path: &Path,
        content: &[u8],
        maximum_bytes: usize,
        condition: &FileWriteCondition,
    ) -> Result<FileMetadata, FileSystemError> {
        if let FileWriteCondition::ExpectedRevision(expected) = condition {
            let current = self.read_file(path, maximum_bytes)?;
            if file_revision(&current) != *expected {
                return Err(FileSystemError::RevisionConflict(path.to_path_buf()));
            }
        }
        self.write_file(path, content, maximum_bytes)
    }

    /// Returns metadata for one existing path.
    fn get_metadata(&self, path: &Path) -> Result<FileMetadata, FileSystemError>;

    /// Lists the direct children of one existing directory.
    fn read_directory(&self, path: &Path) -> Result<Vec<DirectoryEntry>, FileSystemError>;
}
