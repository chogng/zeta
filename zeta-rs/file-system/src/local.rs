use crate::DirectoryEntry;
use crate::ExistingTargetBehavior;
use crate::FileContent;
use crate::FileDeleteMode;
use crate::FileMetadata;
use crate::FileSystem;
use crate::FileSystemError;
use crate::FileType;
use crate::FileWriteCondition;
use crate::MissingTargetBehavior;
use crate::file_revision;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;
use tempfile::NamedTempFile;
use zeta_file_access::Dir;

/// Local implementation that confines all operations to one canonical directory.
pub struct LocalFileSystem {
    dir: Dir,
    write_lock: Mutex<()>,
}

impl LocalFileSystem {
    pub fn new(dir: Dir) -> Self {
        Self {
            dir,
            write_lock: Mutex::new(()),
        }
    }

    fn resolve_existing(&self, path: &Path) -> Result<PathBuf, FileSystemError> {
        match self.dir.resolve_existing(path) {
            Ok(resolved) => Ok(resolved),
            Err(_) => match self.dir.resolve_for_write(path) {
                Ok(candidate) if candidate.try_exists().map_err(io_error)? => {
                    Err(FileSystemError::InvalidPath(path.to_path_buf()))
                }
                Ok(_) => Err(FileSystemError::NotFound(path.to_path_buf())),
                Err(_) => Err(FileSystemError::InvalidPath(path.to_path_buf())),
            },
        }
    }

    fn resolve_for_write(&self, path: &Path) -> Result<PathBuf, FileSystemError> {
        self.dir
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

impl FileSystem for LocalFileSystem {
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
            .map_err(|_| FileSystemError::Io("directory write lock is poisoned".into()))?;
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
            .map_err(|_| FileSystemError::Io("directory write lock is poisoned".into()))?;
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

    fn create_file(
        &self,
        path: &Path,
        existing: ExistingTargetBehavior,
    ) -> Result<FileMetadata, FileSystemError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| FileSystemError::Io("directory write lock is poisoned".into()))?;
        let resolved = self.resolve_for_write(path)?;
        if resolved.exists() {
            return match existing {
                ExistingTargetBehavior::Error => {
                    Err(FileSystemError::AlreadyExists(path.to_path_buf()))
                }
                ExistingTargetBehavior::Ignore => metadata(&resolved),
                ExistingTargetBehavior::Overwrite => self.write_file_inner(path, &[], 1),
            };
        }
        self.write_file_inner(path, &[], 1)
    }

    fn rename(
        &self,
        source: &Path,
        target: &Path,
        existing: ExistingTargetBehavior,
    ) -> Result<(), FileSystemError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| FileSystemError::Io("directory write lock is poisoned".into()))?;
        let source_path = self.resolve_existing(source)?;
        let target_path = self.resolve_for_write(target)?;
        if source_path == target_path {
            return Ok(());
        }
        if target_path.exists() {
            match existing {
                ExistingTargetBehavior::Error => {
                    return Err(FileSystemError::AlreadyExists(target.to_path_buf()));
                }
                ExistingTargetBehavior::Ignore => return Ok(()),
                ExistingTargetBehavior::Overwrite => {
                    let backup = rename_backup_path(&target_path)?;
                    std::fs::rename(&target_path, &backup).map_err(io_error)?;
                    if let Err(error) = std::fs::rename(&source_path, &target_path) {
                        let _ = std::fs::rename(&backup, &target_path);
                        return Err(io_error(error));
                    }
                    let _ = remove_resource(&backup, FileDeleteMode::Recursive);
                    return Ok(());
                }
            }
        }
        let parent = target_path
            .parent()
            .ok_or_else(|| FileSystemError::InvalidPath(target.to_path_buf()))?;
        if !parent.is_dir() {
            return Err(FileSystemError::NotDirectory(
                target.parent().unwrap_or(Path::new("")).to_path_buf(),
            ));
        }
        std::fs::rename(source_path, target_path).map_err(io_error)
    }

    fn delete(
        &self,
        path: &Path,
        missing: MissingTargetBehavior,
        mode: FileDeleteMode,
    ) -> Result<(), FileSystemError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| FileSystemError::Io("directory write lock is poisoned".into()))?;
        let candidate = self.resolve_for_write(path)?;
        if !candidate.exists() {
            return match missing {
                MissingTargetBehavior::Error => Err(FileSystemError::NotFound(path.to_path_buf())),
                MissingTargetBehavior::Ignore => Ok(()),
            };
        }
        let resolved = self.resolve_existing(path)?;
        remove_resource(&resolved, mode)
    }
}

fn rename_backup_path(target: &Path) -> Result<PathBuf, FileSystemError> {
    let parent = target
        .parent()
        .ok_or_else(|| FileSystemError::InvalidPath(target.to_path_buf()))?;
    for sequence in 0..1_024u32 {
        let candidate = parent.join(format!(
            ".zeta-rename-backup-{}-{sequence}",
            std::process::id()
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(FileSystemError::Io(
        "could not allocate a directory rename backup path".into(),
    ))
}

fn remove_resource(path: &Path, mode: FileDeleteMode) -> Result<(), FileSystemError> {
    let metadata = std::fs::symlink_metadata(path).map_err(io_error)?;
    if metadata.file_type().is_dir() {
        match mode {
            FileDeleteMode::FileOrEmptyDirectory => std::fs::remove_dir(path).map_err(io_error),
            FileDeleteMode::Recursive => std::fs::remove_dir_all(path).map_err(io_error),
        }
    } else {
        std::fs::remove_file(path).map_err(io_error)
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
