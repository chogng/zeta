use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use semver::Version;
use serde::Deserialize;
use tempfile::TempDir;
use tough::ExpirationEnforcement;
use tough::FilesystemTransport;
use tough::IntoVec;
use tough::Limits;
use tough::Repository;
use tough::RepositoryLoader;
use tough::TargetName;
use tough::schema::Targets;
use url::Url;
use zeta_http_client::HttpClient;
use zeta_language_server_distribution::LanguageServerActivationAuthority;
use zeta_language_server_distribution::LanguageServerActivationSnapshot;

use crate::LanguageMarketplaceEntry;
use crate::LanguageMarketplaceError;
use crate::LanguageMarketplaceErrorKind;
use crate::LanguageMarketplaceId;
use crate::LanguagePackageDigest;
use crate::LanguagePackageId;
use crate::LanguagePackageVersion;
use crate::archive;
use crate::model::CatalogContext;
use crate::model::PackageCatalogMetadata;
use crate::model::PackageTargetMetadata;
use crate::model::catalog_entries;
use crate::transport::MarketplaceTransport;

const MAX_TRUSTED_ROOT_BYTES: usize = 1024 * 1024;
const MAX_REVOCATIONS_BYTES: usize = 4 * 1024 * 1024;
const LANGUAGE_INDEX_TARGET: &str = "marketplace/languages/index.json";
const REVOCATIONS_TARGET: &str = "marketplace/revocations.json";

/// Host-pinned configuration for one TUF-backed Language Marketplace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteLanguageMarketplaceConfig {
    id: LanguageMarketplaceId,
    metadata_base_url: Url,
    targets_base_url: Url,
    trusted_root: Vec<u8>,
    cache_root: PathBuf,
    consumer_id: String,
    consumer_version: Version,
    allowed_publishers: BTreeSet<String>,
}

impl RemoteLanguageMarketplaceConfig {
    /// Creates one source with an explicit consumer identity/version for compatibility probes.
    pub fn new(
        id: LanguageMarketplaceId,
        metadata_base_url: Url,
        targets_base_url: Url,
        trusted_root: Vec<u8>,
        cache_root: impl Into<PathBuf>,
        consumer_id: impl Into<String>,
        consumer_version: Version,
    ) -> Result<Self, LanguageMarketplaceError> {
        let consumer_id = consumer_id.into();
        if metadata_base_url.scheme() != "https"
            || targets_base_url.scheme() != "https"
            || metadata_base_url.cannot_be_a_base()
            || targets_base_url.cannot_be_a_base()
            || !valid_distribution_base_url(&metadata_base_url)
            || !valid_distribution_base_url(&targets_base_url)
            || trusted_root.is_empty()
            || trusted_root.len() > MAX_TRUSTED_ROOT_BYTES
            || !valid_identifier_segment(&consumer_id)
        {
            return Err(config_error());
        }
        Ok(Self {
            id,
            metadata_base_url,
            targets_base_url,
            trusted_root,
            cache_root: cache_root.into(),
            consumer_id,
            consumer_version,
            allowed_publishers: BTreeSet::new(),
        })
    }

    pub fn id(&self) -> &LanguageMarketplaceId {
        &self.id
    }

    /// Restricts all signed publisher roles accepted from this external source.
    pub fn with_allowed_publishers(
        mut self,
        publishers: impl IntoIterator<Item = String>,
    ) -> Result<Self, LanguageMarketplaceError> {
        let publishers = publishers.into_iter().collect::<Vec<_>>();
        let unique = publishers.iter().cloned().collect::<BTreeSet<_>>();
        if unique.is_empty()
            || unique.len() != publishers.len()
            || unique
                .iter()
                .any(|publisher| !valid_identifier_segment(publisher))
        {
            return Err(config_error());
        }
        self.allowed_publishers = unique;
        Ok(self)
    }
}

/// One immutable catalog generation verified from current unexpired TUF metadata.
#[derive(Clone, Debug)]
pub struct RemoteLanguageMarketplaceSnapshot {
    entries: Vec<LanguageMarketplaceEntry>,
    targets_version: u64,
}

impl RemoteLanguageMarketplaceSnapshot {
    pub fn entries(&self) -> &[LanguageMarketplaceEntry] {
        &self.entries
    }

    pub const fn targets_version(&self) -> u64 {
        self.targets_version
    }
}

/// Synchronous product boundary for a remote signed Language Marketplace.
pub struct RemoteLanguageMarketplace {
    config: RemoteLanguageMarketplaceConfig,
    http: Arc<dyn HttpClient>,
    operation: Mutex<()>,
}

impl RemoteLanguageMarketplace {
    pub fn new(config: RemoteLanguageMarketplaceConfig, http: Arc<dyn HttpClient>) -> Self {
        Self {
            config,
            http,
            operation: Mutex::new(()),
        }
    }

