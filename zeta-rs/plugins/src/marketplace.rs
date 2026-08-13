use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::InstalledPluginRef;
use crate::LocalPluginPackage;
use crate::PackageFileStats;
use crate::PluginActivationAuthority;
use crate::PluginAuthorityCommand;
use crate::PluginAuthorityCommandId;
use crate::PluginAuthorityCommandRequest;
use crate::PluginAuthorityCommandResult;
use crate::PluginError;
use crate::PluginErrorKind;
use crate::PluginId;
use crate::PluginInstallResult;
use crate::PluginPackageDigest;
use crate::PluginPath;
use crate::PluginVersion;
use serde::Deserialize;
use serde::Serialize;

const CATALOG_PATH: &str = ".zeta-marketplace/marketplace.json";
const CATALOG_SCHEMA_VERSION: u32 = 1;
const MAX_CATALOG_BYTES: u64 = 4 * 1024 * 1024;
const MAX_MARKETPLACE_ID_BYTES: usize = 64;

/// Stable identity of one host-registered Plugin Marketplace.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PluginMarketplaceId(String);

impl PluginMarketplaceId {
    pub fn new(value: impl Into<String>) -> Result<Self, PluginError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_MARKETPLACE_ID_BYTES
            || value.starts_with('-')
            || value.ends_with('-')
            || value.contains("--")
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(marketplace_error("Plugin Marketplace identity is invalid"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PluginMarketplaceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Authority by which a Marketplace root entered the process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginMarketplaceMode {
    /// A product-managed catalog selected by the trusted host composition root.
    Managed,
    /// A remotely distributed catalog verified and materialized by the trusted product host.
    RemoteManaged,
    /// An explicitly configured local catalog used while developing Plugins.
    LocalDevelopment,
}

/// One exact digest-pinned package offered by a Marketplace.
#[derive(Clone, Debug)]
pub struct PluginMarketplacePackage {
    marketplace_id: PluginMarketplaceId,
    package: LocalPluginPackage,
}

impl PluginMarketplacePackage {
    pub fn marketplace_id(&self) -> &PluginMarketplaceId {
        &self.marketplace_id
    }

    pub fn package_ref(&self) -> InstalledPluginRef {
        InstalledPluginRef {
            id: self.package.manifest().id.clone(),
            version: self.package.manifest().version.clone(),
            digest: self.package.package_digest().clone(),
        }
    }

    /// Returns the validated manifest used to describe this exact Marketplace package.
    pub fn manifest(&self) -> &crate::PluginManifest {
        self.package.manifest()
    }

    /// Returns bounded package statistics captured by the canonical package validator.
    pub fn stats(&self) -> PackageFileStats {
        self.package.stats()
    }

    pub(crate) fn local_package(&self) -> &LocalPluginPackage {
        &self.package
    }
}

/// Validated snapshot of one host-registered Marketplace catalog.
#[derive(Clone, Debug)]
pub struct PluginMarketplace {
    id: PluginMarketplaceId,
    mode: PluginMarketplaceMode,
    revision: PluginPackageDigest,
    packages: BTreeMap<MarketplacePackageKey, PluginMarketplacePackage>,
}

impl PluginMarketplace {
    /// Loads and validates a catalog and every digest-pinned package it references.
    ///
    /// `root` is supplied only by the product host. Untrusted clients select entries by identity
    /// and never provide filesystem paths to installation operations.
    pub fn open(root: impl AsRef<Path>, mode: PluginMarketplaceMode) -> Result<Self, PluginError> {
        let root = validate_marketplace_root(root.as_ref())?;
        let catalog_path = resolve_marketplace_path(&root, CATALOG_PATH)?;
        let metadata = fs::symlink_metadata(&catalog_path)
            .map_err(|_| marketplace_error("Plugin Marketplace catalog is unavailable"))?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > MAX_CATALOG_BYTES
        {
            return Err(marketplace_error("Plugin Marketplace catalog is unsafe"));
        }
        let bytes = fs::read(&catalog_path)
            .map_err(|_| marketplace_error("Plugin Marketplace catalog cannot be read"))?;
        let document: MarketplaceDocument = serde_json::from_slice(&bytes)
            .map_err(|_| marketplace_error("Plugin Marketplace catalog is invalid"))?;
        if document.schema_version != CATALOG_SCHEMA_VERSION {
            return Err(marketplace_error(
                "Plugin Marketplace catalog schema is unsupported",
            ));
        }
        let id = document.id;
        let mut packages = BTreeMap::new();
        for entry in document.plugins {
            let package_root = resolve_package_root(&root, &entry.path)?;
            let package = LocalPluginPackage::load(package_root)?;
            if package.manifest().id != entry.id
                || package.manifest().version != entry.version
                || package.package_digest() != &entry.digest
            {
                return Err(marketplace_error(
                    "Plugin Marketplace entry does not match exact package content",
                ));
            }
            let key = MarketplacePackageKey {
                id: entry.id,
                version: entry.version,
            };
            let value = PluginMarketplacePackage {
                marketplace_id: id.clone(),
                package,
            };
            if packages.insert(key, value).is_some() {
                return Err(marketplace_error(
                    "Plugin Marketplace contains a duplicate Plugin version",
                ));
            }
        }
        Ok(Self {
            id,
            mode,
            revision: PluginPackageDigest::sha256(bytes),
            packages,
        })
    }

    pub fn id(&self) -> &PluginMarketplaceId {
        &self.id
    }

    pub fn mode(&self) -> PluginMarketplaceMode {
        self.mode
    }

    pub fn revision(&self) -> &PluginPackageDigest {
        &self.revision
    }

    pub fn list(&self) -> Vec<&PluginMarketplacePackage> {
        self.packages.values().collect()
    }

    pub fn resolve(
        &self,
        package: &InstalledPluginRef,
    ) -> Result<&PluginMarketplacePackage, PluginError> {
        let key = MarketplacePackageKey {
            id: package.id.clone(),
            version: package.version.clone(),
        };
        let resolved = self
            .packages
            .get(&key)
            .ok_or_else(|| marketplace_error("Plugin is not available in this Marketplace"))?;
        if resolved.package.package_digest() != &package.digest {
            return Err(marketplace_error(
                "Plugin Marketplace package digest did not match the request",
            ));
        }
        Ok(resolved)
    }
}

/// Marketplace ingestion coordinator over the canonical Plugin authority.
///
/// Installation and update stage immutable exact packages. Grant remains digest-specific. Enable
/// atomically switches effective contributions after the new package is explicitly granted;
/// rollback is the same safe exact-package switch constrained to an older installed version.
#[derive(Clone)]
pub struct PluginMarketplaceService {
    authority: PluginActivationAuthority,
    marketplaces: BTreeMap<PluginMarketplaceId, PluginMarketplace>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginProfileRequestEnablement {
    Disabled,
    Enabled,
}

/// Profile-owned desired Plugin request resolved only through registered Marketplaces.
pub struct PluginProfileRequest {
    pub id: PluginId,
    pub version: PluginVersion,
    pub enablement: PluginProfileRequestEnablement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginProfileResolution {
    pub package: InstalledPluginRef,
    pub installed: bool,
    pub enabled: bool,
    pub granted: bool,
    pub effective: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginWorkspaceRequestResolution {
    AvailableInProfile,
    PendingProfileInstall,
    VersionMismatch,
}

impl PluginMarketplaceService {
    pub fn new(
        authority: PluginActivationAuthority,
        marketplaces: impl IntoIterator<Item = PluginMarketplace>,
    ) -> Result<Self, PluginError> {
        let mut registered = BTreeMap::new();
        for marketplace in marketplaces {
            if registered
                .insert(marketplace.id.clone(), marketplace)
                .is_some()
            {
                return Err(marketplace_error(
                    "Plugin Marketplace identity is registered more than once",
                ));
            }
        }
        Ok(Self {
            authority,
            marketplaces: registered,
        })
    }

    pub fn marketplaces(&self) -> impl Iterator<Item = &PluginMarketplace> {
        self.marketplaces.values()
    }

    pub fn authority(&self) -> &PluginActivationAuthority {
        &self.authority
    }

    /// Resolves one exact version across registered Marketplaces and rejects digest ambiguity.
    pub fn resolve_version(
        &self,
        id: &PluginId,
        version: &PluginVersion,
    ) -> Result<InstalledPluginRef, PluginError> {
        let mut matches = self
            .marketplaces
            .values()
            .flat_map(PluginMarketplace::list)
            .map(PluginMarketplacePackage::package_ref)
            .filter(|package| &package.id == id && &package.version == version);
        let first = matches
            .next()
            .ok_or_else(|| marketplace_error("requested Plugin version is unavailable"))?;
        if matches.any(|package| package.digest != first.digest) {
            return Err(marketplace_error(
                "requested Plugin version is ambiguous across Marketplaces",
            ));
        }
        Ok(first)
    }

    /// Reconciles profile requests without granting any package authority.
    pub fn reconcile_profile(
        &self,
        requests: impl IntoIterator<Item = PluginProfileRequest>,
    ) -> Result<Vec<PluginProfileResolution>, PluginError> {
        let mut resolutions = Vec::new();
        for request in requests {
            let package = self.resolve_version(&request.id, &request.version)?;
            let snapshot = self.authority.snapshot();
            if !snapshot.installed().contains(&package) {
                let source = self
                    .marketplaces
                    .values()
                    .find_map(|marketplace| marketplace.resolve(&package).ok())
                    .ok_or_else(|| marketplace_error("requested Plugin package is unavailable"))?;
                self.authority.install_marketplace(
                    profile_command_id("install", &package, snapshot.revision())?,
                    snapshot.revision(),
                    source,
                )?;
            }
            let snapshot = self.authority.snapshot();
            let current = snapshot
                .enabled()
                .iter()
                .find(|enabled| enabled.id == package.id)
                .cloned();
            match (request.enablement, current) {
                (PluginProfileRequestEnablement::Enabled, Some(current)) if current == package => {}
                (PluginProfileRequestEnablement::Enabled, _) => {
                    self.authority.apply(PluginAuthorityCommandRequest {
                        command_id: profile_command_id("enable", &package, snapshot.revision())?,
                        expected_revision: snapshot.revision(),
                        command: PluginAuthorityCommand::Enable {
                            package: package.clone(),
                        },
                    })?;
                }
                (PluginProfileRequestEnablement::Disabled, Some(current)) if current == package => {
                    self.authority.apply(PluginAuthorityCommandRequest {
                        command_id: profile_command_id("disable", &package, snapshot.revision())?,
                        expected_revision: snapshot.revision(),
                        command: PluginAuthorityCommand::Disable {
                            package: package.clone(),
                        },
                    })?;
                }
                (PluginProfileRequestEnablement::Disabled, _) => {}
            }
            let snapshot = self.authority.snapshot();
            resolutions.push(PluginProfileResolution {
                installed: snapshot.installed().contains(&package),
                enabled: snapshot.enabled().contains(&package),
                granted: snapshot.granted().contains(&package),
                effective: snapshot.activation().packages().iter().any(|active| {
                    active.manifest().id == package.id
                        && active.manifest().version == package.version
                        && active.package_digest() == &package.digest
                }),
                package,
            });
        }
        Ok(resolutions)
    }

    /// Resolves a Workspace request against profile authority without mutating it.
    pub fn resolve_workspace_request(
        &self,
        id: &PluginId,
        version: &PluginVersion,
    ) -> PluginWorkspaceRequestResolution {
        let snapshot = self.authority.snapshot();
        let versions = snapshot
            .installed()
            .iter()
            .filter(|package| &package.id == id)
            .collect::<Vec<_>>();
        if versions.iter().any(|package| &package.version == version) {
            PluginWorkspaceRequestResolution::AvailableInProfile
        } else if versions.is_empty() {
            PluginWorkspaceRequestResolution::PendingProfileInstall
        } else {
            PluginWorkspaceRequestResolution::VersionMismatch
        }
    }

    pub fn install(
        &self,
        command_id: PluginAuthorityCommandId,
        expected_revision: u64,
        marketplace_id: &PluginMarketplaceId,
        package: &InstalledPluginRef,
    ) -> Result<PluginInstallResult, PluginError> {
        let package = self.resolve(marketplace_id, package)?;
        self.authority
            .install_marketplace(command_id, expected_revision, package)
    }

    pub fn stage_update(
        &self,
        command_id: PluginAuthorityCommandId,
        expected_revision: u64,
        marketplace_id: &PluginMarketplaceId,
        package: &InstalledPluginRef,
    ) -> Result<PluginInstallResult, PluginError> {
        let snapshot = self.authority.snapshot();
        let current = snapshot
            .enabled()
            .iter()
            .find(|current| current.id == package.id)
            .or_else(|| {
                snapshot
                    .installed()
                    .iter()
                    .filter(|current| current.id == package.id)
                    .max_by(|left, right| left.version.cmp(&right.version))
            })
            .ok_or_else(|| marketplace_error("Plugin update requires an installed version"))?;
        if package.version <= current.version {
            return Err(marketplace_error(
                "Plugin update target must be newer than the installed version",
            ));
        }
        self.install(command_id, expected_revision, marketplace_id, package)
    }

    pub fn rollback(
        &self,
        command_id: PluginAuthorityCommandId,
        expected_revision: u64,
        package: InstalledPluginRef,
    ) -> Result<PluginAuthorityCommandResult, PluginError> {
        let snapshot = self.authority.snapshot();
        let current = snapshot
            .enabled()
            .iter()
            .find(|current| current.id == package.id)
            .ok_or_else(|| marketplace_error("Plugin rollback requires an enabled version"))?;
        if package.version >= current.version
            || !snapshot.installed().contains(&package)
            || !snapshot.granted().contains(&package)
        {
            return Err(marketplace_error(
                "Plugin rollback target must be an older installed and granted package",
            ));
        }
        self.authority.apply(PluginAuthorityCommandRequest {
            command_id,
            expected_revision,
            command: PluginAuthorityCommand::Enable { package },
        })
    }

    fn resolve(
        &self,
        marketplace_id: &PluginMarketplaceId,
        package: &InstalledPluginRef,
    ) -> Result<&PluginMarketplacePackage, PluginError> {
        self.marketplaces
            .get(marketplace_id)
            .ok_or_else(|| marketplace_error("Plugin Marketplace is not registered"))?
            .resolve(package)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct MarketplacePackageKey {
    id: PluginId,
    version: PluginVersion,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MarketplaceDocument {
    schema_version: u32,
    id: PluginMarketplaceId,
    #[serde(default)]
    plugins: Vec<MarketplaceEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MarketplaceEntry {
    id: PluginId,
    version: PluginVersion,
    digest: PluginPackageDigest,
    path: PluginPath,
}

fn validate_marketplace_root(root: &Path) -> Result<PathBuf, PluginError> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|_| marketplace_error("Plugin Marketplace root is unavailable"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(marketplace_error(
            "Plugin Marketplace root must be a real directory",
        ));
    }
    root.canonicalize()
        .map_err(|_| marketplace_error("Plugin Marketplace root cannot be canonicalized"))
}

fn marketplace_error(message: impl Into<String>) -> PluginError {
    PluginError::new(PluginErrorKind::SourceUnavailable, message)
}

fn profile_command_id(
    action: &str,
    package: &InstalledPluginRef,
    revision: u64,
) -> Result<PluginAuthorityCommandId, PluginError> {
    PluginAuthorityCommandId::new(format!(
        "profile-{action}-{}-{revision}",
        package.digest.as_str().trim_start_matches("sha256:")
    ))
}

fn resolve_package_root(root: &Path, path: &PluginPath) -> Result<PathBuf, PluginError> {
    resolve_marketplace_path(root, path.as_str())
}

fn resolve_marketplace_path(root: &Path, path: &str) -> Result<PathBuf, PluginError> {
    let mut candidate = root.to_path_buf();
    for component in path.split('/') {
        candidate.push(component);
        let metadata = fs::symlink_metadata(&candidate)
            .map_err(|_| marketplace_error("Plugin Marketplace package is unavailable"))?;
        if metadata.file_type().is_symlink() {
            return Err(marketplace_error(
                "Plugin Marketplace package path must not contain links",
            ));
        }
    }
    let canonical = candidate
        .canonicalize()
        .map_err(|_| marketplace_error("Plugin Marketplace package is unavailable"))?;
    if !canonical.starts_with(root) {
        return Err(marketplace_error(
            "Plugin Marketplace package escaped its catalog root",
        ));
    }
    Ok(canonical)
}

#[cfg(test)]
#[path = "marketplace_tests.rs"]
mod tests;
