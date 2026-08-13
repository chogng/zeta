//! Verified, application-managed language-server package installation.
//!
//! Download providers remain outside this crate. They supply an immutable package and the digest
//! obtained from their trusted release metadata; this crate verifies and installs it side by side.

mod activation;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use activation::LanguageServerActivationAuthority;
pub use activation::LanguageServerActivationSnapshot;

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(1);
const MAX_INSTALLED_FILES: usize = 10_000;
const MAX_INSTALLED_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_INSTALLED_TOTAL_BYTES: u64 = 256 * 1024 * 1024;

/// One regular file in a provider-resolved language-server package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageServerPackageFile {
    relative_path: PathBuf,
    bytes: Vec<u8>,
    executable: bool,
}

impl LanguageServerPackageFile {
    pub fn regular(
        relative_path: impl Into<PathBuf>,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<Self, LanguageServerDistributionError> {
        Self::new(relative_path.into(), bytes.into(), false)
    }

    pub fn executable(
        relative_path: impl Into<PathBuf>,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<Self, LanguageServerDistributionError> {
        Self::new(relative_path.into(), bytes.into(), true)
    }

    fn new(
        relative_path: PathBuf,
        bytes: Vec<u8>,
        executable: bool,
    ) -> Result<Self, LanguageServerDistributionError> {
        validate_relative_path(&relative_path)?;
        Ok(Self {
            relative_path,
            bytes,
            executable,
        })
    }
}

/// Immutable package produced by a trusted server-specific distribution provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageServerPackage {
    server_id: String,
    version: String,
    executable_path: PathBuf,
    files: Vec<LanguageServerPackageFile>,
}

impl LanguageServerPackage {
    pub fn new(
        server_id: impl Into<String>,
        version: impl Into<String>,
        executable_path: impl Into<PathBuf>,
        mut files: Vec<LanguageServerPackageFile>,
    ) -> Result<Self, LanguageServerDistributionError> {
        let server_id = server_id.into();
        validate_identity("server ID", &server_id)?;
        let version = version.into();
        validate_identity("version", &version)?;
        let executable_path = executable_path.into();
        validate_relative_path(&executable_path)?;
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        if files.is_empty()
            || !files
                .iter()
                .any(|file| file.relative_path == executable_path && file.executable)
        {
            return Err(LanguageServerDistributionError::ExecutableMissing(
                executable_path,
            ));
        }
        if files
            .windows(2)
            .any(|pair| pair[0].relative_path == pair[1].relative_path)
        {
            return Err(LanguageServerDistributionError::DuplicatePackagePath);
        }
        Ok(Self {
            server_id,
            version,
            executable_path,
            files,
        })
    }

    /// Computes the deterministic package digest a provider must bind to trusted release metadata.
    pub fn sha256(&self) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(self.server_id.as_bytes());
        digest.update([0]);
        digest.update(self.version.as_bytes());
        digest.update([0]);
        let executable_path = self.executable_path.to_string_lossy();
        digest.update((executable_path.len() as u64).to_le_bytes());
        digest.update(executable_path.as_bytes());
        for file in &self.files {
            let path = file.relative_path.to_string_lossy();
            digest.update((path.len() as u64).to_le_bytes());
            digest.update(path.as_bytes());
            digest.update([u8::from(file.executable)]);
            digest.update((file.bytes.len() as u64).to_le_bytes());
            digest.update(&file.bytes);
        }
        digest.finalize().into()
    }
}

/// Result of one verified, side-by-side application-managed installation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledLanguageServer {
    server_id: String,
    version: String,
    executable: PathBuf,
    package_sha256: [u8; 32],
}

impl InstalledLanguageServer {
    /// Returns the provider identity bound into the verified installation receipt.
    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    /// Returns the exact side-by-side package version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the installed package entrypoint declared by trusted package metadata.
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// Returns the package digest verified before publication.
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
}

