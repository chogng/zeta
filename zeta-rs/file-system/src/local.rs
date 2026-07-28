use crate::{DirectoryEntry, FileMetadata, FileSystemError, FileType, WorkspaceFileSystem};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use zeta_sandboxing::WorkspaceRoot;

/// Local implementation that confines all operations to one canonical workspace root.
pub struct LocalFileSystem {
    workspace: WorkspaceRoot,
}

impl LocalFileSystem {
    pub fn new(workspace: WorkspaceRoot) -> Self {
        Self { workspace }
    }

    fn resolve_existing(&self, path: &Path) -> Result<PathBuf, FileSystemError> {
        self.workspace
            .resolve_existing(path)
            .map_err(|_| FileSystemError::InvalidPath(path.to_path_buf()))
    }
}

impl WorkspaceFileSystem for LocalFileSystem {
    fn read_file(&self, path: &Path, maximum_bytes: usize) -> Result<Vec<u8>, FileSystemError> {
        if maximum_bytes == 0 {
            return Err(FileSystemError::ReadLimitExceeded { maximum_bytes });
        }
        let resolved = self.resolve_existing(path)?;
        let mut file = File::open(resolved).map_err(io_error)?;
        let mut bytes = Vec::with_capacity(maximum_bytes.min(8 * 1024));
        file.by_ref()
            .take((maximum_bytes + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() > maximum_bytes {
            return Err(FileSystemError::ReadLimitExceeded { maximum_bytes });
        }
        Ok(bytes)
    }

    fn get_metadata(&self, path: &Path) -> Result<FileMetadata, FileSystemError> {
        let resolved = self.resolve_existing(path)?;
        let metadata = std::fs::symlink_metadata(resolved).map_err(io_error)?;
        Ok(FileMetadata {
            file_type: file_type(metadata.file_type()),
            size_bytes: metadata.len(),
            readonly: metadata.permissions().readonly(),
            modified_at_millis: metadata
                .modified()
                .ok()
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .and_then(|duration| u64::try_from(duration.as_millis()).ok()),
        })
    }

    fn read_directory(&self, path: &Path) -> Result<Vec<DirectoryEntry>, FileSystemError> {
        let resolved = self.resolve_existing(path)?;
        if !resolved.is_dir() {
            return Err(FileSystemError::NotDirectory(path.to_path_buf()));
        }
        let mut entries = std::fs::read_dir(resolved)
            .map_err(io_error)?
            .map(|entry| {
                let entry = entry.map_err(io_error)?;
                let entry_type = entry.file_type().map_err(io_error)?;
                Ok(DirectoryEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    file_type: file_type(entry_type),
                })
            })
            .collect::<Result<Vec<_>, FileSystemError>>()?;
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }
}

fn file_type(file_type: std::fs::FileType) -> FileType {
    if file_type.is_dir() {
        FileType::Directory
    } else if file_type.is_file() {
        FileType::File
    } else if file_type.is_symlink() {
        FileType::SymbolicLink
    } else {
        FileType::Other
    }
}

fn io_error(error: std::io::Error) -> FileSystemError {
    FileSystemError::Io(error.to_string())
}

#[cfg(test)]
#[path = "local_tests.rs"]
mod tests;
