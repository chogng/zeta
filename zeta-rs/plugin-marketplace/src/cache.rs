use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use zeta_plugins::PluginPackageDigest;

use crate::RemoteMarketplaceError;
use crate::RemoteMarketplaceErrorKind;

const DEFAULT_MAX_MATERIALIZED_PACKAGES: usize = 32;
const DEFAULT_MAX_MATERIALIZED_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_CONFIGURED_MATERIALIZED_PACKAGES: usize = 4096;
const MAX_CONFIGURED_MATERIALIZED_BYTES: u64 = 128 * 1024 * 1024 * 1024;

/// Product-selected storage budget for verified, materialized Marketplace package snapshots.
///
/// This cache is independent from the Plugin authority's installed content-addressed object
/// store. Eviction may remove offline reinstall bytes, but never removes an installed package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemoteMarketplaceCachePolicy {
    max_materialized_packages: usize,
    max_materialized_bytes: u64,
}

impl Default for RemoteMarketplaceCachePolicy {
    fn default() -> Self {
        Self {
            max_materialized_packages: DEFAULT_MAX_MATERIALIZED_PACKAGES,
            max_materialized_bytes: DEFAULT_MAX_MATERIALIZED_BYTES,
        }
    }
}

impl RemoteMarketplaceCachePolicy {
    /// Sets the maximum number of materialized package snapshots retained by one Marketplace.
    pub fn with_max_materialized_packages(
        mut self,
        maximum: usize,
    ) -> Result<Self, RemoteMarketplaceError> {
        if maximum == 0 || maximum > MAX_CONFIGURED_MATERIALIZED_PACKAGES {
            return Err(policy_error());
        }
        self.max_materialized_packages = maximum;
        Ok(self)
    }

    /// Sets the maximum expanded bytes retained by one Marketplace package cache.
    pub fn with_max_materialized_bytes(
        mut self,
        maximum: u64,
    ) -> Result<Self, RemoteMarketplaceError> {
        if maximum == 0 || maximum > MAX_CONFIGURED_MATERIALIZED_BYTES {
            return Err(policy_error());
        }
        self.max_materialized_bytes = maximum;
        Ok(self)
    }

    pub fn max_materialized_packages(self) -> usize {
        self.max_materialized_packages
    }

    pub fn max_materialized_bytes(self) -> u64 {
        self.max_materialized_bytes
    }
}

/// Result of one materialized package-cache reconciliation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RemoteMarketplaceCacheReport {
    pub retained_packages: usize,
    pub retained_bytes: u64,
    pub evicted_packages: usize,
    pub evicted_bytes: u64,
    pub excess_packages: usize,
    pub excess_bytes: u64,
}

struct CachedPackage {
    digest: PluginPackageDigest,
    path: PathBuf,
    bytes: u64,
    materialized_at: SystemTime,
    published: bool,
}

pub(crate) fn prune(
    cache_root: &Path,
    policy: RemoteMarketplaceCachePolicy,
    published: &BTreeSet<PluginPackageDigest>,
    protected: &BTreeSet<PluginPackageDigest>,
) -> Result<RemoteMarketplaceCacheReport, RemoteMarketplaceError> {
    let packages_root = cache_root.join("packages");
    if !packages_root.exists() {
        return Ok(RemoteMarketplaceCacheReport::default());
    }
    require_real_directory(&packages_root)?;
    let mut report = RemoteMarketplaceCacheReport::default();
    let mut packages = Vec::new();
    for entry in fs::read_dir(&packages_root).map_err(|_| cache_error())? {
        let entry = entry.map_err(|_| cache_error())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let file_type = entry.file_type().map_err(|_| cache_error())?;
        if name.starts_with(".package-") && file_type.is_dir() && !file_type.is_symlink() {
            fs::remove_dir_all(path).map_err(|_| cache_error())?;
            continue;
        }
        if file_type.is_symlink() || !file_type.is_dir() || !is_digest_directory(&name) {
            return Err(cache_error());
        }
        let digest =
            PluginPackageDigest::new(format!("sha256:{name}")).map_err(|_| cache_error())?;
        let metadata = fs::metadata(&path).map_err(|_| cache_error())?;
        let bytes = directory_bytes(&path)?;
        packages.push(CachedPackage {
            digest: digest.clone(),
            path,
            bytes,
            materialized_at: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            published: published.contains(&digest),
        });
    }
    packages.sort_by(|left, right| {
        left.published
            .cmp(&right.published)
            .then_with(|| left.materialized_at.cmp(&right.materialized_at))
            .then_with(|| left.digest.cmp(&right.digest))
    });
    let mut retained_packages = packages.len();
    let mut retained_bytes = packages.iter().map(|package| package.bytes).sum::<u64>();
    for package in packages {
        if retained_packages <= policy.max_materialized_packages
            && retained_bytes <= policy.max_materialized_bytes
        {
            break;
        }
        if protected.contains(&package.digest) {
            continue;
        }
        fs::remove_dir_all(package.path).map_err(|_| cache_error())?;
        retained_packages -= 1;
        retained_bytes = retained_bytes.saturating_sub(package.bytes);
        report.evicted_packages += 1;
        report.evicted_bytes = report.evicted_bytes.saturating_add(package.bytes);
    }
    report.retained_packages = retained_packages;
    report.retained_bytes = retained_bytes;
    report.excess_packages = retained_packages.saturating_sub(policy.max_materialized_packages);
    report.excess_bytes = retained_bytes.saturating_sub(policy.max_materialized_bytes);
    Ok(report)
}

fn require_real_directory(path: &Path) -> Result<(), RemoteMarketplaceError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| cache_error())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(cache_error());
    }
    Ok(())
}

fn is_digest_directory(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn directory_bytes(root: &Path) -> Result<u64, RemoteMarketplaceError> {
    let mut bytes = 0_u64;
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).map_err(|_| cache_error())? {
            let entry = entry.map_err(|_| cache_error())?;
            let file_type = entry.file_type().map_err(|_| cache_error())?;
            if file_type.is_symlink() {
                return Err(cache_error());
            }
            let metadata = entry.metadata().map_err(|_| cache_error())?;
            if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.is_file() {
                bytes = bytes.checked_add(metadata.len()).ok_or_else(cache_error)?;
            } else {
                return Err(cache_error());
            }
        }
    }
    Ok(bytes)
}

fn policy_error() -> RemoteMarketplaceError {
    RemoteMarketplaceError::new(
        RemoteMarketplaceErrorKind::InvalidConfiguration,
        "Plugin Marketplace cache policy is invalid",
    )
}

fn cache_error() -> RemoteMarketplaceError {
    RemoteMarketplaceError::new(
        RemoteMarketplaceErrorKind::CacheUnavailable,
        "Plugin Marketplace cache is unavailable",
    )
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
