use std::collections::HashSet;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Paths selected for reading and atomically replacing a possibly symlinked file.
///
/// `read_path` is absent when the symlink chain cannot be resolved safely.
/// `write_path` always remains usable as the conservative replacement target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymlinkWritePaths {
    pub read_path: Option<PathBuf>,
    pub write_path: PathBuf,
}

/// Resolves the final target of a symlink chain while retaining a safe write path.
///
/// Relative symlink targets are resolved against their containing directory.
/// Missing final targets are valid. Cycles and metadata failures fall back to
/// the original path and suppress `read_path`.
pub fn resolve_symlink_write_paths(path: &Path) -> SymlinkWritePaths {
    let root = path.to_path_buf();
    let mut current = root.clone();
    let mut visited = HashSet::new();

    loop {
        let metadata = match std::fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return SymlinkWritePaths {
                    read_path: Some(current.clone()),
                    write_path: current,
                };
            }
            Err(_) => return unresolved(root),
        };

        if !metadata.file_type().is_symlink() {
            return SymlinkWritePaths {
                read_path: Some(current.clone()),
                write_path: current,
            };
        }
        if !visited.insert(current.clone()) {
            return unresolved(root);
        }

        let target = match std::fs::read_link(&current) {
            Ok(target) => target,
            Err(_) => return unresolved(root),
        };
        current = if target.is_absolute() {
            target
        } else if let Some(parent) = current.parent() {
            parent.join(target)
        } else {
            return unresolved(root);
        };
    }
}

/// Atomically replaces a file with the supplied bytes.
///
/// The parent directory is created first. Bytes are flushed to the temporary
/// file before it is renamed over `write_path`. Failures before rename leave
/// the previous destination intact. A parent-directory sync failure is reported
/// after the replacement has become visible.
pub fn write_atomically(write_path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = write_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path {} has no parent directory", write_path.display()),
        )
    })?;
    std::fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(contents)?;
    temporary.as_file().sync_all()?;
    temporary.persist(write_path).map_err(|error| error.error)?;
    sync_parent(parent)
}

/// Atomically replaces a UTF-8 text file.
pub fn write_text_atomically(write_path: &Path, contents: &str) -> io::Result<()> {
    write_atomically(write_path, contents.as_bytes())
}

fn unresolved(write_path: PathBuf) -> SymlinkWritePaths {
    SymlinkWritePaths {
        read_path: None,
        write_path,
    }
}

fn sync_parent(parent: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::fs::File::open(parent)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = parent;
        Ok(())
    }
}
