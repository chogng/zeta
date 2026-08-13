use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use serde::Deserialize;
use serde::Serialize;

use crate::InstalledLanguageServer;
use crate::LanguageServerDistributionError;
use crate::LanguageServerInstaller;
use crate::hex_digest;

const ACTIVATION_SCHEMA_VERSION: u32 = 1;
static ACTIVATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Durable authority selecting one verified installed version for each server identity.
///
/// Product composition opens this authority at the profile language root. Implementations must
/// activate only `InstalledLanguageServer` values returned by this crate; every snapshot reopens
/// and hashes the selected installation before exposing it to provider registry construction.
#[derive(Clone)]
pub struct LanguageServerActivationAuthority {
    inner: Arc<ActivationInner>,
}

struct ActivationInner {
    root: PathBuf,
    installer: LanguageServerInstaller,
    document: Mutex<ActivationDocument>,
}

/// One completely revalidated generation of active language-server installations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageServerActivationSnapshot {
    generation: u64,
    servers: Vec<InstalledLanguageServer>,
}

impl LanguageServerActivationSnapshot {
    /// Returns the monotonic durable activation generation.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns exact, revalidated installations ordered by server identity.
    pub fn servers(&self) -> &[InstalledLanguageServer] {
        &self.servers
    }
}

impl LanguageServerActivationAuthority {
    /// Opens or initializes the authority below one product profile language root.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, LanguageServerDistributionError> {
        let root = root.into();
        let installer = LanguageServerInstaller::new(root.clone())?;
        fs::create_dir_all(&root)?;
        let metadata = fs::symlink_metadata(&root)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LanguageServerDistributionError::UnsafeInstallDirectory(
                root,
            ));
        }
        let path = root.join("activation.json");
        let backup = root.join("activation.previous.json");
        match (path.exists(), backup.exists()) {
            (false, true) => fs::rename(&backup, &path)?,
            (true, true) => fs::remove_file(&backup)?,
            _ => {}
        }
        let document = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ActivationDocument::empty()
            }
            Err(error) => return Err(error.into()),
        };
        document.validate()?;
        let authority = Self {
            inner: Arc::new(ActivationInner {
                root,
                installer,
                document: Mutex::new(document),
            }),
        };
        authority.snapshot()?;
        Ok(authority)
    }

    /// Installs one package through the authority-owned side-by-side installer.
    pub fn installer(&self) -> &LanguageServerInstaller {
        &self.inner.installer
    }

    /// Atomically selects an exact verified installation and returns the new full snapshot.
    pub fn activate(
        &self,
        installed: InstalledLanguageServer,
    ) -> Result<LanguageServerActivationSnapshot, LanguageServerDistributionError> {
        let installed = self.inner.installer.load_installed(
            installed.server_id(),
            installed.version(),
            installed.package_sha256(),
        )?;
        let mut document = self
            .inner
            .document
            .lock()
            .map_err(|_| LanguageServerDistributionError::ActivationUnavailable)?;
        let record = ActivationRecord::from_installed(&installed);
        if document.active.get(installed.server_id()) != Some(&record) {
            document.generation = document
                .generation
                .checked_add(1)
                .ok_or(LanguageServerDistributionError::ActivationUnavailable)?;
            document
                .active
                .insert(installed.server_id().to_owned(), record);
            write_document(&self.inner.root, &document)?;
        }
        snapshot(&self.inner.installer, &document)
    }

    /// Reopens and hashes every selected installation in the current durable generation.
    pub fn snapshot(
        &self,
    ) -> Result<LanguageServerActivationSnapshot, LanguageServerDistributionError> {
        let document = self
            .inner
            .document
            .lock()
            .map_err(|_| LanguageServerDistributionError::ActivationUnavailable)?;
        snapshot(&self.inner.installer, &document)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActivationDocument {
    schema_version: u32,
    generation: u64,
    active: BTreeMap<String, ActivationRecord>,
}

impl ActivationDocument {
    fn empty() -> Self {
        Self {
            schema_version: ACTIVATION_SCHEMA_VERSION,
            generation: 1,
            active: BTreeMap::new(),
        }
    }

    fn validate(&self) -> Result<(), LanguageServerDistributionError> {
        if self.schema_version != ACTIVATION_SCHEMA_VERSION || self.generation == 0 {
            return Err(LanguageServerDistributionError::InvalidActivationReceipt);
        }
        for (server_id, record) in &self.active {
            if server_id != &record.server_id || parse_hex_digest(&record.package_sha256).is_none()
            {
                return Err(LanguageServerDistributionError::InvalidActivationReceipt);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActivationRecord {
    server_id: String,
    version: String,
    package_sha256: String,
}

impl ActivationRecord {
    fn from_installed(installed: &InstalledLanguageServer) -> Self {
        Self {
            server_id: installed.server_id().to_owned(),
            version: installed.version().to_owned(),
            package_sha256: hex_digest(installed.package_sha256()),
        }
    }
}

fn snapshot(
    installer: &LanguageServerInstaller,
    document: &ActivationDocument,
) -> Result<LanguageServerActivationSnapshot, LanguageServerDistributionError> {
    document.validate()?;
    let servers = document
        .active
        .values()
        .map(|record| {
            let digest = parse_hex_digest(&record.package_sha256)
                .ok_or(LanguageServerDistributionError::InvalidActivationReceipt)?;
            installer.load_installed(&record.server_id, &record.version, digest)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LanguageServerActivationSnapshot {
        generation: document.generation,
        servers,
    })
}

fn write_document(
    root: &Path,
    document: &ActivationDocument,
) -> Result<(), LanguageServerDistributionError> {
    let path = root.join("activation.json");
    let backup = root.join("activation.previous.json");
    let staging = root.join(format!(
        ".activation-{}-{}.json",
        std::process::id(),
        ACTIVATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)?;
        output.write_all(&serde_json::to_vec_pretty(document)?)?;
        output.sync_all()?;
        if path.exists() {
            fs::rename(&path, &backup)?;
        }
        if let Err(error) = fs::rename(&staging, &path) {
            if backup.exists() {
                let _ = fs::rename(&backup, &path);
            }
            return Err(error.into());
        }
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}

fn parse_hex_digest(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(digest)
}
