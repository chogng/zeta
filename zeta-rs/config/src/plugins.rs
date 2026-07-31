use crate::ConfigError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_PLUGIN_ID_BYTES: usize = 128;

/// Stable Plugin package identity used by declarative configuration requests.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PluginId(String);

impl PluginId {
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        let Some((publisher, name)) = value.split_once('/') else {
            return Err(ConfigError(
                "plugin id must use '<publisher>/<name>' form".into(),
            ));
        };
        if value.len() > MAX_PLUGIN_ID_BYTES
            || value.matches('/').count() != 1
            || !is_plugin_segment(publisher)
            || !is_plugin_segment(name)
        {
            return Err(ConfigError(
                "plugin id must be at most 128 bytes with lowercase ASCII, digit, or single-hyphen segments"
                    .into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PluginId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Exact SemVer requested for one Plugin package.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PluginVersion(String);

impl PluginVersion {
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        semver::Version::parse(&value)
            .map_err(|_| ConfigError("plugin version must be an exact semantic version".into()))?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PluginVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PluginVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Desired participation of a requested Plugin in future activation resolution.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginRequestEnablement {
    #[default]
    Disabled,
    Enabled,
}

/// User request for one exact Plugin package.
///
/// This is desired configuration only. It does not install the package, grant capabilities,
/// bind credentials, or prove that activation succeeded.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginRequest {
    pub plugin_id: PluginId,
    pub version: PluginVersion,
    #[serde(default)]
    pub enablement: PluginRequestEnablement,
}

/// User Plugin requests keyed by stable package identity.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginsConfig {
    #[serde(default)]
    pub requests: BTreeMap<PluginId, PluginRequest>,
}

impl PluginsConfig {
    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        validate_request_keys(&self.requests, "User")
    }
}

/// Scope requested by a Workspace Plugin request.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspacePluginRequestScope {
    #[default]
    Workspace,
}

/// A non-authoritative Workspace request for an exact Plugin package.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePluginRequest {
    pub plugin_id: PluginId,
    pub version: PluginVersion,
    #[serde(default)]
    pub requested_scope: WorkspacePluginRequestScope,
}

/// Workspace Plugin requests keyed by stable package identity.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePluginRequests {
    #[serde(default)]
    pub requests: BTreeMap<PluginId, WorkspacePluginRequest>,
}

impl WorkspacePluginRequests {
    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        for (plugin_id, request) in &self.requests {
            if &request.plugin_id != plugin_id {
                return Err(ConfigError(format!(
                    "Workspace Plugin request '{}' contains request for '{}'",
                    plugin_id, request.plugin_id
                )));
            }
        }
        Ok(())
    }
}

fn validate_request_keys(
    requests: &BTreeMap<PluginId, PluginRequest>,
    scope: &str,
) -> Result<(), ConfigError> {
    for (plugin_id, request) in requests {
        if &request.plugin_id != plugin_id {
            return Err(ConfigError(format!(
                "{scope} Plugin request '{}' contains request for '{}'",
                plugin_id, request.plugin_id
            )));
        }
    }
    Ok(())
}

fn is_plugin_segment(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