    pub fn id(&self) -> &LanguageMarketplaceId {
        &self.config.id
    }

    /// Refreshes signed metadata, falling back only to an unexpired completely cached repository.
    pub fn sync(&self) -> Result<RemoteLanguageMarketplaceSnapshot, LanguageMarketplaceError> {
        let _guard = self.operation.lock().map_err(|_| cache_error())?;
        prepare_cache_root(&self.config.cache_root)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|_| cache_error())?;
        runtime.block_on(self.sync_async())
    }

    /// Downloads, verifies, installs and durably activates one exact signed catalog entry.
    pub fn install(
        &self,
        requested: &LanguageMarketplaceEntry,
        authority: &LanguageServerActivationAuthority,
    ) -> Result<LanguageServerActivationSnapshot, LanguageMarketplaceError> {
        if requested.marketplace_id != self.config.id {
            return Err(metadata_error());
        }
        if !requested.compatibility.is_compatible() {
            return Err(LanguageMarketplaceError::new(
                LanguageMarketplaceErrorKind::Incompatible,
                "Language Marketplace package is incompatible with this build",
            ));
        }
        let _guard = self.operation.lock().map_err(|_| cache_error())?;
        prepare_cache_root(&self.config.cache_root)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|_| cache_error())?;
        let package_root = runtime.block_on(self.materialize_exact(requested))?;
        let package = archive::language_server_package(&package_root, requested)?;
        let digest = package.sha256();
        let installed = authority
            .installer()
            .install_verified(package, digest)
            .map_err(|_| activation_error())?;
        authority
            .activate(installed)
            .map_err(|_| activation_error())
    }

    async fn sync_async(
        &self,
    ) -> Result<RemoteLanguageMarketplaceSnapshot, LanguageMarketplaceError> {
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
            Err(tough::error::Error::Transport { .. }) => {
                return self.open_cached_snapshot().await;
            }
            Err(_) => return Err(metadata_error()),
        };
        let snapshot = snapshot_from_repository(&self.config, &repository).await?;
        cache_repository(&self.config, &repository).await?;
        replace_directory(datastore, &datastore_root)?;
        Ok(snapshot)
    }

    async fn open_cached_snapshot(
        &self,
    ) -> Result<RemoteLanguageMarketplaceSnapshot, LanguageMarketplaceError> {
        let repository = open_cached_repository(&self.config).await?;
        snapshot_from_repository(&self.config, &repository).await
    }

    async fn materialize_exact(
        &self,
        requested: &LanguageMarketplaceEntry,
    ) -> Result<PathBuf, LanguageMarketplaceError> {
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
        let (repository, remote_source) = match remote {
            Ok(repository) => (repository, true),
            Err(tough::error::Error::Transport { .. }) => {
                (open_cached_repository(&self.config).await?, false)
            }
            Err(_) => return Err(metadata_error()),
        };
        let snapshot = snapshot_from_repository(&self.config, &repository).await?;
        let exact = snapshot
            .entries
            .iter()
            .find(|entry| exact_entry(entry, requested))
            .ok_or_else(metadata_error)?;
        let path = materialize_package(&repository, exact, &self.config.cache_root).await?;
        if remote_source {
            cache_repository(&self.config, &repository).await?;
            replace_directory(datastore, &datastore_root)?;
        }
        Ok(path)
    }
}

async fn snapshot_from_repository(
    config: &RemoteLanguageMarketplaceConfig,
    repository: &Repository,
) -> Result<RemoteLanguageMarketplaceSnapshot, LanguageMarketplaceError> {
    validate_language_index(repository).await?;
    let revoked = read_revocations(repository).await?;
    let mut publishers = BTreeSet::new();
    let mut entries = BTreeMap::new();
    collect_entries(
        &repository.targets().signed,
        config,
        &revoked,
        &mut publishers,
        &mut entries,
    )?;
    if !config.allowed_publishers.is_empty()
        && !publishers
            .iter()
            .all(|publisher| config.allowed_publishers.contains(publisher))
    {
        return Err(metadata_error());
    }
    Ok(RemoteLanguageMarketplaceSnapshot {
        entries: entries.into_values().collect(),
        targets_version: repository.targets().signed.version.get(),
    })
}

