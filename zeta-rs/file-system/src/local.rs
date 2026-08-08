use crate::file_revision;
use crate::DirectoryEntry;
use crate::FileContent;
use crate::FileMetadata;
use crate::FileSystemError;
use crate::FileType;
use crate::FileWriteCondition;
use crate::WorkspaceFileSystem;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;
use tempfile::NamedTempFile;
use zeta_workspace::WorkspaceRoot;

/// Local implementation that confines all operations to one canonical workspace root.
pub struct LocalFileSystem {
    workspace: WorkspaceRoot,
    write_lock: Mutex<()>,
}

impl LocalFileSystem {
    pub fn new(workspace: WorkspaceRoot) -> Self {
        Self {
            workspace,
            write_lock: Mutex::new(()),
        }
    }

    fn resolve_existing(&self, path: &Path) -> Result<PathBuf, FileSystemError> {
        self.workspace
            .resolve_existing(path)
            .map_err(|_| FileSystemError::InvalidPath(path.to_path_buf()))
    }

    fn resolve_for_write(&self, path: &Path) -> Result<PathBuf, FileSystemError> {
        self.workspace
            .resolve_for_write(path)
            .map_err(|_| FileSystemError::InvalidPath(path.to_path_buf()))
    }

    fn write_file_inner(
        &self,
        path: &Path,
        content: &[u8],
        maximum_bytes: usize,
    ) -> Result<FileMetadata, FileSystemError> {
        if content.len() > maximum_bytes {
            return Err(FileSystemError::WriteLimitExceeded { maximum_bytes });
        }
        let resolved = self.resolve_for_write(path)?;
        let existing_metadata = match std::fs::metadata(&resolved) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err(FileSystemError::NotFile(path.to_path_buf()));
                }
                if metadata.permissions().readonly() {
                    return Err(FileSystemError::ReadOnly(path.to_path_buf()));
                }
                Some(metadata)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(io_error(error)),
        };
        let parent = resolved
            .parent()
            .ok_or_else(|| FileSystemError::InvalidPath(path.to_path_buf()))?;
        let parent_metadata = std::fs::metadata(parent).map_err(io_error)?;
        if !parent_metadata.is_dir() {
            return Err(FileSystemError::NotDirectory(
                path.parent().unwrap_or(Path::new("")).to_path_buf(),
            ));
        }
        atomic_write(
            &resolved,
            content,
            existing_metadata
                .as_ref()
                .map(std::fs::Metadata::permissions),
        )
        .map_err(io_error)?;
        metadata(&resolved)
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
        Read::by_ref(&mut file)
            .take((maximum_bytes + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() > maximum_bytes {
            return Err(FileSystemError::ReadLimitExceeded { maximum_bytes });
        }
        Ok(bytes)
    }

    fn write_file(
        &self,
        path: &Path,
        content: &[u8],
        maximum_bytes: usize,
    ) -> Result<FileMetadata, FileSystemError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| FileSystemError::Io("workspace write lock is poisoned".into()))?;
        self.write_file_inner(path, content, maximum_bytes)
    }

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

    fn write_file_with_condition(
        &self,
        path: &Path,
        content: &[u8],
        maximum_bytes: usize,
        condition: &FileWriteCondition,
    ) -> Result<FileMetadata, FileSystemError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| FileSystemError::Io("workspace write lock is poisoned".into()))?;
        if let FileWriteCondition::ExpectedRevision(expected) = condition {
            let current = self.read_file(path, maximum_bytes)?;
            if file_revision(&current) != *expected {
                return Err(FileSystemError::RevisionConflict(path.to_path_buf()));
            }
        }
        self.write_file_inner(path, content, maximum_bytes)
    }

    fn get_metadata(&self, path: &Path) -> Result<FileMetadata, FileSystemError> {
        let resolved = self.resolve_existing(path)?;
        metadata(&resolved)
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

fn atomic_write(
    target: &Path,
    content: &[u8],
    permissions: Option<std::fs::Permissions>,
) -> std::io::Result<()> {
    let parent = target.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("write target has no parent: {}", target.display()),
        )
    })?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(content)?;
    if let Some(permissions) = permissions {
        temporary.as_file().set_permissions(permissions)?;
    }
    temporary.as_file().sync_all()?;
    temporary.persist(target).map_err(|error| error.error)?;
    sync_parent(parent)
}

fn sync_parent(parent: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        File::open(parent)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = parent;
        Ok(())
    }
}

fn metadata(path: &Path) -> Result<FileMetadata, FileSystemError> {
    let metadata = std::fs::symlink_metadata(path).map_err(io_error)?;
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
