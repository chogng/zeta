#![feature(windows_by_handle)]

#[path = "../../../zeta-rs/plugins/src/error.rs"]
mod error;
#[path = "../../../zeta-rs/plugins/src/identity.rs"]
mod identity;
#[path = "../../../zeta-rs/plugins/src/manifest/mod.rs"]
mod manifest;
mod package;
#[path = "../../../zeta-rs/plugins/src/path.rs"]
mod path;

pub use error::PluginError;
pub use error::PluginErrorKind;
pub use identity::InstalledPluginRef;
pub use identity::InvalidPluginId;
pub use identity::InvalidPluginPackageDigest;
pub use identity::InvalidPluginVersion;
pub use identity::PluginId;
pub use identity::PluginPackageDigest;
pub use identity::PluginVersion;
pub use manifest::AssetContribution;
pub use manifest::ConnectorContribution;
pub use manifest::ContributionKind;
pub use manifest::ContributionReference;
pub use manifest::CredentialKind;
pub use manifest::CredentialSlot;
pub use manifest::EditorExtensionActivationEvent;
pub use manifest::EditorExtensionCapability;
pub use manifest::EditorExtensionContribution;
pub use manifest::EditorExtensionRuntimeApiVersion;
pub use manifest::InvalidContributionReference;
pub use manifest::InvalidManifestLocalId;
pub use manifest::InvalidNetworkHost;
pub use manifest::InvalidVersionRequirement;
pub use manifest::ManifestLocalId;
pub use manifest::McpServerContribution;
pub use manifest::NetworkHost;
pub use manifest::Permission;
pub use manifest::PluginCompatibility;
pub use manifest::PluginContributions;
pub use manifest::PluginManifest;
pub use manifest::SkillContribution;
pub use manifest::WorkspaceAccess;
pub use manifest::ZetaVersionRequirement;
pub use package::LocalPluginCatalog;
pub use package::LocalPluginPackage;
pub use package::PackageFileStats;
pub use package::PluginPackageSource;
pub use path::InvalidPluginPath;
pub use path::PluginPath;