fn collect_entries(
    targets: &Targets,
    config: &RemoteLanguageMarketplaceConfig,
    revoked: &BTreeSet<ExactPackageRef>,
    publishers: &mut BTreeSet<String>,
    entries: &mut BTreeMap<
        (LanguagePackageId, LanguagePackageVersion, String),
        LanguageMarketplaceEntry,
    >,
) -> Result<(), LanguageMarketplaceError> {
    let Some(delegations) = &targets.delegations else {
        return Ok(());
    };
    for role in &delegations.roles {
        let Some(signed) = role.targets.as_ref() else {
            continue;
        };
        for (name, target) in &signed.signed.targets {
            let Some(identity) = target.custom.get("marketplacePackage").cloned() else {
                continue;
            };
            let identity: PackageTargetMetadata =
                serde_json::from_value(identity).map_err(|_| metadata_error())?;
            let publisher = identity.id.publisher();
            if role.name != format!("publishers/{publisher}") {
                return Err(metadata_error());
            }
            publishers.insert(publisher.to_owned());
            let Some(catalog_value) = target.custom.get("marketplaceCatalog").cloned() else {
                return Err(metadata_error());
            };
            if catalog_value
                .pointer("/manifest/packageType")
                .and_then(serde_json::Value::as_str)
                != Some("language")
            {
                continue;
            }
            let catalog: PackageCatalogMetadata =
                serde_json::from_value(catalog_value).map_err(|_| metadata_error())?;
            let exact = ExactPackageRef {
                id: identity.id.clone(),
                version: identity.version.clone(),
                digest: identity.package_digest.clone(),
            };
            if revoked.contains(&exact) {
                continue;
            }
            for entry in catalog_entries(CatalogContext {
                marketplace_id: &config.id,
                package: identity,
                catalog,
                target_name: name.raw(),
                target_length: target.length,
                consumer_id: &config.consumer_id,
                consumer_version: &config.consumer_version,
            })? {
                let key = (
                    entry.package_id.clone(),
                    entry.version.clone(),
                    entry.server_id.clone(),
                );
                if let Some(existing) = entries.insert(key, entry.clone())
                    && existing != entry
                {
                    return Err(metadata_error());
                }
            }
        }
        collect_entries(&signed.signed, config, revoked, publishers, entries)?;
    }
    Ok(())
}

async fn validate_language_index(repository: &Repository) -> Result<(), LanguageMarketplaceError> {
    let name = TargetName::new(LANGUAGE_INDEX_TARGET).map_err(|_| metadata_error())?;
    let target = repository
        .targets()
        .signed
        .targets
        .get(&name)
        .ok_or_else(metadata_error)?;
    let bytes = read_target(repository, &name, target.length, MAX_REVOCATIONS_BYTES).await?;
    let document: LanguageIndexDocument =
        serde_json::from_slice(&bytes).map_err(|_| metadata_error())?;
    if document.schema_version != 2 || !document.languages.is_array() {
        return Err(metadata_error());
    }
    Ok(())
}

async fn read_revocations(
    repository: &Repository,
) -> Result<BTreeSet<ExactPackageRef>, LanguageMarketplaceError> {
    let name = TargetName::new(REVOCATIONS_TARGET).map_err(|_| metadata_error())?;
    let target = repository
        .targets()
        .signed
        .targets
        .get(&name)
        .ok_or_else(metadata_error)?;
    let bytes = read_target(repository, &name, target.length, MAX_REVOCATIONS_BYTES).await?;
    let document: RevocationDocument =
        serde_json::from_slice(&bytes).map_err(|_| metadata_error())?;
    if document.schema_version != 1 {
        return Err(metadata_error());
    }
    let mut revoked = BTreeSet::new();
    for package in document.revoked {
        let key = (package.id.clone(), package.version.clone());
        if let Some(existing) = revoked.iter().find(|existing: &&ExactPackageRef| {
            (existing.id.clone(), existing.version.clone()) == key
        }) && *existing != package
        {
            return Err(metadata_error());
        }
        revoked.insert(package);
    }
    Ok(revoked)
}

async fn materialize_package(
    repository: &Repository,
    entry: &LanguageMarketplaceEntry,
    cache_root: &Path,
) -> Result<PathBuf, LanguageMarketplaceError> {
    let packages_root = cache_root.join("packages");
    fs::create_dir_all(&packages_root).map_err(|_| cache_error())?;
    let destination = packages_root.join(
        entry
            .digest
            .as_str()
            .strip_prefix("sha256:")
            .ok_or_else(metadata_error)?,
    );
    if destination.exists() {
        archive::verify_package(
            &destination,
            &entry.digest,
            entry.package_file_count,
            entry.package_size_bytes,
        )?;
        return Ok(destination);
    }
    let name = TargetName::new(&entry.target_name).map_err(|_| metadata_error())?;
    let bytes = read_target(
        repository,
        &name,
        entry.target_length,
        archive::MAX_ARCHIVE_BYTES as usize,
    )
    .await?;
    let staging = tempfile::Builder::new()
        .prefix(".language-package-")
        .tempdir_in(&packages_root)
        .map_err(|_| cache_error())?;
    archive::extract(
        &bytes,
        staging.path(),
        entry.package_file_count,
        entry.package_size_bytes,
    )?;
    archive::verify_package(
        staging.path(),
        &entry.digest,
        entry.package_file_count,
        entry.package_size_bytes,
    )?;
    promote_snapshot(staging, &destination)?;
    archive::verify_package(
        &destination,
        &entry.digest,
        entry.package_file_count,
        entry.package_size_bytes,
    )?;
    Ok(destination)
}

