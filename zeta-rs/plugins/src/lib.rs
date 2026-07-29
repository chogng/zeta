//! Validation and read-only discovery for declarative Zeta Plugin packages.
//!
//! This crate currently owns Plugin identity, manifest schema, package-relative path validation,
//! deterministic local package digests, and local-development discovery. Installation authority,
//! activation, grants, credentials, and Skill or MCP runtime behavior are not implemented here.

mod error;
mod identity;
mod manifest;
mod package;
mod path;

pub use error::{PluginError, PluginErrorKind};
pub use identity::{
    InstalledPluginRef, InvalidPluginId, InvalidPluginPackageDigest, InvalidPluginVersion,
    PluginId, PluginPackageDigest, PluginVersion,
};
pub use manifest::{
    AssetContribution, ContributionKind, ContributionReference, CredentialKind, CredentialSlot,
    InvalidContributionReference, InvalidManifestLocalId, InvalidNetworkHost,
    InvalidVersionRequirement, ManifestLocalId, McpServerContribution, NetworkHost, Permission,
    PluginCompatibility, PluginContributions, PluginManifest, SkillContribution, WorkspaceAccess,
    ZetaVersionRequirement,
};
pub use package::{LocalPluginCatalog, LocalPluginPackage, PackageFileStats, PluginPackageSource};
pub use path::{InvalidPluginPath, PluginPath};
