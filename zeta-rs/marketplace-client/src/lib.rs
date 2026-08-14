//! Remote registry adapter and shared Marketplace service contracts for Zeta.
//!
//! Product callers depend on typed search/install/capability contracts. The private remote adapter
//! verifies the current HTTPS/TUF static distribution and hands an opaque payload to Zeta's local
//! Manager; it never exposes remote URLs, cache paths, archives, or extraction directories.

mod archive;
mod catalog;
mod catalog_provenance;
mod client;
mod error;
mod model;
mod remote;
mod service;

pub use catalog::MarketplaceInstallCapability;
pub use client::MarketplacePackagePayload;
pub use client::MarketplaceRegistryClient;
pub use client::MarketplaceRemoteClient;
pub use error::MarketplaceClientError;
pub use error::MarketplaceClientErrorKind;
pub use model::AcquireCapabilityRequest;
pub use model::AcquiredCapability;
pub use model::ActivationSpec;
pub use model::ArtifactHandle;
pub use model::AvailableCapability;
pub use model::CapabilityDescriptor;
pub use model::CapabilityKind;
pub use model::CapabilityLease;
pub use model::CapabilityRef;
pub use model::ConnectorActivationSpec;
pub use model::DownloadPackageRequest;
pub use model::ExecutableActivationSpec;
pub use model::ExecutableRuntime;
pub use model::GetPackageRequest;
pub use model::InstallPackageRequest;
pub use model::InstallationState;
pub use model::InstalledPackage;
pub use model::LanguageActivationSpec;
pub use model::ListInstalledRequest;
pub use model::MarketplaceErrorCode;
pub use model::McpActivationSpec;
pub use model::McpTransportSpec;
pub use model::OpenResourceRequest;
pub use model::PackageDetails;
pub use model::PackageRef;
pub use model::PackageSource;
pub use model::PackageSummary;
pub use model::ReleaseCapabilityRequest;
pub use model::ResourceContent;
pub use model::ResourceRef;
pub use model::SearchPackagesRequest;
pub use model::SearchPackagesResult;
pub use model::SkillActivationSpec;
pub use model::UninstallMode;
pub use model::UninstallPackageRequest;
pub use model::UpdatePackageRequest;
pub use model::UpstreamReference;
pub use model::UpstreamRegistry;
pub use remote::RemoteMarketplaceConfig;
pub use service::MarketplaceServiceClient;