async fn cache_repository(
    config: &RemoteLanguageMarketplaceConfig,
    repository: &Repository,
) -> Result<(), LanguageMarketplaceError> {
    let root = config.cache_root.join("repository");
    let parent = root.parent().ok_or_else(cache_error)?;
    let staging = tempfile::Builder::new()
        .prefix(".language-repository-")
        .tempdir_in(parent)
        .map_err(|_| cache_error())?;
    let metadata = staging.path().join("metadata");
    let targets = staging.path().join("targets");
    let target_names = [LANGUAGE_INDEX_TARGET, REVOCATIONS_TARGET];
    repository
        .cache(&metadata, &targets, Some(&target_names), true)
        .await
        .map_err(|_| cache_error())?;
    replace_directory(staging, &root)
}

async fn open_cached_repository(
    config: &RemoteLanguageMarketplaceConfig,
) -> Result<Repository, LanguageMarketplaceError> {
    let repository_root = config.cache_root.join("repository");
    let metadata = directory_url(&repository_root.join("metadata"))?;
    let targets = directory_url(&repository_root.join("targets"))?;
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

async fn read_target(
    repository: &Repository,
    name: &TargetName,
    signed_length: u64,
    limit: usize,
) -> Result<Vec<u8>, LanguageMarketplaceError> {
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

fn exact_entry(left: &LanguageMarketplaceEntry, right: &LanguageMarketplaceEntry) -> bool {
    left == right
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LanguageIndexDocument {
    schema_version: u32,
    languages: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RevocationDocument {
    schema_version: u32,
    #[serde(default)]
    revoked: Vec<ExactPackageRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExactPackageRef {
    id: LanguagePackageId,
    version: LanguagePackageVersion,
    digest: LanguagePackageDigest,
}

fn prepare_cache_root(root: &Path) -> Result<(), LanguageMarketplaceError> {
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

fn recover_complete_directory(root: &Path, name: &str) -> Result<(), LanguageMarketplaceError> {
    let current = root.join(name);
    let backup = current.with_extension("previous");
    match (current.exists(), backup.exists()) {
        (false, true) => fs::rename(backup, current).map_err(|_| cache_error()),
        (true, true) => fs::remove_dir_all(backup).map_err(|_| cache_error()),
        _ => Ok(()),
    }
}

fn stage_datastore(root: &Path, current: &Path) -> Result<TempDir, LanguageMarketplaceError> {
    let staging = tempfile::Builder::new()
        .prefix(".language-tuf-state-")
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
        if !entry.file_type().map_err(|_| cache_error())?.is_file() {
            return Err(cache_error());
        }
        fs::copy(entry.path(), staging.path().join(entry.file_name()))
            .map_err(|_| cache_error())?;
    }
    Ok(staging)
}

fn promote_snapshot(staging: TempDir, destination: &Path) -> Result<(), LanguageMarketplaceError> {
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

fn replace_directory(staging: TempDir, destination: &Path) -> Result<(), LanguageMarketplaceError> {
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

fn directory_url(path: &Path) -> Result<Url, LanguageMarketplaceError> {
    Url::from_directory_path(path).map_err(|_| cache_error())
}

fn valid_distribution_base_url(url: &Url) -> bool {
    url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path().ends_with('/')
}

fn valid_identifier_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn config_error() -> LanguageMarketplaceError {
    LanguageMarketplaceError::new(
        LanguageMarketplaceErrorKind::InvalidConfiguration,
        "remote Language Marketplace configuration is invalid",
    )
}

fn metadata_error() -> LanguageMarketplaceError {
    LanguageMarketplaceError::new(
        LanguageMarketplaceErrorKind::MetadataUntrusted,
        "Language Marketplace metadata could not be verified",
    )
}

fn distribution_error() -> LanguageMarketplaceError {
    LanguageMarketplaceError::new(
        LanguageMarketplaceErrorKind::DistributionUnavailable,
        "Language Marketplace distribution is unavailable",
    )
}

fn cache_error() -> LanguageMarketplaceError {
    LanguageMarketplaceError::new(
        LanguageMarketplaceErrorKind::CacheUnavailable,
        "Language Marketplace cache is unavailable",
    )
}

fn activation_error() -> LanguageMarketplaceError {
    LanguageMarketplaceError::new(
        LanguageMarketplaceErrorKind::ActivationUnavailable,
        "Language Marketplace package could not be activated",
    )
}
