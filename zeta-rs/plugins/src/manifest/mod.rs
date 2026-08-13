mod editor_extension;
mod model;
mod validation;

pub use editor_extension::EditorExtensionActivationEvent;
pub use editor_extension::EditorExtensionCapability;
pub use editor_extension::EditorExtensionContribution;
pub use editor_extension::EditorExtensionRuntimeApiVersion;
pub use model::{
    AssetContribution, ConnectorContribution, ContributionKind, ContributionReference,
    CredentialKind, CredentialSlot, InvalidContributionReference, InvalidManifestLocalId,
    InvalidNetworkHost, InvalidVersionRequirement, ManifestLocalId, McpServerContribution,
    NetworkHost, Permission, PluginCompatibility, PluginContributions, PluginManifest,
    SkillContribution, WorkspaceAccess, ZetaVersionRequirement,
};

pub const PLUGIN_MANIFEST_PATH: &str = ".zeta-plugin/plugin.json";
pub(crate) const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
