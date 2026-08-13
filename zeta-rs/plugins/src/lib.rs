//! Validation and read-only discovery for declarative Zeta Plugin packages.
//!
//! This crate currently owns Plugin identity, manifest schema, package-relative path validation,
//! deterministic local package digests, local-development discovery, durable installed/active
//! authority, immutable activation snapshots, and exact invocation leases. Grants, credentials,
//! and Skill or MCP runtime behavior remain outside this crate.

mod activation;
mod authority;
mod discovery;
mod error;
mod identity;
mod manifest;
mod marketplace;
mod package;
mod path;

pub use activation::PluginActivationSnapshot;
pub use authority::PluginActivationAuthority;
pub use authority::PluginAuthorityChange;
pub use authority::PluginAuthorityCommand;
pub use authority::PluginAuthorityCommandId;
pub use authority::PluginAuthorityCommandRequest;
pub use authority::PluginAuthorityCommandResult;
pub use authority::PluginAuthorityDisposition;
pub use authority::PluginAuthoritySnapshot;
pub use authority::PluginAuthoritySubscription;
pub use authority::PluginInstallResult;
pub use authority::PluginInvocationFence;
pub use authority::PluginInvocationLease;
pub use discovery::project_local_plugin_discovery;
pub use error::{PluginError, PluginErrorKind};
pub use identity::{
    InstalledPluginRef, InvalidPluginId, InvalidPluginPackageDigest, InvalidPluginVersion,
    PluginId, PluginPackageDigest, PluginVersion,
};
pub use manifest::{
    AssetContribution, ConnectorContribution, ContributionKind, ContributionReference,
    CredentialKind, CredentialSlot, InvalidContributionReference, InvalidManifestLocalId,
    InvalidNetworkHost, InvalidVersionRequirement, ManifestLocalId, McpServerContribution,
    NetworkHost, Permission, PluginCompatibility, PluginContributions, PluginManifest,
    SkillContribution, WorkspaceAccess, ZetaVersionRequirement,
};
pub use marketplace::PluginMarketplace;
pub use marketplace::PluginMarketplaceId;
pub use marketplace::PluginMarketplaceMode;
pub use marketplace::PluginMarketplacePackage;
pub use marketplace::PluginMarketplaceService;
pub use marketplace::PluginProfileRequest;
pub use marketplace::PluginProfileRequestEnablement;
pub use marketplace::PluginProfileResolution;
pub use marketplace::PluginWorkspaceRequestResolution;
pub use package::{
    InstalledPluginPackage, LocalPluginCatalog, LocalPluginPackage, PackageFileStats,
    PluginPackageSource, PluginPackageStore,
};
pub use path::{InvalidPluginPath, PluginPath};
