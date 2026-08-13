use std::collections::BTreeMap;

use serde::Deserialize;
use tough::Repository;
use tough::TargetName;
use tough::schema::Targets;
use zeta_plugins::InstalledPluginRef;
use zeta_plugins::PackageFileStats;
use zeta_plugins::PluginId;
use zeta_plugins::PluginManifest;
use zeta_plugins::PluginPackageDigest;
use zeta_plugins::PluginVersion;

use crate::RemoteMarketplaceError;
use crate::RemoteMarketplaceErrorKind;

pub(crate) const REVOCATIONS_TARGET: &str = "zeta/revocations.json";

#[derive(Clone, Debug)]
pub(crate) struct PublishedPluginTarget {
    pub name: TargetName,
    pub package: InstalledPluginRef,
    pub length: u64,
    pub catalog: Option<PublishedPluginCatalog>,
}

#[derive(Clone, Debug)]
pub(crate) struct PublishedPluginCatalog {
    pub manifest: PluginManifest,
    pub stats: PackageFileStats,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PluginTargetMetadata {
    schema_version: u32,
    id: PluginId,
    version: PluginVersion,
    package_digest: PluginPackageDigest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PluginTargetCatalogMetadata {
    schema_version: u32,
    manifest: PluginManifest,
    package_file_count: u64,
    package_size_bytes: u64,
}

impl PluginTargetCatalogMetadata {
    pub(crate) fn into_catalog(
        self,
        package: &InstalledPluginRef,
    ) -> Result<PublishedPluginCatalog, RemoteMarketplaceError> {
        if self.schema_version != 1
            || self.manifest.id != package.id
            || self.manifest.version != package.version
            || self.package_file_count == 0
            || self.package_file_count > crate::archive::MAX_ARCHIVE_ENTRIES as u64
            || self.package_size_bytes == 0
            || self.package_size_bytes > crate::archive::MAX_EXPANDED_BYTES
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
    collect_delegated_targets(&repository.targets().signed, &mut packages)?;
    Ok(packages.into_values().collect())
}

fn collect_delegated_targets(
    targets: &Targets,
    packages: &mut BTreeMap<(PluginId, PluginVersion), PublishedPluginTarget>,
) -> Result<(), RemoteMarketplaceError> {
    let Some(delegations) = &targets.delegations else {
        return Ok(());
    };
    for role in &delegations.roles {
        let Some(signed) = role.targets.as_ref() else {
            continue;
        };
        for (name, target) in &signed.signed.targets {
            if !name.raw().starts_with("plugins/") {
                continue;
            }
            let metadata = target
                .custom
                .get("zetaPlugin")
                .cloned()
                .ok_or_else(metadata_error)?;
            let metadata: PluginTargetMetadata =
                serde_json::from_value(metadata).map_err(|_| metadata_error())?;
            if metadata.schema_version != 1 {
                return Err(metadata_error());
            }
            let publisher = metadata
                .id
                .as_str()
                .split_once('/')
                .map(|(publisher, _)| publisher)
                .ok_or_else(metadata_error)?;
            if role.name != format!("publishers/{publisher}")
                || name.raw()
                    != format!("plugins/{}/{}.zip", metadata.id.as_str(), metadata.version)
            {
                return Err(metadata_error());
            }
            let package = InstalledPluginRef {
                id: metadata.id,
                version: metadata.version,
                digest: metadata.package_digest,
            };
            let catalog = target
                .custom
                .get("zetaCatalog")
                .cloned()
                .map(serde_json::from_value::<PluginTargetCatalogMetadata>)
                .transpose()
                .map_err(|_| metadata_error())?
                .map(|catalog| catalog.into_catalog(&package))
                .transpose()?;
            let key = (package.id.clone(), package.version.clone());
            let published = PublishedPluginTarget {
                name: name.clone(),
                package,
                length: target.length,
                catalog,
            };
            if let Some(existing) = packages.insert(key, published.clone())
                && existing.package != published.package
            {
                return Err(metadata_error());
            }
        }
        collect_delegated_targets(&signed.signed, packages)?;
    }
    Ok(())
}

fn metadata_error() -> RemoteMarketplaceError {
    RemoteMarketplaceError::new(
        RemoteMarketplaceErrorKind::MetadataUntrusted,
        "Plugin Marketplace signed metadata is invalid",
    )
}
