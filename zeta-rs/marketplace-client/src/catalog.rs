use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use semver::Version;
use serde::Deserialize;
use tempfile::TempDir;

use crate::AvailableCapability;
use crate::CapabilityKind;
use crate::MarketplaceClientError;
use crate::MarketplacePackagePayload;
use crate::PackageDetails;
use crate::PackageRef;
use crate::PackageSource;
use crate::PackageSummary;
use crate::catalog_provenance::CatalogUpstreamReference;
use crate::remote::RemoteMarketplaceConfig;
use crate::remote::RemotePackageTarget;
use crate::remote::RemoteSource;

const MAX_PACKAGE_ID_BYTES: usize = 128;
const REMOTE_DISCOVERY_REFRESH_INTERVAL: Duration = Duration::from_secs(60);

pub(crate) struct Catalog {
    releases: Mutex<BTreeMap<String, BTreeMap<Version, Release>>>,
    remote: RemoteSource,
    last_remote_refresh: Mutex<Option<Instant>>,
}

#[derive(Clone)]
pub(crate) struct Release {
    pub manifest: CatalogManifest,
    pub target: RemotePackageTarget,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CatalogManifest {
    pub schema_version: u32,
    pub package_type: PackageType,
    pub source: PackageSource,
    pub id: String,
    pub version: Version,
    pub display_name: String,
    pub description: String,
    pub license: String,
    #[serde(default)]
    pub upstream: Option<CatalogUpstreamReference>,
    pub capabilities: Vec<CatalogCapability>,
    #[serde(default)]
    pub languages: Vec<CatalogLanguage>,
    #[serde(default)]
    pub consumers: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PackageType {
    Plugin,
    Skill,
    Mcp,
    Language,
    Theme,
}

impl PackageType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
            Self::Skill => "skill",
            Self::Mcp => "mcp",
            Self::Language => "language",
            Self::Theme => "theme",
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CatalogCapability {
    pub kind: CatalogCapabilityKind,
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub runtime: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CatalogLanguage {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub file_extensions: Vec<String>,
    #[serde(default)]
    pub lsp: bool,
    #[serde(default)]
    pub language_server: Option<String>,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CatalogCapabilityKind {
    Skill,
    Mcp,
    Connector,
    Executable,
    Asset,
}

impl CatalogCapability {
    fn public_kind(&self, package_type: PackageType) -> CapabilityKind {
        match (self.kind, package_type) {
            (CatalogCapabilityKind::Asset, PackageType::Theme) => CapabilityKind::Theme,
            (CatalogCapabilityKind::Asset, PackageType::Language) => CapabilityKind::Language,
            (CatalogCapabilityKind::Skill, _) => CapabilityKind::Skill,
            (CatalogCapabilityKind::Mcp, _) => CapabilityKind::Mcp,
            (CatalogCapabilityKind::Connector, _) => CapabilityKind::Connector,
            (CatalogCapabilityKind::Executable, _) => CapabilityKind::Executable,
            (CatalogCapabilityKind::Asset, _) => CapabilityKind::Asset,
        }
    }
}

/// Manager-only normalized capability layout carried with one verified download.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketplaceInstallCapability {
    pub kind: CapabilityKind,
    pub id: String,
    pub path: String,
    pub runtime: Option<String>,
    pub language_ids: Vec<String>,
}

/// Exact verified package payload downloaded from the remote Marketplace.
///
/// The extracted source directory remains private. The local Manager can copy the payload into its
/// own staging directory but cannot obtain the remote cache path.
pub(crate) struct MarketplaceDownloadedPackage {
    package: PackageRef,
    package_type: String,
    capabilities: Vec<MarketplaceInstallCapability>,
    contents: TempDir,
    expected_file_count: u64,
    expected_size_bytes: u64,
}

impl MarketplaceDownloadedPackage {
    pub(crate) fn new(release: &Release, contents: TempDir) -> Self {
        Self {
            package: release.target.package().clone(),
            package_type: release.manifest.package_type.as_str().to_owned(),
            capabilities: release.install_capabilities(),
            contents,
            expected_file_count: release.target.package_file_count(),
            expected_size_bytes: release.target.package_size_bytes(),
        }
    }
}

impl MarketplacePackagePayload for MarketplaceDownloadedPackage {
    fn package(&self) -> &PackageRef {
        &self.package
    }

    fn package_type(&self) -> &str {
        &self.package_type
    }

    fn capabilities(&self) -> &[MarketplaceInstallCapability] {
        &self.capabilities
    }

    fn expected_file_count(&self) -> u64 {
        self.expected_file_count
    }

    fn expected_size_bytes(&self) -> u64 {
        self.expected_size_bytes
    }

    fn copy_to(&self, destination: &Path) -> Result<(), MarketplaceClientError> {
        copy_tree(self.contents.path(), destination)
    }
}

impl Catalog {
    pub(crate) fn load_remote(
        config: RemoteMarketplaceConfig,
    ) -> Result<Self, MarketplaceClientError> {
        let remote = RemoteSource::new(config)?;
        let releases = index_releases(remote.releases()?)?;
        Ok(Self {
            releases: Mutex::new(releases),
            remote,
            last_remote_refresh: Mutex::new(Some(Instant::now())),
        })
    }

    pub(crate) fn resolve(
        &self,
        package_id: &str,
        version: Option<&str>,
    ) -> Result<Release, MarketplaceClientError> {
        self.refresh_remote_if_stale()?;
        self.resolve_indexed(package_id, version)
    }

    pub(crate) fn resolve_fresh(
        &self,
        package_id: &str,
        version: Option<&str>,
    ) -> Result<Release, MarketplaceClientError> {
        self.refresh_remote()?;
        self.resolve_indexed(package_id, version)
    }

    fn resolve_indexed(
        &self,
        package_id: &str,
        version: Option<&str>,
    ) -> Result<Release, MarketplaceClientError> {
        let releases = self
            .releases
            .lock()
            .map_err(|_| MarketplaceClientError::unavailable())?;
        let versions = releases
            .get(package_id)
            .ok_or_else(MarketplaceClientError::package_not_found)?;
        match version {
            Some(version) => {
                let version = Version::parse(version)
                    .map_err(|_| MarketplaceClientError::version_not_found())?;
                versions
                    .get(&version)
                    .cloned()
                    .ok_or_else(MarketplaceClientError::version_not_found)
            }
            None => versions
                .last_key_value()
                .map(|(_, release)| release.clone())
                .ok_or_else(MarketplaceClientError::version_not_found),
        }
    }

    pub(crate) fn search(
        &self,
        query: &str,
        package_type: Option<&str>,
        limit: usize,
    ) -> Result<Vec<PackageSummary>, MarketplaceClientError> {
        self.refresh_remote_if_stale()?;
        let query = query.trim().to_ascii_lowercase();
        let releases = self
            .releases
            .lock()
            .map_err(|_| MarketplaceClientError::unavailable())?;
        Ok(releases
            .values()
            .filter_map(|versions| versions.last_key_value().map(|(_, release)| release))
            .filter(|release| {
                package_type
                    .is_none_or(|expected| release.manifest.package_type.as_str() == expected)
                    && (query.is_empty()
                        || release.manifest.id.to_ascii_lowercase().contains(&query)
                        || release
                            .manifest
                            .display_name
                            .to_ascii_lowercase()
                            .contains(&query)
                        || release
                            .manifest
                            .description
                            .to_ascii_lowercase()
                            .contains(&query)
                        || release.manifest.upstream.as_ref().is_some_and(|upstream| {
                            upstream.name.to_ascii_lowercase().contains(&query)
                        }))
            })
            .take(limit)
            .map(Release::summary)
            .collect())
    }

    pub(crate) fn materialize(
        &self,
        release: &Release,
    ) -> Result<MarketplaceDownloadedPackage, MarketplaceClientError> {
        let contents = self.remote.materialize(&release.target)?;
        Ok(MarketplaceDownloadedPackage::new(release, contents))
    }

    fn refresh_remote(&self) -> Result<(), MarketplaceClientError> {
        let releases = index_releases(self.remote.releases()?)?;
        *self
            .releases
            .lock()
            .map_err(|_| MarketplaceClientError::unavailable())? = releases;
        *self
            .last_remote_refresh
            .lock()
            .map_err(|_| MarketplaceClientError::unavailable())? = Some(Instant::now());
        Ok(())
    }

    fn refresh_remote_if_stale(&self) -> Result<(), MarketplaceClientError> {
        if self
            .last_remote_refresh
            .lock()
            .map_err(|_| MarketplaceClientError::unavailable())?
            .is_some_and(|refreshed| refreshed.elapsed() < REMOTE_DISCOVERY_REFRESH_INTERVAL)
        {
            return Ok(());
        }
        self.refresh_remote()
    }
}

fn index_releases(
    releases: Vec<Release>,
) -> Result<BTreeMap<String, BTreeMap<Version, Release>>, MarketplaceClientError> {
    let mut indexed = BTreeMap::<String, BTreeMap<Version, Release>>::new();
    for release in releases {
        let id = release.manifest.id.clone();
        let version = release.manifest.version.clone();
        if indexed
            .entry(id)
            .or_default()
            .insert(version, release)
            .is_some()
        {
            return Err(MarketplaceClientError::package_untrusted());
        }
    }
    Ok(indexed)
}

impl Release {
    fn summary(&self) -> PackageSummary {
        PackageSummary {
            id: self.manifest.id.clone(),
            version: self.manifest.version.to_string(),
            package_type: self.manifest.package_type.as_str().to_owned(),
            display_name: self.manifest.display_name.clone(),
            description: self.manifest.description.clone(),
        }
    }

    pub(crate) fn details(&self) -> PackageDetails {
        PackageDetails {
            package: self.target.package().clone(),
            package_type: self.manifest.package_type.as_str().to_owned(),
            display_name: self.manifest.display_name.clone(),
            description: self.manifest.description.clone(),
            license: self.manifest.license.clone(),
            source: self.manifest.source,
            upstream: self
                .manifest
                .upstream
                .as_ref()
                .map(CatalogUpstreamReference::public_reference),
            capabilities: self.available_capabilities(),
        }
    }

    fn available_capabilities(&self) -> Vec<AvailableCapability> {
        self.manifest
            .capabilities
            .iter()
            .map(|capability| AvailableCapability {
                kind: capability.public_kind(self.manifest.package_type),
                id: capability.id.clone(),
                contract_version: "1".into(),
                permissions: Vec::new(),
                authentication_provider: None,
            })
            .collect()
    }

    fn install_capabilities(&self) -> Vec<MarketplaceInstallCapability> {
        self.manifest
            .capabilities
            .iter()
            .map(|capability| MarketplaceInstallCapability {
                kind: capability.public_kind(self.manifest.package_type),
                id: capability.id.clone(),
                path: capability.path.clone(),
                runtime: capability.runtime.clone(),
                language_ids: language_ids_for(&self.manifest, capability),
            })
            .collect()
    }
}

fn language_ids_for(manifest: &CatalogManifest, capability: &CatalogCapability) -> Vec<String> {
    if capability.kind != CatalogCapabilityKind::Executable {
        return Vec::new();
    }
    manifest
        .languages
        .iter()
        .filter(|language| match manifest.schema_version {
            1 => language.lsp,
            2 => language.language_server.as_deref() == Some(capability.id.as_str()),
            _ => false,
        })
        .map(|language| language.id.clone())
        .collect()
}

pub(crate) fn validate_manifest(manifest: &CatalogManifest) -> Result<(), MarketplaceClientError> {
    if !matches!(manifest.schema_version, 1 | 2)
        || !valid_package_id(&manifest.id)
        || manifest.display_name.trim().is_empty()
        || manifest.description.trim().is_empty()
        || manifest.license.trim().is_empty()
        || manifest.capabilities.is_empty()
        || (manifest.schema_version == 2 && !manifest.consumers.is_empty())
    {
        return Err(MarketplaceClientError::package_untrusted());
    }
    if manifest.capabilities.iter().any(|capability| {
        (manifest.schema_version == 2
            && capability.kind == CatalogCapabilityKind::Executable
            && capability.runtime.is_none())
            || (capability.kind != CatalogCapabilityKind::Executable
                && capability.runtime.is_some())
    }) {
        return Err(MarketplaceClientError::package_untrusted());
    }
    if let Some(upstream) = &manifest.upstream {
        upstream.validate(
            manifest.schema_version,
            matches!(manifest.package_type, PackageType::Mcp),
        )?;
    }
    if matches!(manifest.package_type, PackageType::Language) != !manifest.languages.is_empty()
        || manifest.languages.iter().any(|language| {
            language.id.trim().is_empty()
                || language.display_name.trim().is_empty()
                || language.aliases.iter().any(|alias| alias.trim().is_empty())
                || language
                    .file_extensions
                    .iter()
                    .any(|extension| !extension.starts_with('.') || extension.len() < 2)
                || (manifest.schema_version == 1 && language.language_server.is_some())
                || (manifest.schema_version == 2 && language.lsp)
        })
    {
        return Err(MarketplaceClientError::package_untrusted());
    }
    let executable_ids = manifest
        .capabilities
        .iter()
        .filter(|capability| capability.kind == CatalogCapabilityKind::Executable)
        .map(|capability| capability.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if manifest.languages.iter().any(|language| {
        language
            .language_server
            .as_deref()
            .is_some_and(|server| !executable_ids.contains(server))
    }) || (manifest.schema_version == 1
        && manifest.languages.iter().any(|language| language.lsp)
        && executable_ids.len() != 1)
    {
        return Err(MarketplaceClientError::package_untrusted());
    }
    Ok(())
}

fn valid_package_id(value: &str) -> bool {
    let Some((publisher, name)) = value.split_once('/') else {
        return false;
    };
    value.len() <= MAX_PACKAGE_ID_BYTES
        && value.matches('/').count() == 1
        && valid_identifier_segment(publisher)
        && valid_identifier_segment(name)
}

fn valid_identifier_segment(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), MarketplaceClientError> {
    let mut children = fs::read_dir(source)
        .map_err(|_| MarketplaceClientError::storage())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| MarketplaceClientError::storage())?;
    children.sort_by_key(fs::DirEntry::file_name);
    for child in children {
        let source_path = child.path();
        let destination_path = destination.join(child.file_name());
        let metadata =
            fs::symlink_metadata(&source_path).map_err(|_| MarketplaceClientError::storage())?;
        if metadata.file_type().is_symlink() {
            return Err(MarketplaceClientError::storage());
        }
        if metadata.is_dir() {
            fs::create_dir(&destination_path).map_err(|_| MarketplaceClientError::storage())?;
            copy_tree(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path)
                .map_err(|_| MarketplaceClientError::storage())?;
            fs::set_permissions(&destination_path, metadata.permissions())
                .map_err(|_| MarketplaceClientError::storage())?;
        } else {
            return Err(MarketplaceClientError::storage());
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