/// Installer rooted in a product-owned directory, independent from PATH and global package tools.
pub struct LanguageServerInstaller {
    root: PathBuf,
}

impl LanguageServerInstaller {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, LanguageServerDistributionError> {
        let root = root.into();
        if root.as_os_str().is_empty() {
            return Err(LanguageServerDistributionError::InvalidInstallRoot);
        }
        Ok(Self { root })
    }

    /// Verifies the provider digest and atomically publishes a versioned package directory.
    ///
    /// Existing versions are never overwritten. Updates install beside the current version; the
    /// product config authority chooses when to activate the returned executable path.
    pub fn install_verified(
        &self,
        package: LanguageServerPackage,
        expected_sha256: [u8; 32],
    ) -> Result<InstalledLanguageServer, LanguageServerDistributionError> {
        let actual = package.sha256();
        if actual != expected_sha256 {
            return Err(LanguageServerDistributionError::DigestMismatch);
        }
        fs::create_dir_all(&self.root)?;
        ensure_directory(&self.root)?;
        let server_root = self.root.join(&package.server_id);
        fs::create_dir_all(&server_root)?;
        ensure_directory(&server_root)?;
        let target = server_root.join(&package.version);
        if path_exists(&target)? {
            return installed_from_receipt(&target, &package, actual);
        }
        let staging_root = self.root.join(".staging");
        fs::create_dir_all(&staging_root)?;
        ensure_directory(&staging_root)?;
        let staging = staging_root.join(format!(
            "{}-{}-{}-{}",
            package.server_id,
            package.version,
            std::process::id(),
            STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&staging)?;
        let mut guard = StagingGuard(Some(staging.clone()));
        for file in &package.files {
            let destination = staging.join(&file.relative_path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)?;
            output.write_all(&file.bytes)?;
            output.sync_all()?;
            set_executable(&destination, file.executable)?;
        }
        let receipt = InstallationReceipt {
            server_id: package.server_id.clone(),
            version: package.version.clone(),
            executable_path: package.executable_path.clone(),
            package_sha256: hex_digest(actual),
            executable_paths: package
                .files
                .iter()
                .filter(|file| file.executable)
                .map(|file| file.relative_path.clone())
                .collect(),
        };
        let receipt_path = staging.join("installation.json");
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&receipt_path)?;
        output.write_all(&serde_json::to_vec_pretty(&receipt)?)?;
        output.sync_all()?;
        fs::rename(&staging, &target)?;
        guard.0 = None;
        Ok(installed_result(&target, &package, actual))
    }

