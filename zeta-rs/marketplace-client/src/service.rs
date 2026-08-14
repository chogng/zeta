use crate::AcquireCapabilityRequest;
use crate::AcquiredCapability;
use crate::ArtifactHandle;
use crate::DownloadPackageRequest;
use crate::GetPackageRequest;
use crate::InstallPackageRequest;
use crate::InstalledPackage;
use crate::ListInstalledRequest;
use crate::MarketplaceClientError;
use crate::OpenResourceRequest;
use crate::PackageDetails;
use crate::ReleaseCapabilityRequest;
use crate::ResourceContent;
use crate::SearchPackagesRequest;
use crate::SearchPackagesResult;
use crate::UninstallPackageRequest;
use crate::UpdatePackageRequest;

/// Unified product-facing Marketplace service implemented by the local package manager.
///
/// Product callers use this interface for both remote discovery and local lifecycle operations.
/// Implementations must keep remote protocol, cache, package paths, and filesystem layout private.
pub trait MarketplaceServiceClient: Send + Sync {
    fn search(
        &self,
        request: SearchPackagesRequest,
    ) -> Result<SearchPackagesResult, MarketplaceClientError>;

    fn get(&self, request: GetPackageRequest) -> Result<PackageDetails, MarketplaceClientError>;

    fn download(
        &self,
        request: DownloadPackageRequest,
    ) -> Result<ArtifactHandle, MarketplaceClientError>;

    fn install(
        &self,
        request: InstallPackageRequest,
    ) -> Result<InstalledPackage, MarketplaceClientError>;

    fn update(
        &self,
        request: UpdatePackageRequest,
    ) -> Result<InstalledPackage, MarketplaceClientError>;

    fn uninstall(&self, request: UninstallPackageRequest) -> Result<(), MarketplaceClientError>;

    fn list_installed(
        &self,
        request: ListInstalledRequest,
    ) -> Result<Vec<InstalledPackage>, MarketplaceClientError>;

    fn acquire_capability(
        &self,
        request: AcquireCapabilityRequest,
    ) -> Result<AcquiredCapability, MarketplaceClientError>;

    fn release_capability(
        &self,
        request: ReleaseCapabilityRequest,
    ) -> Result<(), MarketplaceClientError>;

    fn open_resource(
        &self,
        request: OpenResourceRequest,
    ) -> Result<ResourceContent, MarketplaceClientError>;
}
