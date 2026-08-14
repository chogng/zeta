use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;
use tough::Repository;
use tough::TargetName;
use tough::schema::Targets;
use zeta_plugins::InstalledPluginRef;
use zeta_plugins::PackageFileStats;
use zeta_plugins::PluginId;
use zeta_plugins::PluginManifest;
use zeta_plugins::PluginPackageDigest;
use zeta_plugins::PluginPackageDigestAlgorithm;
use zeta_plugins::PluginVersion;

use crate::RemoteMarketplaceError;
use crate::RemoteMarketplaceErrorKind;

pub(crate) const REVOCATIONS_TARGET: &str = "marketplace/revocations.json";
pub(crate) const LEGACY_REVOCATIONS_TARGET: &str = "zeta/revocations.json";

#[derive(Clone, Debug)]
pub(crate) struct PublishedPluginTarget {
    pub name: TargetName,
    pub package: InstalledPluginRef,
    pub length: u64,
    pub catalog: Option<PublishedPluginCatalog>,
    pub digest_algorithm: PluginPackageDigestAlgorithm,
}

#[derive(Clone, Debug)]
pub(crate) struct PublishedPluginCatalog {
    pub manifest: PluginManifest,
    pub stats: PackageFileStats,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PackageTargetMetadata {
    schema_version: u32,
    id: PluginId,
    version: PluginVersion,
    package_digest: PluginPackageDigest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MarketplaceTargetCatalogMetadata {
    schema_version: u32,
    manifest: Value,
    #[serde(default)]
    consumer_metadata: BTreeMap<String, Value>,
    package_file_count: u64,
    package_size_bytes: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarketplaceManifestIdentity {
    schema_version: u32,
    package_type: String,
    id: PluginId,
    version: PluginVersion,
}

impl MarketplaceTargetCatalogMetadata {
    pub(crate) fn into_zeta_catalog(
        self,
        package: &InstalledPluginRef,
    ) -> Result<Option<PublishedPluginCatalog>, RemoteMarketplaceError> {
        let identity: MarketplaceManifestIdentity =
            serde_json::from_value(self.manifest).map_err(|_| metadata_error())?;
        if identity.package_type != "plugin" {
            return Ok(None);
        }
        let Some(metadata) = self.consumer_metadata.get("zeta") else {
            return Ok(None);
        };
        if self.schema_version != 1
            || !matches!(identity.schema_version, 1 | 2)
            || identity.id != package.id
            || identity.version != package.version
            || !valid_stats(self.package_file_count, self.package_size_bytes)
        {
            return Err(metadata_error());
        }
        let manifest: PluginManifest =
            serde_json::from_value(metadata.clone()).map_err(|_| metadata_error())?;
        if manifest.id != package.id || manifest.version != package.version {
            return Err(metadata_error());
        }
        Ok(Some(PublishedPluginCatalog {
            manifest,
            stats: PackageFileStats {
                file_count: self.package_file_count,
                total_bytes: self.package_size_bytes,
            },
        }))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyTargetCatalogMetadata {
    schema_version: u32,
    manifest: PluginManifest,
    package_file_count: u64,
    package_size_bytes: u64,
}

impl LegacyTargetCatalogMetadata {
    fn into_catalog(
        self,
        package: &InstalledPluginRef,
    ) -> Result<PublishedPluginCatalog, RemoteMarketplaceError> {
        if self.schema_version != 1
            || self.manifest.id != package.id
            || self.manifest.version != package.version
            || !valid_stats(self.package_file_count, self.package_size_bytes)
        {
            return Err(metadata_error());
        }
        Ok(PublishedPluginCatalog {
            manifest: self.manifest,
            stats: PackageFileStats {
                file_count: self.package_file_count,
                total_bytes: self.package_size_bytes,
            },
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RevocationDocument {
    schema_version: u32,
    #[serde(default)]
    revoked: Vec<InstalledPluginRef>,
}

impl RevocationDocument {
    pub fn parse(bytes: &[u8]) -> Result<Vec<InstalledPluginRef>, RemoteMarketplaceError> {
        let document: Self = serde_json::from_slice(bytes).map_err(|_| metadata_error())?;
        if document.schema_version != 1 {
            return Err(metadata_error());
        }
        let mut unique = BTreeMap::new();
        for package in document.revoked {
            let key = (package.id.clone(), package.version.clone());
            if let Some(existing) = unique.insert(key, package.clone())
                && existing != package
            {
                return Err(metadata_error());
            }
        }
        Ok(unique.into_values().collect())
    }
}

pub(crate) fn published_plugins(
    repository: &Repository,
) -> Result<Vec<PublishedPluginTarget>, RemoteMarketplaceError> {
    let mut packages = BTreeMap::new();
    collect_delegated_targets(&repository.targets().signed, &mut packages, None)?;
    Ok(packages.into_values().collect())
}

pub(crate) fn published_publishers(
    repository: &Repository,
) -> Result<BTreeSet<String>, RemoteMarketplaceError> {
    let mut publishers = BTreeSet::new();
    collect_delegated_targets(
        &repository.targets().signed,
        &mut BTreeMap::new(),
        Some(&mut publishers),
    )?;
    Ok(publishers)
}

pub(crate) fn revocations_target(
    repository: &Repository,
) -> Result<&'static str, RemoteMarketplaceError> {
    for target in [REVOCATIONS_TARGET, LEGACY_REVOCATIONS_TARGET] {
        let name = TargetName::new(target).map_err(|_| metadata_error())?;
        if repository.targets().signed.targets.contains_key(&name) {
            return Ok(target);
        }
    }
    Err(metadata_error())
}

fn collect_delegated_targets(
    targets: &Targets,
    packages: &mut BTreeMap<(PluginId, PluginVersion), PublishedPluginTarget>,
    mut publishers: Option<&mut BTreeSet<String>>,
) -> Result<(), RemoteMarketplaceError> {
    let Some(delegations) = &targets.delegations else {
        return Ok(());
    };
    for role in &delegations.roles {
        let Some(signed) = role.targets.as_ref() else {
            continue;
        };
        for (name, target) in &signed.signed.targets {
            let generic = target.custom.get("marketplacePackage").cloned();
            let legacy = target.custom.get("zetaPlugin").cloned();
            let (metadata, digest_algorithm, prefix, catalog_key) = match (generic, legacy) {
                (Some(metadata), None) => (
                    metadata,
                    PluginPackageDigestAlgorithm::MarketplaceV1,
                    "packages",
                    "marketplaceCatalog",
                ),
                (None, Some(metadata)) => (
                    metadata,
                    PluginPackageDigestAlgorithm::LegacyZetaV1,
                    "plugins",
                    "zetaCatalog",
                ),
                (None, None) => continue,
                (Some(_), Some(_)) => return Err(metadata_error()),
            };
            let metadata: PackageTargetMetadata =
                serde_json::from_value(metadata).map_err(|_| metadata_error())?;
            if metadata.schema_version != 1 {
                return Err(metadata_error());
            }
            let publisher = metadata.id.publisher();
            if role.name != format!("publishers/{publisher}")
                || name.raw()
                    != format!("{prefix}/{}/{}.zip", metadata.id.as_str(), metadata.version)
            {
                return Err(metadata_error());
            }
            if let Some(publishers) = publishers.as_deref_mut() {
                publishers.insert(publisher.to_owned());
            }
            let package = InstalledPluginRef {
                id: metadata.id,
                version: metadata.version,
                digest: metadata.package_digest,
            };
            let catalog = match digest_algorithm {
                PluginPackageDigestAlgorithm::MarketplaceV1 => target
                    .custom
                    .get(catalog_key)
                    .cloned()
                    .ok_or_else(metadata_error)
                    .and_then(|value| {
                        serde_json::from_value::<MarketplaceTargetCatalogMetadata>(value)
                            .map_err(|_| metadata_error())
                    })?
                    .into_zeta_catalog(&package)?,
                PluginPackageDigestAlgorithm::LegacyZetaV1 => target
                    .custom
                    .get(catalog_key)
                    .cloned()
                    .map(serde_json::from_value::<LegacyTargetCatalogMetadata>)
                    .transpose()
                    .map_err(|_| metadata_error())?
                    .map(|catalog| catalog.into_catalog(&package))
                    .transpose()?,
            };
            if digest_algorithm == PluginPackageDigestAlgorithm::MarketplaceV1 && catalog.is_none()
            {
                continue;
            }
            let key = (package.id.clone(), package.version.clone());
            let published = PublishedPluginTarget {
                name: name.clone(),
                package,
                length: target.length,
                catalog,
                digest_algorithm,
            };
            if let Some(existing) = packages.insert(key, published.clone())
                && existing.package != published.package
            {
                return Err(metadata_error());
            }
        }
        collect_delegated_targets(&signed.signed, packages, publishers.as_deref_mut())?;
    }
    Ok(())
}

fn valid_stats(file_count: u64, size_bytes: u64) -> bool {
    file_count > 0
        && file_count <= crate::archive::MAX_ARCHIVE_ENTRIES as u64
        && size_bytes > 0
        && size_bytes <= crate::archive::MAX_EXPANDED_BYTES
}

fn metadata_error() -> RemoteMarketplaceError {
    RemoteMarketplaceError::new(
        RemoteMarketplaceErrorKind::MetadataUntrusted,
        "Plugin Marketplace signed metadata is invalid",
    )
}
