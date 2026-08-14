use std::path::Path;
use std::sync::Mutex;

use crate::DownloadPackageRequest;
use crate::GetPackageRequest;
use crate::MarketplaceClientError;
use crate::PackageDetails;
use crate::SearchPackagesRequest;
use crate::SearchPackagesResult;
use crate::catalog::Catalog;
use crate::catalog::MarketplaceInstallCapability;
use crate::remote::RemoteMarketplaceConfig;

/// Opaque verified package payload handed from the remote client to the local Manager.
///
/// Implementations must keep remote cache paths private and may only copy into an empty
/// Manager-owned staging directory.
pub trait MarketplacePackagePayload: Send {
    fn package(&self) -> &crate::PackageRef;
    fn package_type(&self) -> &str;
    fn capabilities(&self) -> &[MarketplaceInstallCapability];
    fn expected_file_count(&self) -> u64;
    fn expected_size_bytes(&self) -> u64;
    fn copy_to(&self, destination: &Path) -> Result<(), MarketplaceClientError>;
}

const DEFAULT_SEARCH_LIMIT: usize = 50;
const MAX_SEARCH_LIMIT: usize = 200;

/// Remote Marketplace registry port consumed by the product-local package manager.
///
/// Implementations own discovery, TUF verification, download, and remote cache behavior. They
/// return normalized DTOs and verified payloads; local installation, update, uninstall, leases,
/// and activation remain the Manager's responsibility.
pub trait MarketplaceRegistryClient: Send + Sync {
    fn search(
        &self,
        request: SearchPackagesRequest,
    ) -> Result<SearchPackagesResult, MarketplaceClientError>;

    fn get(&self, request: GetPackageRequest) -> Result<PackageDetails, MarketplaceClientError>;

    fn download(
        &self,
        request: DownloadPackageRequest,
    ) -> Result<Box<dyn MarketplacePackagePayload>, MarketplaceClientError>;
}

/// HTTPS/TUF client for one product-pinned remote Marketplace distribution.
pub struct MarketplaceRemoteClient {
    config: RemoteMarketplaceConfig,
    catalog: Mutex<Option<Catalog>>,
}

impl MarketplaceRemoteClient {
    /// Creates a lazy remote client without making App Server startup depend on network access.
    pub fn open(config: RemoteMarketplaceConfig) -> Result<Self, MarketplaceClientError> {
        Ok(Self {
            config,
            catalog: Mutex::new(None),
        })
    }

    fn with_catalog<T>(
        &self,
        operation: impl FnOnce(&Catalog) -> Result<T, MarketplaceClientError>,
    ) -> Result<T, MarketplaceClientError> {
        let mut catalog = self
            .catalog
            .lock()
            .map_err(|_| MarketplaceClientError::unavailable())?;
        if catalog.is_none() {
            *catalog = Some(Catalog::load_remote(self.config.clone())?);
        }
        operation(catalog.as_ref().expect("catalog was initialized"))
    }
}

impl MarketplaceRegistryClient for MarketplaceRemoteClient {
    fn search(
        &self,
        request: SearchPackagesRequest,
    ) -> Result<SearchPackagesResult, MarketplaceClientError> {
        let limit = request
            .limit
            .unwrap_or(DEFAULT_SEARCH_LIMIT)
            .clamp(1, MAX_SEARCH_LIMIT);
        self.with_catalog(|catalog| {
            Ok(SearchPackagesResult {
                packages: catalog.search(&request.query, request.package_type.as_deref(), limit)?,
            })
        })
    }

    fn get(&self, request: GetPackageRequest) -> Result<PackageDetails, MarketplaceClientError> {
        self.with_catalog(|catalog| {
            Ok(catalog
                .resolve(&request.package_id, request.version.as_deref())?
                .details())
        })
    }

    fn download(
        &self,
        request: DownloadPackageRequest,
    ) -> Result<Box<dyn MarketplacePackagePayload>, MarketplaceClientError> {
        self.with_catalog(|catalog| {
            let release = catalog.resolve_fresh(&request.package_id, request.version.as_deref())?;
            Ok(Box::new(catalog.materialize(&release)?) as Box<dyn MarketplacePackagePayload>)
        })
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
