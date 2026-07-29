//! Shared Seti file-icon manifest and resolver.

mod manifest;
mod resolver;

pub use manifest::{
    SetiFileIconAssociations, SetiFileIconManifest, SetiFontDefinition, SetiFontSource,
    SetiIconDefinition, SetiManifestError, bundled_seti_manifest, parse_seti_manifest,
};
pub use resolver::{ResolvedSetiFileIcon, SetiColorScheme, resolve_file_icon};
