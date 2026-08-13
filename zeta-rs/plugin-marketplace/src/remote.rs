use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

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
use zeta_plugins::PluginMarketplaceMaterializationError;
use zeta_plugins::PluginMarketplacePackageMaterializer;
use zeta_plugins::PluginPackageDigest;
use zeta_plugins::VerifiedRemotePluginPackage;

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
            || !valid_distribution_base_url(&metadata_base_url)
            || !valid_distribution_base_url(&targets_base_url)
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
    pub fn open_cached(&self) -> Result<RemotePluginMarketplaceSnapshot, RemoteMarketplaceError> {
        prepare_cache_root(&self.config.cache_root)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|_| cache_error())?;
        runtime.block_on(self.open_cached_async())
    }

    async fn sync_async(&self) -> Result<RemotePluginMarketplaceSnapshot, RemoteMarketplaceError> {
        let datastore_root = self.config.cache_root.join("tuf");
        let datastore = stage_datastore(&self.config.cache_root, &datastore_root)?;
        let repository = RepositoryLoader::new(
            &self.config.trusted_root,
            self.config.metadata_base_url.clone(),
            self.config.targets_base_url.clone(),
        )
        .transport(MarketplaceTransport::new(Arc::clone(&self.http)))
        .datastore(datastore.path())
        .limits(Limits::default())
        .expiration_enforcement(ExpirationEnforcement::Safe)
        .load()
        .await;
        let repository = match repository {
            Ok(repository) => repository,
            Err(tough::error::Error::Transport { .. }) => return self.open_cached_async().await,
            Err(_) => return Err(metadata_error()),
        };
        let snapshot = self
            .materialize(&repository, RepositorySource::Remote)
            .await?;
        replace_directory(datastore, &datastore_root)?;
        Ok(snapshot)
    }

    async fn open_cached_async(
        &self,
    ) -> Result<RemotePluginMarketplaceSnapshot, RemoteMarketplaceError> {
        let repository = open_cached_repository(&self.config).await?;
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
        let materializer = Arc::new(RemotePackageMaterializer {
            config: self.config.clone(),
            http: Arc::clone(&self.http),
        });
        let packages = catalog_packages(
            repository,
            &published,
            &revoked_set,
            &self.config.cache_root,
            materializer,
        )
        .await?;
        let snapshot_digest = signed_repository_digest(repository)?;
        let marketplace = PluginMarketplace::from_verified_remote(
            self.config.id.clone(),
            snapshot_digest,
            packages,
        )
        .map_err(|_| package_error())?;
        if source == RepositorySource::Remote {
            self.cache_repository(repository).await?;
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
    ) -> Result<(), RemoteMarketplaceError> {
        cache_repository(&self.config, repository).await
    }
}

struct RemotePackageMaterializer {
    config: RemotePluginMarketplaceConfig,
    http: Arc<dyn HttpClient>,
}

impl PluginMarketplacePackageMaterializer for RemotePackageMaterializer {
    fn materialize(
        &self,
        package: &InstalledPluginRef,
    ) -> Result<LocalPluginPackage, PluginMarketplaceMaterializationError> {
        prepare_cache_root(&self.config.cache_root).map_err(remote_plugin_error)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|_| remote_plugin_error(cache_error()))?;
        runtime
            .block_on(self.materialize_async(package))
            .map_err(remote_plugin_error)
    }
}

impl RemotePackageMaterializer {
    async fn materialize_async(
        &self,
        package: &InstalledPluginRef,
    ) -> Result<LocalPluginPackage, RemoteMarketplaceError> {
        let datastore_root = self.config.cache_root.join("tuf");
        let datastore = stage_datastore(&self.config.cache_root, &datastore_root)?;
        let remote = RepositoryLoader::new(
            &self.config.trusted_root,
            self.config.metadata_base_url.clone(),
            self.config.targets_base_url.clone(),
        )
        .transport(MarketplaceTransport::new(Arc::clone(&self.http)))
        .datastore(datastore.path())
        .limits(Limits::default())
        .expiration_enforcement(ExpirationEnforcement::Safe)
        .load()
        .await;
        let (repository, source) = match remote {
            Ok(repository) => (repository, RepositorySource::Remote),
            Err(tough::error::Error::Transport { .. }) => (
                open_cached_repository(&self.config).await?,
                RepositorySource::Cache,
            ),
            Err(_) => return Err(metadata_error()),
        };
        let revoked = read_revocations(&repository)
            .await?
            .into_iter()
            .collect::<BTreeSet<_>>();
        if revoked.contains(package) {
            return Err(package_error());
        }
        let published = metadata::published_plugins(&repository)?;
        let target = published
            .iter()
            .find(|published| &published.package == package)
            .ok_or_else(metadata_error)?;
        let materialized =
            materialize_package(&repository, target, &self.config.cache_root).await?;
        if source == RepositorySource::Remote {
            cache_repository(&self.config, &repository).await?;
            replace_directory(datastore, &datastore_root)?;
        }
        Ok(materialized)
    }
}

