use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use sha2::Digest;
use sha2::Sha256;
use zeta_language_server_distribution::LanguageServerPackage;
use zeta_language_server_distribution::LanguageServerPackageFile;
use zip::ZipArchive;

use crate::LanguageMarketplaceEntry;
use crate::LanguageMarketplaceError;
use crate::LanguageMarketplaceErrorKind;
use crate::LanguagePackageDigest;

pub(crate) const MAX_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_EXPANDED_BYTES: u64 = 256 * 1024 * 1024;
const PACKAGE_DIGEST_DOMAIN: &[u8] = b"marketplace-package-v1\0";

pub(crate) fn extract(
    bytes: &[u8],
    destination: &Path,
    expected_files: u64,
    expected_bytes: u64,
) -> Result<(), LanguageMarketplaceError> {
    if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(package_error());
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| package_error())?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(package_error());
    }
    fs::create_dir_all(destination).map_err(|_| cache_error())?;
    let mut paths = BTreeSet::new();
    let mut expanded = 0_u64;
    let mut file_count = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|_| package_error())?;
        if entry.is_symlink() || entry.encrypted() {
            return Err(package_error());
        }
        let path = safe_path(entry.name())?;
        if !paths.insert(path.clone()) {
            return Err(package_error());
        }
        let output = destination.join(&path);
        if entry.is_dir() {
            fs::create_dir_all(&output).map_err(|_| cache_error())?;
            continue;
        }
        if !entry.is_file() || entry.size() > MAX_FILE_BYTES {
            return Err(package_error());
        }
        expanded = expanded
            .checked_add(entry.size())
            .filter(|bytes| *bytes <= MAX_EXPANDED_BYTES)
            .ok_or_else(package_error)?;
        file_count = file_count.checked_add(1).ok_or_else(package_error)?;
        let parent = output.parent().ok_or_else(package_error)?;
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
            .map_err(|_| package_error())?;
        if copied != expected {
            return Err(package_error());
        }
        output.flush().map_err(|_| cache_error())?;
    }
    if file_count != expected_files || expanded != expected_bytes {
        return Err(package_error());
    }
    Ok(())
}

pub(crate) fn verify_package(
    root: &Path,
    expected_digest: &LanguagePackageDigest,
    expected_files: u64,
    expected_bytes: u64,
) -> Result<(), LanguageMarketplaceError> {
    let (digest, files, bytes) = scan_package(root)?;
    if &digest != expected_digest || files != expected_files || bytes != expected_bytes {
        return Err(package_error());
    }
    Ok(())
}

pub(crate) fn language_server_package(
    root: &Path,
    entry: &LanguageMarketplaceEntry,
) -> Result<LanguageServerPackage, LanguageMarketplaceError> {
    verify_package(
        root,
        &entry.digest,
        entry.package_file_count,
        entry.package_size_bytes,
    )?;
    let mut pending = vec![root.to_path_buf()];
    let mut package_files = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut children = fs::read_dir(&directory)
            .map_err(|_| package_error())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| package_error())?;
        children.sort_by_key(fs::DirEntry::file_name);
        for child in children {
            let absolute = child.path();
            let metadata = fs::symlink_metadata(&absolute).map_err(|_| package_error())?;
            if metadata.file_type().is_symlink() {
                return Err(package_error());
            }
            if metadata.is_dir() {
                pending.push(absolute);
                continue;
            }
            if !metadata.is_file() || regular_file_has_multiple_links(&metadata) {
                return Err(package_error());
            }
            let relative = absolute
                .strip_prefix(root)
                .map_err(|_| package_error())?
                .to_path_buf();
            let bytes = fs::read(&absolute).map_err(|_| package_error())?;
            let file = if relative == entry.executable_path {
                LanguageServerPackageFile::executable(relative, bytes)
            } else {
                LanguageServerPackageFile::regular(relative, bytes)
            }
            .map_err(|_| package_error())?;
            package_files.push(file);
        }
    }
    LanguageServerPackage::new(
        &entry.server_id,
        entry.version.to_string(),
        &entry.executable_path,
        package_files,
    )
    .map_err(|_| package_error())
}

fn scan_package(
    root: &Path,
) -> Result<(LanguagePackageDigest, u64, u64), LanguageMarketplaceError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| package_error())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(package_error());
    }
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut children = fs::read_dir(&directory)
            .map_err(|_| package_error())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| package_error())?;
        children.sort_by_key(fs::DirEntry::file_name);
        for child in children {
            let absolute = child.path();
            let metadata = fs::symlink_metadata(&absolute).map_err(|_| package_error())?;
            if metadata.file_type().is_symlink() {
                return Err(package_error());
            }
            if metadata.is_dir() {
                pending.push(absolute);
                continue;
            }
            if !metadata.is_file()
                || regular_file_has_multiple_links(&metadata)
                || metadata.len() > MAX_FILE_BYTES
                || files.len() >= MAX_ARCHIVE_ENTRIES
            {
                return Err(package_error());
            }
            let relative = absolute
                .strip_prefix(root)
                .map_err(|_| package_error())?
                .to_str()
                .ok_or_else(package_error)?
                .replace('\\', "/");
            files.push((relative, absolute, metadata.len()));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hasher = Sha256::new();
    hasher.update(PACKAGE_DIGEST_DOMAIN);
    let mut total_bytes = 0_u64;
    for (relative, absolute, expected_bytes) in &files {
        total_bytes = total_bytes
            .checked_add(*expected_bytes)
            .filter(|bytes| *bytes <= MAX_EXPANDED_BYTES)
            .ok_or_else(package_error)?;
        update_length(&mut hasher, relative.len() as u64);
        hasher.update(relative.as_bytes());
        update_length(&mut hasher, *expected_bytes);
        let bytes = fs::read(absolute).map_err(|_| package_error())?;
        if bytes.len() as u64 != *expected_bytes {
            return Err(package_error());
        }
        hasher.update(bytes);
    }
    let digest = LanguagePackageDigest::new(format!("sha256:{:x}", hasher.finalize()))?;
    Ok((digest, files.len() as u64, total_bytes))
}

fn update_length(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_be_bytes());
}

fn safe_path(name: &str) -> Result<PathBuf, LanguageMarketplaceError> {
    if name.is_empty() || name.contains('\\') || name.contains('\0') {
        return Err(package_error());
    }
    let path = Path::new(name);
    if path.components().any(|component| {
        !matches!(component, Component::Normal(_))
            && !(matches!(component, Component::CurDir) && name.ends_with('/'))
    }) {
        return Err(package_error());
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        if let Component::Normal(component) = component {
            normalized.push(component);
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(package_error());
    }
    Ok(normalized)
}

#[cfg(unix)]
fn regular_file_has_multiple_links(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    metadata.nlink() != 1
}

#[cfg(not(unix))]
fn regular_file_has_multiple_links(_metadata: &fs::Metadata) -> bool {
    false
}

fn package_error() -> LanguageMarketplaceError {
    LanguageMarketplaceError::new(
        LanguageMarketplaceErrorKind::PackageUnsafe,
        "Language Marketplace package did not match signed metadata",
    )
}

fn cache_error() -> LanguageMarketplaceError {
    LanguageMarketplaceError::new(
        LanguageMarketplaceErrorKind::CacheUnavailable,
        "Language Marketplace cache is unavailable",
    )
}
