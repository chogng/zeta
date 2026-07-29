use crate::manifest::PLUGIN_MANIFEST_PATH;
use crate::{PluginError, PluginErrorKind, PluginPackageDigest, PluginPath};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::path::{Path, PathBuf};

use super::local::PackageFileStats;

const MAX_PACKAGE_FILES: u64 = 10_000;
const MAX_PACKAGE_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PACKAGE_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const DIGEST_DOMAIN: &[u8] = b"zeta-plugin-package-v1\0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ScannedEntryKind {
    Directory,
    File,
}

pub(super) struct ScannedPackage {
    pub(super) digest: PluginPackageDigest,
    pub(super) manifest_bytes: Vec<u8>,
    pub(super) entries: BTreeMap<PluginPath, ScannedEntryKind>,
    pub(super) stats: PackageFileStats,
}

pub(super) fn scan_and_digest(root: &Path) -> Result<ScannedPackage, PluginError> {
    let mut entries = BTreeMap::new();
    let mut files = Vec::new();
    walk_directory(root, root, &mut entries, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    hasher.update(DIGEST_DOMAIN);
    let mut manifest_bytes = None;
    let mut total_bytes = 0_u64;
    for (relative, absolute, expected_metadata) in &files {
        total_bytes = total_bytes
            .checked_add(expected_metadata.len())
            .ok_or_else(|| unsafe_package("plugin package total size overflowed"))?;
        if total_bytes > MAX_PACKAGE_TOTAL_BYTES {
            return Err(unsafe_package(
                "plugin package exceeds the 256 MiB total size limit",
            ));
        }

        update_length(&mut hasher, relative.as_str().len() as u64);
        hasher.update(relative.as_str().as_bytes());
        update_length(&mut hasher, expected_metadata.len());
        let bytes = hash_file(&mut hasher, absolute, relative, expected_metadata)?;
        if relative.as_str() == PLUGIN_MANIFEST_PATH {
            manifest_bytes = Some(bytes);
        }
    }
    let Some(manifest_bytes) = manifest_bytes else {
        return Err(PluginError::new(
            PluginErrorKind::SourceUnavailable,
            format!("plugin package is missing '{PLUGIN_MANIFEST_PATH}'"),
        ));
    };

    Ok(ScannedPackage {
        digest: PluginPackageDigest::from_hasher(hasher),
        manifest_bytes,
        entries,
        stats: PackageFileStats {
            file_count: files.len() as u64,
            total_bytes,
        },
    })
}

fn walk_directory(
    root: &Path,
    directory: &Path,
    entries: &mut BTreeMap<PluginPath, ScannedEntryKind>,
    files: &mut Vec<(PluginPath, PathBuf, Metadata)>,
) -> Result<(), PluginError> {
    let mut children: Vec<_> = fs::read_dir(directory)
        .map_err(|_| source_unavailable("plugin package directory cannot be read"))?
        .collect::<Result<_, _>>()
        .map_err(|_| source_unavailable("plugin package directory cannot be read"))?;
    children.sort_by_key(|entry| entry.file_name());

    for child in children {
        let absolute = child.path();
        let relative_path = absolute
            .strip_prefix(root)
            .map_err(|_| unsafe_package("plugin package entry escaped its root"))?;
        let relative = PluginPath::from_relative_path(relative_path)
            .map_err(|error| unsafe_package(format!("unsafe package path: {error}")))?;
        let metadata = fs::symlink_metadata(&absolute)
            .map_err(|_| source_unavailable("plugin package entry metadata cannot be read"))?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return Err(unsafe_package(format!(
                "plugin package path '{relative}' is a symbolic link"
            )));
        }
        if file_type.is_dir() {
            entries.insert(relative, ScannedEntryKind::Directory);
            walk_directory(root, &absolute, entries, files)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(unsafe_package(format!(
                "plugin package path '{relative}' is not a regular file or directory"
            )));
        }
        if regular_file_has_multiple_links(&metadata) {
            return Err(unsafe_package(format!(
                "plugin package path '{relative}' is a hard link"
            )));
        }
        if metadata.len() > MAX_PACKAGE_FILE_BYTES {
            return Err(unsafe_package(format!(
                "plugin package path '{relative}' exceeds the 16 MiB per-file limit"
            )));
        }
        if files.len() as u64 >= MAX_PACKAGE_FILES {
            return Err(unsafe_package(
                "plugin package exceeds the 10000-file limit",
            ));
        }
        entries.insert(relative.clone(), ScannedEntryKind::File);
        files.push((relative, absolute, metadata));
    }
    Ok(())
}

fn hash_file(
    hasher: &mut Sha256,
    absolute: &Path,
    relative: &PluginPath,
    expected_metadata: &Metadata,
) -> Result<Vec<u8>, PluginError> {
    let mut file = File::open(absolute).map_err(|_| {
        source_unavailable(format!("plugin package path '{relative}' cannot be read"))
    })?;
    let capture = relative.as_str() == PLUGIN_MANIFEST_PATH;
    let mut captured = Vec::new();
    let mut bytes_read = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|_| {
            source_unavailable(format!("plugin package path '{relative}' cannot be read"))
        })?;
        if count == 0 {
            break;
        }
        bytes_read += count as u64;
        if bytes_read > MAX_PACKAGE_FILE_BYTES {
            return Err(unsafe_package(format!(
                "plugin package path '{relative}' changed beyond the per-file limit during read"
            )));
        }
        hasher.update(&buffer[..count]);
        if capture {
            captured.extend_from_slice(&buffer[..count]);
        }
    }
    let observed_metadata = fs::symlink_metadata(absolute).map_err(|_| {
        source_unavailable(format!(
            "plugin package path '{relative}' changed during validation"
        ))
    })?;
    if observed_metadata.file_type().is_symlink()
        || !observed_metadata.is_file()
        || observed_metadata.len() != expected_metadata.len()
        || bytes_read != expected_metadata.len()
        || !same_file(expected_metadata, &observed_metadata)
    {
        return Err(unsafe_package(format!(
            "plugin package path '{relative}' changed during validation"
        )));
    }
    Ok(captured)
}

fn update_length(hasher: &mut Sha256, length: u64) {
    hasher.update(length.to_be_bytes());
}

#[cfg(unix)]
fn regular_file_has_multiple_links(metadata: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink() > 1
}

#[cfg(windows)]
fn regular_file_has_multiple_links(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.number_of_links().is_some_and(|links| links > 1)
}

#[cfg(not(any(unix, windows)))]
fn regular_file_has_multiple_links(_: &Metadata) -> bool {
    false
}

#[cfg(unix)]
fn same_file(before: &Metadata, after: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    before.dev() == after.dev() && before.ino() == after.ino()
}

#[cfg(windows)]
fn same_file(before: &Metadata, after: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    match (
        before.volume_serial_number(),
        before.file_index(),
        after.volume_serial_number(),
        after.file_index(),
    ) {
        (Some(before_volume), Some(before_file), Some(after_volume), Some(after_file)) => {
            before_volume == after_volume && before_file == after_file
        }
        _ => true,
    }
}

#[cfg(not(any(unix, windows)))]
fn same_file(_: &Metadata, _: &Metadata) -> bool {
    true
}

fn source_unavailable(message: impl Into<String>) -> PluginError {
    PluginError::new(PluginErrorKind::SourceUnavailable, message)
}

fn unsafe_package(message: impl Into<String>) -> PluginError {
    PluginError::new(PluginErrorKind::PackageUnsafe, message)
}
