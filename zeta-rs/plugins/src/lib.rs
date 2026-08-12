//! Validation and read-only discovery for declarative Zeta Plugin packages.
//!
//! This crate currently owns Plugin identity, manifest schema, package-relative path validation,
//! deterministic local package digests, and local-development discovery. Installation authority,
//! activation, grants, credentials, and Skill or MCP runtime behavior are not implemented here.

mod activation;
mod discovery;
mod error;
mod identity;
mod manifest;
mod package;
mod path;

pub use activation::PluginActivationSnapshot;
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
pub use package::{
    InstalledPluginPackage, LocalPluginCatalog, LocalPluginPackage, PackageFileStats,
    PluginPackageSource, PluginPackageStore,
};
pub use path::{InvalidPluginPath, PluginPath};
