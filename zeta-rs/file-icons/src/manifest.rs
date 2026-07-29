use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::sync::OnceLock;

/// One font resource referenced by the bundled Seti manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetiFontSource {
    pub path: String,
    pub format: String,
}

/// Font metadata used by browser renderers of the Seti theme.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetiFontDefinition {
    pub id: String,
    pub src: Vec<SetiFontSource>,
    pub weight: String,
    pub style: String,
    pub size: String,
}

/// Theme artwork for one Seti icon identifier.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetiIconDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_character: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
}

/// Filename, extension, and language associations for one color scheme.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetiFileIconAssociations {
    pub file: String,
    pub file_extensions: BTreeMap<String, String>,
    pub file_names: BTreeMap<String, String>,
    pub language_ids: BTreeMap<String, String>,
}

/// Seti resource consumed by both Rust and TypeScript resolvers.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetiFileIconManifest {
    #[serde(rename = "information_for_contributors")]
    pub information_for_contributors: Vec<String>,
    pub fonts: Vec<SetiFontDefinition>,
    pub icon_definitions: BTreeMap<String, SetiIconDefinition>,
    pub file: String,
    pub file_extensions: BTreeMap<String, String>,
    pub file_names: BTreeMap<String, String>,
    pub language_ids: BTreeMap<String, String>,
    pub light: SetiFileIconAssociations,
    pub version: String,
}

impl SetiFileIconManifest {
    /// Checks every association-to-definition reference.
    pub fn validate(&self) -> Result<(), SetiManifestError> {
        validate_reference(&self.icon_definitions, "file", &self.file)?;
        validate_associations(
            &self.icon_definitions,
            "fileExtensions",
            &self.file_extensions,
        )?;
        validate_associations(&self.icon_definitions, "fileNames", &self.file_names)?;
        validate_associations(&self.icon_definitions, "languageIds", &self.language_ids)?;
        validate_reference(&self.icon_definitions, "light.file", &self.light.file)?;
        validate_associations(
            &self.icon_definitions,
            "light.fileExtensions",
            &self.light.file_extensions,
        )?;
        validate_associations(
            &self.icon_definitions,
            "light.fileNames",
            &self.light.file_names,
        )?;
        validate_associations(
            &self.icon_definitions,
            "light.languageIds",
            &self.light.language_ids,
        )
    }
}

/// Parse or semantic validation failure for a Seti manifest.
#[derive(Debug)]
pub enum SetiManifestError {
    InvalidJson(serde_json::Error),
    UnknownIconDefinition {
        association: String,
        icon_id: String,
    },
}

impl fmt::Display for SetiManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(formatter, "invalid Seti manifest JSON: {error}"),
            Self::UnknownIconDefinition {
                association,
                icon_id,
            } => write!(
                formatter,
                "Seti association {association} references unknown icon {icon_id}"
            ),
        }
    }
}

impl Error for SetiManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidJson(error) => Some(error),
            Self::UnknownIconDefinition { .. } => None,
        }
    }
}

/// Parses and validates one external Seti manifest document.
pub fn parse_seti_manifest(source: &str) -> Result<SetiFileIconManifest, SetiManifestError> {
    let manifest = serde_json::from_str(source).map_err(SetiManifestError::InvalidJson)?;
    SetiFileIconManifest::validate(&manifest)?;
    Ok(manifest)
}

/// Returns the checked-in Seti resource owned by this crate.
pub fn bundled_seti_manifest() -> &'static SetiFileIconManifest {
    static MANIFEST: OnceLock<SetiFileIconManifest> = OnceLock::new();
    MANIFEST.get_or_init(|| {
        parse_seti_manifest(include_str!("../seti/manifest.json"))
            .expect("bundled Seti manifest must be valid")
    })
}

fn validate_associations(
    definitions: &BTreeMap<String, SetiIconDefinition>,
    name: &str,
    associations: &BTreeMap<String, String>,
) -> Result<(), SetiManifestError> {
    for (key, icon_id) in associations {
        validate_reference(definitions, &format!("{name}.{key}"), icon_id)?;
    }
    Ok(())
}

fn validate_reference(
    definitions: &BTreeMap<String, SetiIconDefinition>,
    association: &str,
    icon_id: &str,
) -> Result<(), SetiManifestError> {
    if definitions.contains_key(icon_id) {
        return Ok(());
    }
    Err(SetiManifestError::UnknownIconDefinition {
        association: association.into(),
        icon_id: icon_id.into(),
    })
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
