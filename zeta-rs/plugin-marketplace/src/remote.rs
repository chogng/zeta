use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use tempfile::TempDir;
use tough::ExpirationEnforcement;
use tough::FilesystemTransport;
use tough::IntoVec;
use tough::Limits;
use tough::Repository;
use tough::RepositoryLoader;
use tough::TargetName;
use url::Url;
use zeta_http_client::HttpClient;
use zeta_plugins::InstalledPluginRef;
use zeta_plugins::LocalPluginPackage;
use zeta_plugins::PluginMarketplace;
use zeta_plugins::PluginMarketplaceId;
use zeta_plugins::PluginMarketplaceMode;

use crate::RemoteMarketplaceError;
use crate::RemoteMarketplaceErrorKind;
use crate::archive;
use crate::metadata;
use crate::metadata::PublishedPluginTarget;
use crate::transport::MarketplaceTransport;

const MAX_TRUSTED_ROOT_BYTES: usize = 1024 * 1024;
const MAX_REVOCATIONS_BYTES: usize = 4 * 1024 * 1024;

/// Host-pinned configuration for one remote, product-managed Marketplace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemotePluginMarketplaceConfig {
    id: PluginMarketplaceId,
    metadata_base_url: Url,
    targets_base_url: Url,
    trusted_root: Vec<u8>,
    cache_root: PathBuf,
}

impl RemotePluginMarketplaceConfig {
    pub fn new(
        id: PluginMarketplaceId,
        metadata_base_url: Url,
        targets_base_url: Url,
        trusted_root: Vec<u8>,
        cache_root: impl Into<PathBuf>,
    ) -> Result<Self, RemoteMarketplaceError> {
        if metadata_base_url.scheme() != "https"
            || targets_base_url.scheme() != "https"
            || metadata_base_url.cannot_be_a_base()
            || targets_base_url.cannot_be_a_base()
            || trusted_root.is_empty()
            || trusted_root.len() > MAX_TRUSTED_ROOT_BYTES
        {
            return Err(config_error());
        }
        Ok(Self {
            id,
            metadata_base_url,
            targets_base_url,
            trusted_root,
            cache_root: cache_root.into(),
        })
    }

    pub fn id(&self) -> &PluginMarketplaceId {
        &self.id
    }
}

/// One verified immutable remote Marketplace cache snapshot.
#[derive(Clone, Debug)]
pub struct RemotePluginMarketplaceSnapshot {
    marketplace: PluginMarketplace,
    revoked: Vec<InstalledPluginRef>,
    targets_version: u64,
}

impl RemotePluginMarketplaceSnapshot {
    pub fn marketplace(&self) -> &PluginMarketplace {
        &self.marketplace
    }

    pub fn into_marketplace(self) -> PluginMarketplace {
        self.marketplace
    }

    pub fn revoked(&self) -> &[InstalledPluginRef] {
        &self.revoked
    }

    pub fn targets_version(&self) -> u64 {
        self.targets_version
    }
}

/// Synchronous product boundary for refreshing one TUF-backed remote Marketplace.
pub struct RemotePluginMarketplace {
    config: RemotePluginMarketplaceConfig,
    http: Arc<dyn HttpClient>,
}

impl RemotePluginMarketplace {
    pub fn new(config: RemotePluginMarketplaceConfig, http: Arc<dyn HttpClient>) -> Self {
        Self { config, http }
    }

