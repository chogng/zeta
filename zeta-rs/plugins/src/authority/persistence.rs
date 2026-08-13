use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use super::ActivePlugin;
use super::InstalledKey;
use crate::InstalledPluginRef;
use crate::PluginError;
use crate::PluginErrorKind;

const AUTHORITY_SCHEMA_VERSION: u32 = 2;
const MAX_AUTHORITY_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PersistedCommandReceipt {
    pub expected_revision: u64,
    pub command_digest: String,
    pub result_revision: u64,
    pub activation_generation: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ActivePluginRecord {
    pub package: InstalledPluginRef,
    pub activation_revision: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PersistedAuthority {
    schema_version: u32,
    pub revision: u64,
    pub activation_generation: u64,
    pub installed: Vec<InstalledPluginRef>,
    #[serde(default)]
    pub enabled: Vec<InstalledPluginRef>,
    #[serde(default)]
    pub granted: Vec<InstalledPluginRef>,
    #[serde(default)]
    pub active: Vec<ActivePluginRecord>,
    pub receipts: BTreeMap<String, PersistedCommandReceipt>,
}

impl PersistedAuthority {
    pub fn empty() -> Self {
        Self {
            schema_version: AUTHORITY_SCHEMA_VERSION,
            revision: 0,
            activation_generation: 1,
            installed: Vec::new(),
            enabled: Vec::new(),
            granted: Vec::new(),
            active: Vec::new(),
            receipts: BTreeMap::new(),
        }
    }

    pub fn from_state(
        revision: u64,
        activation_generation: u64,
        installed: &BTreeMap<InstalledKey, InstalledPluginRef>,
        enabled: &BTreeMap<crate::PluginId, InstalledPluginRef>,
        granted: &BTreeMap<InstalledKey, InstalledPluginRef>,
        active: &BTreeMap<crate::PluginId, ActivePlugin>,
        receipts: &BTreeMap<String, PersistedCommandReceipt>,
    ) -> Self {
        Self {
            schema_version: AUTHORITY_SCHEMA_VERSION,
            revision,
            activation_generation,
            installed: installed.values().cloned().collect(),
            enabled: enabled.values().cloned().collect(),
            granted: granted.values().cloned().collect(),
            active: active
                .values()
                .map(|active| ActivePluginRecord {
                    package: active.package.clone(),
                    activation_revision: active.activation_revision,
                })
                .collect(),
            receipts: receipts.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), PluginError> {
        if !matches!(self.schema_version, 1 | AUTHORITY_SCHEMA_VERSION)
            || self.activation_generation == 0
            || self.active.iter().any(|active| {
                active.activation_revision == 0
                    || active.activation_revision > self.activation_generation
            })
            || self.receipts.values().any(|receipt| {
                receipt.result_revision > self.revision
                    || receipt.activation_generation == 0
                    || receipt.activation_generation > self.activation_generation
                    || receipt.command_digest.len() != 64
            })
        {
            return Err(persistence_error("Plugin authority record is invalid"));
        }
        Ok(())
    }

    pub fn migrate(mut self) -> Self {
        if self.schema_version == 1 {
            self.enabled = self
                .active
                .iter()
                .map(|active| active.package.clone())
                .collect();
            self.granted = self.enabled.clone();
            self.schema_version = AUTHORITY_SCHEMA_VERSION;
        }
        self
    }
}

pub(super) struct FileAuthorityPersistence {
    path: PathBuf,
}

impl FileAuthorityPersistence {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        Ok(Self { path })
    }

    pub fn load(&self) -> Result<Option<PersistedAuthority>, PluginError> {
        let metadata = match fs::metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(io_error(error)),
        };
        if !metadata.is_file() || metadata.len() > MAX_AUTHORITY_BYTES {
            return Err(persistence_error("Plugin authority record is invalid"));
        }
        let bytes = fs::read(&self.path).map_err(io_error)?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| persistence_error("Plugin authority record is invalid"))
    }

    pub fn persist(&self, authority: &PersistedAuthority) -> Result<(), PluginError> {
        let bytes = serde_json::to_vec(authority)
            .map_err(|_| persistence_error("Plugin authority record could not be encoded"))?;
        if bytes.len() as u64 > MAX_AUTHORITY_BYTES {
            return Err(persistence_error(
                "Plugin authority record exceeds its size limit",
            ));
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| persistence_error("Plugin authority path is invalid"))?;
        let staging = parent.join(staging_filename()?);
        let mut options = fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| {
            let mut file = options.open(&staging).map_err(io_error)?;
            file.write_all(&bytes).map_err(io_error)?;
            file.sync_all().map_err(io_error)?;
            promote_file(&staging, &self.path)?;
            sync_directory(parent)
        })();
        if staging.exists() {
            let _ = fs::remove_file(staging);
        }
        result
    }
}

fn staging_filename() -> Result<String, PluginError> {
    let mut random = [0_u8; 16];
    getrandom::getrandom(&mut random)
        .map_err(|_| persistence_error("Plugin authority staging identity is unavailable"))?;
    Ok(format!(
        ".authority-{}.staging",
        random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

#[cfg(not(windows))]
fn promote_file(staging: &Path, destination: &Path) -> Result<(), PluginError> {
    fs::rename(staging, destination).map_err(io_error)
}

#[cfg(windows)]
fn promote_file(staging: &Path, destination: &Path) -> Result<(), PluginError> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING;
    use windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    let staging = staging
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            staging.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        return Err(io_error(std::io::Error::last_os_error()));
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), PluginError> {
    #[cfg(unix)]
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(io_error)?;
    Ok(())
}

fn io_error(_: impl std::fmt::Display) -> PluginError {
    persistence_error("Plugin authority persistence is unavailable")
}

fn persistence_error(message: impl Into<String>) -> PluginError {
    PluginError::new(PluginErrorKind::AuthorityUnavailable, message)
}