    /// Reopens one exact side-by-side installation and revalidates every installed file.
    ///
    /// Activation receipts use this on every process start. The installation receipt is not
    /// treated as sufficient proof: the package tree is scanned without following links and its
    /// deterministic digest must still match `expected_sha256`.
    pub fn load_installed(
        &self,
        server_id: &str,
        version: &str,
        expected_sha256: [u8; 32],
    ) -> Result<InstalledLanguageServer, LanguageServerDistributionError> {
        validate_identity("server ID", server_id)?;
        validate_identity("version", version)?;
        ensure_directory(&self.root)?;
        let server_root = self.root.join(server_id);
        ensure_directory(&server_root)?;
        let target = server_root.join(version);
        ensure_directory(&target)?;
        let receipt_path = target.join("installation.json");
        if !is_regular_file(&receipt_path, FileKind::Regular) {
            return Err(LanguageServerDistributionError::ExistingInstallationMismatch);
        }
        let receipt: InstallationReceipt = serde_json::from_slice(&fs::read(&receipt_path)?)?;
        if receipt.server_id != server_id
            || receipt.version != version
            || receipt.package_sha256 != hex_digest(expected_sha256)
        {
            return Err(LanguageServerDistributionError::ExistingInstallationMismatch);
        }
        validate_relative_path(&receipt.executable_path)?;
        let executable_paths = receipt.executable_paths();
        let files = installed_package_files(&target, &receipt.executable_path, &executable_paths)?;
        let package =
            LanguageServerPackage::new(server_id, version, receipt.executable_path, files)?;
        if package.sha256() != expected_sha256 {
            return Err(LanguageServerDistributionError::ExistingInstallationMismatch);
        }
        Ok(installed_result(&target, &package, expected_sha256))
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstallationReceipt {
    server_id: String,
    version: String,
    executable_path: PathBuf,
    package_sha256: String,
    #[serde(default)]
    executable_paths: Vec<PathBuf>,
}

impl InstallationReceipt {
    fn executable_paths(&self) -> Vec<PathBuf> {
        if self.executable_paths.is_empty() {
            vec![self.executable_path.clone()]
        } else {
            self.executable_paths.clone()
        }
    }
}

struct StagingGuard(Option<PathBuf>);

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if let Some(path) = self.0.as_ref() {
            let _ = fs::remove_dir_all(path);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LanguageServerDistributionError {
    #[error("language-server install root must not be empty")]
    InvalidInstallRoot,
    #[error("invalid {kind} `{value}`")]
    InvalidIdentity { kind: &'static str, value: String },
    #[error("package path must be non-empty, relative, and contain no parent components: {0}")]
    InvalidPackagePath(PathBuf),
    #[error("package does not contain its declared executable: {0}")]
    ExecutableMissing(PathBuf),
    #[error("package contains a duplicate file path")]
    DuplicatePackagePath,
    #[error("package SHA-256 does not match trusted provider metadata")]
    DigestMismatch,
    #[error("an existing installation does not match the requested package")]
    ExistingInstallationMismatch,
    #[error("installed language-server package exceeds bounded file limits")]
    InstalledPackageTooLarge,
    #[error("language-server activation receipt is invalid")]
    InvalidActivationReceipt,
    #[error("language-server activation authority is unavailable")]
    ActivationUnavailable,
    #[error("language-server install directory is not a real directory: {0}")]
    UnsafeInstallDirectory(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Receipt(#[from] serde_json::Error),
}

fn validate_identity(
    kind: &'static str,
    value: &str,
) -> Result<(), LanguageServerDistributionError> {
    let valid = !value.is_empty()
        && value != "."
        && value != ".."
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    valid
        .then_some(())
        .ok_or_else(|| LanguageServerDistributionError::InvalidIdentity {
            kind,
            value: value.into(),
        })
}

fn ensure_directory(path: &Path) -> Result<(), LanguageServerDistributionError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(LanguageServerDistributionError::UnsafeInstallDirectory(
            path.to_path_buf(),
        ))
    }
}

fn path_exists(path: &Path) -> Result<bool, std::io::Error> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn validate_relative_path(path: &Path) -> Result<(), LanguageServerDistributionError> {
    let valid = !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    valid
        .then_some(())
        .ok_or_else(|| LanguageServerDistributionError::InvalidPackagePath(path.to_path_buf()))
}

fn installed_from_receipt(
    target: &Path,
    package: &LanguageServerPackage,
    digest: [u8; 32],
) -> Result<InstalledLanguageServer, LanguageServerDistributionError> {
    let receipt_path = target.join("installation.json");
    if !is_regular_file(target, FileKind::Directory)
        || !is_regular_file(&receipt_path, FileKind::Regular)
        || !package.files.iter().all(|file| {
            let installed_path = target.join(&file.relative_path);
            is_regular_file(&installed_path, FileKind::Regular)
                && fs::read(&installed_path).is_ok_and(|bytes| bytes == file.bytes)
                && executable_mode_matches(&installed_path, file.executable)
        })
    {
        return Err(LanguageServerDistributionError::ExistingInstallationMismatch);
    }
    let receipt: InstallationReceipt = serde_json::from_slice(&fs::read(receipt_path)?)?;
    if receipt.server_id != package.server_id
        || receipt.version != package.version
        || receipt.executable_path != package.executable_path
        || receipt.package_sha256 != hex_digest(digest)
        || receipt.executable_paths()
            != package
                .files
                .iter()
                .filter(|file| file.executable)
                .map(|file| file.relative_path.clone())
                .collect::<Vec<_>>()
    {
        return Err(LanguageServerDistributionError::ExistingInstallationMismatch);
    }
    Ok(installed_result(target, package, digest))
}

#[derive(Clone, Copy)]
enum FileKind {
    Directory,
    Regular,
}

fn is_regular_file(path: &Path, kind: FileKind) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| match kind {
        FileKind::Directory => metadata.is_dir(),
        FileKind::Regular => metadata.is_file(),
    })
}

#[cfg(unix)]
fn executable_mode_matches(path: &Path, executable: bool) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .is_ok_and(|installed| installed == executable)
}

#[cfg(not(unix))]
fn executable_mode_matches(_path: &Path, _executable: bool) -> bool {
    true
}

fn installed_result(
    target: &Path,
    package: &LanguageServerPackage,
    digest: [u8; 32],
) -> InstalledLanguageServer {
    InstalledLanguageServer {
        server_id: package.server_id.clone(),
        version: package.version.clone(),
        executable: target.join(&package.executable_path),
        package_sha256: digest,
    }
}

fn installed_package_files(
    target: &Path,
    executable_path: &Path,
    executable_paths: &[PathBuf],
) -> Result<Vec<LanguageServerPackageFile>, LanguageServerDistributionError> {
    if !executable_paths.iter().any(|path| path == executable_path)
        || executable_paths
            .iter()
            .any(|path| validate_relative_path(path).is_err())
        || executable_paths.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(LanguageServerDistributionError::ExistingInstallationMismatch);
    }
    let mut pending = vec![target.to_path_buf()];
    let mut files = Vec::new();
    let mut total_bytes = 0_u64;
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() {
                return Err(LanguageServerDistributionError::ExistingInstallationMismatch);
            }
            if metadata.is_dir() {
                pending.push(path);
                continue;
            }
            if !metadata.is_file() || regular_file_has_multiple_links(&metadata) {
                return Err(LanguageServerDistributionError::ExistingInstallationMismatch);
            }
            let relative = path
                .strip_prefix(target)
                .map_err(|_| LanguageServerDistributionError::ExistingInstallationMismatch)?
                .to_path_buf();
            if relative == Path::new("installation.json") {
                continue;
            }
            if files.len() >= MAX_INSTALLED_FILES || metadata.len() > MAX_INSTALLED_FILE_BYTES {
                return Err(LanguageServerDistributionError::InstalledPackageTooLarge);
            }
            total_bytes = total_bytes
                .checked_add(metadata.len())
                .filter(|bytes| *bytes <= MAX_INSTALLED_TOTAL_BYTES)
                .ok_or(LanguageServerDistributionError::InstalledPackageTooLarge)?;
            let bytes = fs::read(&path)?;
            let executable = executable_paths.contains(&relative);
            if !executable_mode_matches(&path, executable) {
                return Err(LanguageServerDistributionError::ExistingInstallationMismatch);
            }
            files.push(LanguageServerPackageFile::new(relative, bytes, executable)?);
        }
    }
    if !files
        .iter()
        .any(|file| file.relative_path == executable_path && file.executable)
    {
        return Err(LanguageServerDistributionError::ExecutableMissing(
            executable_path.to_path_buf(),
        ));
    }
    Ok(files)
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

fn hex_digest(digest: [u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(unix)]
fn set_executable(path: &Path, executable: bool) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(if executable { 0o755 } else { 0o644 });
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path, _executable: bool) -> Result<(), std::io::Error> {
    Ok(())
}

#[cfg(test)]
#[path = "installer_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "activation_tests.rs"]
mod activation_tests;
