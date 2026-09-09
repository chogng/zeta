//! Shared Plugin identities, manifests, paths, and validated package observations.

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
    AssetContribution, ConnectorContribution, ContributionKind, ContributionReference,
    CredentialKind, CredentialSlot, DeclarativeExtensionContribution, DirectoryAccess,
    EditorExtensionActivationEvent, EditorExtensionCapability, EditorExtensionContribution,
    EditorExtensionRuntimeApiVersion, InvalidContributionReference, InvalidManifestLocalId,
    InvalidNetworkHost, InvalidVersionRequirement, ManifestLocalId, McpServerContribution,
    NetworkHost, Permission, PluginCompatibility, PluginContributions, PluginManifest,
    SkillContribution, ZetaVersionRequirement,
};
pub use package::{
    LocalPluginCatalog, LocalPluginPackage, PackageFileStats, PluginPackageDigestAlgorithm,
    PluginPackageSource,
};
pub use path::{InvalidPluginPath, PluginPath};