async fn cache_repository(
    config: &RemotePluginMarketplaceConfig,
    repository: &Repository,
) -> Result<(), RemoteMarketplaceError> {
    let root = config.cache_root.join("repository");
    let parent = root.parent().ok_or_else(cache_error)?;
    let staging = tempfile::Builder::new()
        .prefix(".repository-")
        .tempdir_in(parent)
        .map_err(|_| cache_error())?;
    let metadata = staging.path().join("metadata");
    let targets = staging.path().join("targets");
    let target_names = [metadata::REVOCATIONS_TARGET.to_owned()];
    repository
        .cache(&metadata, &targets, Some(&target_names), true)
        .await
        .map_err(|_| cache_error())?;
    replace_directory(staging, &root)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RepositorySource {
    Remote,
    Cache,
}

async fn catalog_packages(
    repository: &Repository,
    published: &[PublishedPluginTarget],
    revoked: &BTreeSet<InstalledPluginRef>,
    cache_root: &Path,
    materializer: Arc<RemotePackageMaterializer>,
) -> Result<Vec<VerifiedRemotePluginPackage>, RemoteMarketplaceError> {
    let mut packages = Vec::new();
    for published in published {
        if revoked.contains(&published.package) {
            continue;
        }
        if published.length > archive::MAX_ARCHIVE_BYTES {
            return Err(package_error());
        }
        let catalog = match &published.catalog {
            Some(catalog) => catalog.clone(),
            None => {
                let package = materialize_package(repository, published, cache_root).await?;
                metadata::PublishedPluginCatalog {
                    manifest: package.manifest().clone(),
                    stats: package.stats(),
                }
            }
        };
        packages.push(VerifiedRemotePluginPackage::new(
            catalog.manifest,
            published.package.digest.clone(),
            catalog.stats,
            materializer.clone(),
        ));
    }
    Ok(packages)
}

async fn materialize_package(
    repository: &Repository,
    published: &PublishedPluginTarget,
    cache_root: &Path,
) -> Result<LocalPluginPackage, RemoteMarketplaceError> {
    if published.length > archive::MAX_ARCHIVE_BYTES {
        return Err(package_error());
    }
    let packages_root = cache_root.join("packages");
    fs::create_dir_all(&packages_root).map_err(|_| cache_error())?;
    let digest_path = published
        .package
        .digest
        .as_str()
        .trim_start_matches("sha256:");
    let destination = packages_root.join(digest_path);
    if destination.exists() {
        return load_exact_package(&destination, &published.package);
    }
    let bytes = read_target(
        repository,
        &published.name,
        published.length,
        archive::MAX_ARCHIVE_BYTES as usize,
    )
    .await?;
    let staging = tempfile::Builder::new()
        .prefix(".package-")
        .tempdir_in(&packages_root)
        .map_err(|_| cache_error())?;
    archive::extract(&bytes, staging.path())?;
    load_exact_package(staging.path(), &published.package)?;
    promote_snapshot(staging, &destination)?;
    load_exact_package(&destination, &published.package)
}

fn load_exact_package(
    root: &Path,
    expected: &InstalledPluginRef,
) -> Result<LocalPluginPackage, RemoteMarketplaceError> {
    let package = LocalPluginPackage::load(root).map_err(|_| package_error())?;
    if package.manifest().id != expected.id
        || package.manifest().version != expected.version
        || package.package_digest() != &expected.digest
    {
        return Err(package_error());
    }
    Ok(package)
}

async fn open_cached_repository(
    config: &RemotePluginMarketplaceConfig,
) -> Result<Repository, RemoteMarketplaceError> {
    let repository_root = config.cache_root.join("repository");
    let metadata = directory_url(&repository_root.join("metadata"))?;
    let targets = directory_url(&repository_root.join("targets"))?;
    // Reuse the online datastore so an offline reopen cannot accept metadata older than any
    // signed version this profile has already observed.
    let datastore = config.cache_root.join("tuf");
    fs::create_dir_all(&datastore).map_err(|_| cache_error())?;
    RepositoryLoader::new(&config.trusted_root, metadata, targets)
        .transport(FilesystemTransport)
        .datastore(datastore)
        .limits(Limits::default())
        .expiration_enforcement(ExpirationEnforcement::Safe)
        .load()
        .await
        .map_err(|_| metadata_error())
}

async fn read_revocations(
    repository: &Repository,
) -> Result<Vec<InstalledPluginRef>, RemoteMarketplaceError> {
    let name = TargetName::new(metadata::REVOCATIONS_TARGET).map_err(|_| metadata_error())?;
    let target = repository
        .targets()
        .signed
        .targets
        .get(&name)
        .ok_or_else(metadata_error)?;
    let bytes = read_target(repository, &name, target.length, MAX_REVOCATIONS_BYTES).await?;
    metadata::RevocationDocument::parse(&bytes)
}

async fn read_target(
    repository: &Repository,
    name: &TargetName,
    signed_length: u64,
    limit: usize,
) -> Result<Vec<u8>, RemoteMarketplaceError> {
    if signed_length > limit as u64 {
        return Err(distribution_error());
    }
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

fn signed_repository_digest(
    repository: &Repository,
) -> Result<PluginPackageDigest, RemoteMarketplaceError> {
    let mut hasher = Sha256::new();
    let snapshot = serde_json::to_vec(repository.snapshot()).map_err(|_| metadata_error())?;
    let targets = serde_json::to_vec(repository.targets()).map_err(|_| metadata_error())?;
    hasher.update(snapshot);
    hasher.update(targets);
    PluginPackageDigest::new(format!("sha256:{:x}", hasher.finalize()))
        .map_err(|_| metadata_error())
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
    recover_complete_directory(root, "repository")?;
    recover_complete_directory(root, "tuf")
}

fn recover_complete_directory(root: &Path, name: &str) -> Result<(), RemoteMarketplaceError> {
    let current = root.join(name);
    let backup = current.with_extension("previous");
    match (current.exists(), backup.exists()) {
        (false, true) => fs::rename(backup, current).map_err(|_| cache_error()),
        (true, true) => fs::remove_dir_all(backup).map_err(|_| cache_error()),
        _ => Ok(()),
    }
}

fn stage_datastore(root: &Path, current: &Path) -> Result<TempDir, RemoteMarketplaceError> {
    let staging = tempfile::Builder::new()
        .prefix(".tuf-state-")
        .tempdir_in(root)
        .map_err(|_| cache_error())?;
    if !current.exists() {
        return Ok(staging);
    }
    let metadata = fs::symlink_metadata(current).map_err(|_| cache_error())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(cache_error());
    }
    for entry in fs::read_dir(current).map_err(|_| cache_error())? {
        let entry = entry.map_err(|_| cache_error())?;
        let metadata = entry.file_type().map_err(|_| cache_error())?;
        if !metadata.is_file() {
            return Err(cache_error());
        }
        fs::copy(entry.path(), staging.path().join(entry.file_name()))
            .map_err(|_| cache_error())?;
    }
    Ok(staging)
}

fn valid_distribution_base_url(url: &Url) -> bool {
    url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path().ends_with('/')
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

fn remote_plugin_error(error: RemoteMarketplaceError) -> PluginMarketplaceMaterializationError {
    match error.kind() {
        RemoteMarketplaceErrorKind::PackageUnsafe => {
            PluginMarketplaceMaterializationError::PackageUnsafe
        }
        RemoteMarketplaceErrorKind::InvalidConfiguration
        | RemoteMarketplaceErrorKind::MetadataUntrusted
        | RemoteMarketplaceErrorKind::DistributionUnavailable
        | RemoteMarketplaceErrorKind::CacheUnavailable => {
            PluginMarketplaceMaterializationError::SourceUnavailable
        }
    }
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
