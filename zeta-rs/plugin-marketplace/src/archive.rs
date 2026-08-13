use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use zip::ZipArchive;

use crate::RemoteMarketplaceError;
use crate::RemoteMarketplaceErrorKind;

pub(crate) const MAX_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 4096;
const MAX_EXPANDED_BYTES: u64 = 128 * 1024 * 1024;

pub(crate) fn extract(bytes: &[u8], destination: &Path) -> Result<(), RemoteMarketplaceError> {
    if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(archive_error());
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| archive_error())?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(archive_error());
    }
    fs::create_dir_all(destination).map_err(|_| cache_error())?;
    let mut paths = BTreeSet::new();
    let mut expanded = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|_| archive_error())?;
        if entry.is_symlink() || entry.encrypted() {
            return Err(archive_error());
        }
        let path = safe_path(entry.name())?;
        if !paths.insert(path.clone()) {
            return Err(archive_error());
        }
        expanded = expanded
            .checked_add(entry.size())
            .filter(|bytes| *bytes <= MAX_EXPANDED_BYTES)
            .ok_or_else(archive_error)?;
        let output = destination.join(&path);
        if entry.is_dir() {
            fs::create_dir_all(&output).map_err(|_| cache_error())?;
            continue;
        }
        if !entry.is_file() {
            return Err(archive_error());
        }
        let parent = output.parent().ok_or_else(archive_error)?;
        fs::create_dir_all(parent).map_err(|_| cache_error())?;
        let mut options = fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let expected = entry.size();
        let mut output = options.open(output).map_err(|_| cache_error())?;
        let copied = std::io::copy(&mut entry.take(expected + 1), &mut output)
            .map_err(|_| archive_error())?;
        if copied != expected {
            return Err(archive_error());
        }
        output.flush().map_err(|_| cache_error())?;
    }
    Ok(())
}

fn safe_path(name: &str) -> Result<PathBuf, RemoteMarketplaceError> {
    if name.is_empty() || name.contains('\\') || name.contains('\0') {
        return Err(archive_error());
    }
    let path = Path::new(name);
    if path.components().any(|component| {
        !matches!(component, Component::Normal(_))
            && !(matches!(component, Component::CurDir) && name.ends_with('/'))
    }) {
        return Err(archive_error());
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        if let Component::Normal(component) = component {
            normalized.push(component);
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(archive_error());
    }
    Ok(normalized)
}

fn archive_error() -> RemoteMarketplaceError {
    RemoteMarketplaceError::new(
        RemoteMarketplaceErrorKind::PackageUnsafe,
        "Plugin Marketplace package archive is unsafe",
    )
}

fn cache_error() -> RemoteMarketplaceError {
    RemoteMarketplaceError::new(
        RemoteMarketplaceErrorKind::CacheUnavailable,
        "Plugin Marketplace cache is unavailable",
    )
}
