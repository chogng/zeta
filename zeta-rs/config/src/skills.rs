use crate::ConfigError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stable, namespaced identity for one Skill source declaration.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SkillSourceId(String);

impl SkillSourceId {
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        let Some((namespace, local_id)) = value.split_once(":skill-source:") else {
            return Err(ConfigError(
                "Skill source id must use '<namespace>:skill-source:<local-id>' form".into(),
            ));
        };
        if namespace.trim().is_empty()
            || local_id.trim().is_empty()
            || local_id.contains(':')
            || namespace.contains(char::is_whitespace)
            || local_id.contains(char::is_whitespace)
            || value.contains('\0')
        {
            return Err(ConfigError(
                "Skill source id must have a non-empty local identifier".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn belongs_to_namespace(&self, namespace: &str) -> bool {
        self.0
            .strip_prefix(namespace)
            .is_some_and(|suffix| suffix.starts_with(":skill-source:"))
    }
}

impl std::fmt::Display for SkillSourceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SkillSourceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Desired enablement for one configured Skill source.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkillSourceEnablement {
    #[default]
    Disabled,
    Enabled,
}

/// A runtime-free, user-owned Skill source declaration.
///
/// `root_reference` is an opaque host reference, not a trusted filesystem handle. A future Skill
/// manager resolves it, checks containment and trust, then publishes a separate catalog snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSourceConfig {
    pub id: SkillSourceId,
    pub root_reference: String,
    #[serde(default)]
    pub enablement: SkillSourceEnablement,
}

impl SkillSourceConfig {
    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        if self.root_reference.trim().is_empty()
            || self.root_reference.contains('\0')
            || self.root_reference.contains(['\n', '\r'])
        {
            return Err(ConfigError(
                "Skill source root reference must be non-empty plain text".into(),
            ));
        }
        Ok(())
    }
}

/// Skill source declarations owned by the user configuration authority.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsConfig {
    #[serde(default)]
    pub sources: BTreeMap<SkillSourceId, SkillSourceConfig>,
}

impl SkillsConfig {
    pub(crate) fn validate_for_namespace(&self, namespace: &str) -> Result<(), ConfigError> {
        for (source_id, source) in &self.sources {
            if &source.id != source_id {
                return Err(ConfigError(format!(
                    "Skill source entry '{}' contains declaration for '{}'",
                    source_id, source.id
                )));
            }
            if !source_id.belongs_to_namespace(namespace) {
                return Err(ConfigError(format!(
                    "Skill source '{}' is outside the '{namespace}' namespace",
                    source_id
                )));
            }
            source.validate()?;
        }
        Ok(())
    }
}