    pub fn sync(&self) -> Result<RemotePluginMarketplaceSnapshot, RemoteMarketplaceError> {
        prepare_cache_root(&self.config.cache_root)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|_| cache_error())?;
        runtime.block_on(self.sync_async())
    }

    /// Opens the last completely cached repository while enforcing current metadata expiration.
    ///
    /// Every target is revalidated against cached TUF metadata. The materialized package
    /// directory is never trusted as an offline authority on its own.
    pub fn open_cached(
        &self,
    ) -> Result<RemotePluginMarketplaceSnapshot, RemoteMarketplaceError> {
        prepare_cache_root(&self.config.cache_root)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|_| cache_error())?;
        runtime.block_on(self.open_cached_async())
    }

    async fn sync_async(&self) -> Result<RemotePluginMarketplaceSnapshot, RemoteMarketplaceError> {
        let datastore = self.config.cache_root.join("tuf");
        fs::create_dir_all(&datastore).map_err(|_| cache_error())?;
        let repository = RepositoryLoader::new(
            &self.config.trusted_root,
            self.config.metadata_base_url.clone(),
            self.config.targets_base_url.clone(),
        )
        .transport(MarketplaceTransport::new(Arc::clone(&self.http)))
        .datastore(datastore)
        .limits(Limits::default())
        .expiration_enforcement(ExpirationEnforcement::Safe)
        .load()
        .await;
        let repository = match repository {
            Ok(repository) => repository,
            Err(tough::error::Error::Transport { .. }) => return self.open_cached_async().await,
            Err(_) => return Err(metadata_error()),
        };
        self.materialize(&repository, RepositorySource::Remote).await
    }

    async fn open_cached_async(
        &self,
    ) -> Result<RemotePluginMarketplaceSnapshot, RemoteMarketplaceError> {
        let repository_root = self.config.cache_root.join("repository");
        let metadata = directory_url(&repository_root.join("metadata"))?;
        let targets = directory_url(&repository_root.join("targets"))?;
        let repository = RepositoryLoader::new(&self.config.trusted_root, metadata, targets)
            .transport(FilesystemTransport)
            .datastore(self.config.cache_root.join("offline-tuf"))
            .limits(Limits::default())
            .expiration_enforcement(ExpirationEnforcement::Safe)
            .load()
            .await
            .map_err(|_| metadata_error())?;
        self.materialize(&repository, RepositorySource::Cache).await
    }

    async fn materialize(
        &self,
        repository: &Repository,
        source: RepositorySource,
    ) -> Result<RemotePluginMarketplaceSnapshot, RemoteMarketplaceError> {
        let revoked = read_revocations(repository).await?;
        let revoked_set = revoked.iter().cloned().collect::<BTreeSet<_>>();
        let published = metadata::published_plugins(repository)?;
        let revision = repository.targets().signed.version.get();
        let snapshots = self.config.cache_root.join("snapshots");
        fs::create_dir_all(&snapshots).map_err(|_| cache_error())?;
        let staging = tempfile::Builder::new()
            .prefix(".staging-")
            .tempdir_in(&snapshots)
            .map_err(|_| cache_error())?;
        let entries =
            materialize_packages(repository, &published, &revoked_set, staging.path()).await?;
        write_catalog(staging.path(), &self.config.id, &entries)?;
        let snapshot_digest = signed_repository_digest(repository)?;
        let destination = snapshots.join(snapshot_digest);
        promote_snapshot(staging, &destination)?;
        let marketplace = PluginMarketplace::open(
            &destination,
            PluginMarketplaceMode::RemoteManaged,
        )
        .map_err(|_| package_error())?;
        if source == RepositorySource::Remote {
            self.cache_repository(repository, &published).await?;
        }
        Ok(RemotePluginMarketplaceSnapshot {
            marketplace,
            revoked,
            targets_version: revision,
        })
    }

    async fn cache_repository(
        &self,
        repository: &Repository,
        published: &[PublishedPluginTarget],
    ) -> Result<(), RemoteMarketplaceError> {
        let root = self.config.cache_root.join("repository");
        let parent = root.parent().ok_or_else(cache_error)?;
        let staging = tempfile::Builder::new()
            .prefix(".repository-")
            .tempdir_in(parent)
            .map_err(|_| cache_error())?;
        let metadata = staging.path().join("metadata");
        let targets = staging.path().join("targets");
        let mut target_names = published
            .iter()
            .map(|target| target.name.raw().to_owned())
            .collect::<Vec<_>>();
        target_names.push(metadata::REVOCATIONS_TARGET.to_owned());
        repository
            .cache(&metadata, &targets, Some(&target_names), true)
            .await
            .map_err(|_| cache_error())?;
        replace_directory(staging, &root)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RepositorySource {
    Remote,
    Cache,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogDocument<'a> {
    schema_version: u32,
    id: &'a PluginMarketplaceId,
    plugins: &'a [CatalogEntry],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogEntry {
    id: zeta_plugins::PluginId,
    version: zeta_plugins::PluginVersion,
    digest: zeta_plugins::PluginPackageDigest,
    path: String,
}

async fn materialize_packages(
    repository: &Repository,
    published: &[PublishedPluginTarget],
    revoked: &BTreeSet<InstalledPluginRef>,
    root: &Path,
) -> Result<Vec<CatalogEntry>, RemoteMarketplaceError> {
    let packages_root = root.join("packages");
    fs::create_dir_all(&packages_root).map_err(|_| cache_error())?;
    let mut entries = Vec::new();
    for published in published {
        if revoked.contains(&published.package) {
            continue;
        }
        if published.length > archive::MAX_ARCHIVE_BYTES {
            return Err(package_error());
        }
        let bytes = read_target(
            repository,
            &published.name,
            archive::MAX_ARCHIVE_BYTES as usize,
        )
        .await?;
        let digest_path = published
            .package
            .digest
            .as_str()
            .trim_start_matches("sha256:");
        let package_root = packages_root.join(digest_path);
        archive::extract(&bytes, &package_root)?;
        let package = LocalPluginPackage::load(&package_root).map_err(|_| package_error())?;
        if package.manifest().id != published.package.id
            || package.manifest().version != published.package.version
            || package.package_digest() != &published.package.digest
        {
            return Err(package_error());
        }
        entries.push(CatalogEntry {
            id: published.package.id.clone(),
            version: published.package.version.clone(),
            digest: published.package.digest.clone(),
            path: format!("packages/{digest_path}"),
        });
    }
    entries.sort_by(|left, right| (&left.id, &left.version).cmp(&(&right.id, &right.version)));
    Ok(entries)
}

async fn read_revocations(
    repository: &Repository,
) -> Result<Vec<InstalledPluginRef>, RemoteMarketplaceError> {
    let name = TargetName::new(metadata::REVOCATIONS_TARGET).map_err(|_| metadata_error())?;
    if !repository.targets().signed.targets.contains_key(&name) {
        return Err(metadata_error());
    }
    let bytes = read_target(repository, &name, MAX_REVOCATIONS_BYTES).await?;
    metadata::RevocationDocument::parse(&bytes)
}

async fn read_target(
    repository: &Repository,
    name: &TargetName,
    limit: usize,
) -> Result<Vec<u8>, RemoteMarketplaceError> {
    let stream = repository
        .read_target(name)
        .await
        .map_err(|_| distribution_error())?
        .ok_or_else(distribution_error)?;
    let bytes = stream.into_vec().await.map_err(|_| distribution_error())?;
    if bytes.len() > limit {
        return Err(distribution_error());
    }
    Ok(bytes)
}

fn write_catalog(
    root: &Path,
    id: &PluginMarketplaceId,
    entries: &[CatalogEntry],
) -> Result<(), RemoteMarketplaceError> {
    let directory = root.join(".zeta-marketplace");
    fs::create_dir_all(&directory).map_err(|_| cache_error())?;
    let bytes = serde_json::to_vec(&CatalogDocument {
        schema_version: 1,
        id,
        plugins: entries,
    })
    .map_err(|_| cache_error())?;
    fs::write(directory.join("marketplace.json"), bytes).map_err(|_| cache_error())
}

fn signed_repository_digest(repository: &Repository) -> Result<String, RemoteMarketplaceError> {
    let mut hasher = Sha256::new();
    let snapshot = serde_json::to_vec(repository.snapshot()).map_err(|_| metadata_error())?;
    let targets = serde_json::to_vec(repository.targets()).map_err(|_| metadata_error())?;
    hasher.update(snapshot);
    hasher.update(targets);
    Ok(format!("{:x}", hasher.finalize()))
}

fn promote_snapshot(staging: TempDir, destination: &Path) -> Result<(), RemoteMarketplaceError> {
    if destination.exists() {
        return Ok(());
    }
    let source = staging.keep();
    match fs::rename(&source, destination) {
        Ok(()) => Ok(()),
        Err(_) if destination.exists() => {
            let _ = fs::remove_dir_all(source);
            Ok(())
        }
        Err(_) => Err(cache_error()),
    }
}

fn replace_directory(staging: TempDir, destination: &Path) -> Result<(), RemoteMarketplaceError> {
    let source = staging.keep();
    let backup = destination.with_extension("previous");
    if backup.exists() {
        fs::remove_dir_all(&backup).map_err(|_| cache_error())?;
    }
    if destination.exists() {
        fs::rename(destination, &backup).map_err(|_| cache_error())?;
    }
    if fs::rename(&source, destination).is_err() {
        if backup.exists() {
            let _ = fs::rename(&backup, destination);
        }
        return Err(cache_error());
    }
    if backup.exists() {
        fs::remove_dir_all(backup).map_err(|_| cache_error())?;
    }
    Ok(())
}

fn directory_url(path: &Path) -> Result<Url, RemoteMarketplaceError> {
    Url::from_directory_path(path).map_err(|_| cache_error())
}

fn prepare_cache_root(root: &Path) -> Result<(), RemoteMarketplaceError> {
    if root.exists() {
        let metadata = fs::symlink_metadata(root).map_err(|_| cache_error())?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(cache_error());
        }
    } else {
        fs::create_dir_all(root).map_err(|_| cache_error())?;
    }
    Ok(())
}

fn config_error() -> RemoteMarketplaceError {
    RemoteMarketplaceError::new(
        RemoteMarketplaceErrorKind::InvalidConfiguration,
        "remote Plugin Marketplace configuration is invalid",
    )
}

fn metadata_error() -> RemoteMarketplaceError {
    RemoteMarketplaceError::new(
        RemoteMarketplaceErrorKind::MetadataUntrusted,
        "Plugin Marketplace metadata could not be verified",
    )
}

fn distribution_error() -> RemoteMarketplaceError {
    RemoteMarketplaceError::new(
        RemoteMarketplaceErrorKind::DistributionUnavailable,
        "Plugin Marketplace distribution is unavailable",
    )
}

fn package_error() -> RemoteMarketplaceError {
    RemoteMarketplaceError::new(
        RemoteMarketplaceErrorKind::PackageUnsafe,
        "Plugin Marketplace package did not match signed metadata",
    )
}

fn cache_error() -> RemoteMarketplaceError {
    RemoteMarketplaceError::new(
        RemoteMarketplaceErrorKind::CacheUnavailable,
        "Plugin Marketplace cache is unavailable",
    )
}
