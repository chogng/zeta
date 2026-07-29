use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;

const MAX_PLUGIN_ID_BYTES: usize = 128;
const SHA256_PREFIX: &str = "sha256:";
const SHA256_HEX_BYTES: usize = 64;

/// Stable identity of one Plugin across versions.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PluginId(String);

impl PluginId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidPluginId> {
        let value = value.into();
        if value.len() > MAX_PLUGIN_ID_BYTES {
            return Err(InvalidPluginId::TooLong);
        }
        let Some((publisher, name)) = value.split_once('/') else {
            return Err(InvalidPluginId::InvalidShape);
        };
        if value.matches('/').count() != 1
            || !is_plugin_id_segment(publisher)
            || !is_plugin_id_segment(name)
        {
            return Err(InvalidPluginId::InvalidShape);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for PluginId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PluginId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Reason a Plugin ID was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidPluginId {
    TooLong,
    InvalidShape,
}

impl fmt::Display for InvalidPluginId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong => write!(
                formatter,
                "plugin id exceeds the {MAX_PLUGIN_ID_BYTES}-byte limit"
            ),
            Self::InvalidShape => formatter.write_str(
                "plugin id must use '<publisher>/<name>' with lowercase ASCII letters, digits, \
                 and single hyphens",
            ),
        }
    }
}

impl std::error::Error for InvalidPluginId {}

/// Exact SemVer release of one Plugin.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PluginVersion(semver::Version);

impl PluginVersion {
    pub fn new(value: impl AsRef<str>) -> Result<Self, InvalidPluginVersion> {
        let value = value.as_ref();
        if value.is_empty() {
            return Err(InvalidPluginVersion::Empty);
        }
        semver::Version::parse(value)
            .map(Self)
            .map_err(|_| InvalidPluginVersion::InvalidSemver)
    }

    pub fn as_semver(&self) -> &semver::Version {
        &self.0
    }
}

impl fmt::Display for PluginVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Serialize for PluginVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for PluginVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Reason a Plugin version was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidPluginVersion {
    Empty,
    InvalidSemver,
}

impl fmt::Display for InvalidPluginVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("plugin version must not be empty"),
            Self::InvalidSemver => {
                formatter.write_str("plugin version must be a valid semantic version")
            }
        }
    }
}

impl std::error::Error for InvalidPluginVersion {}

/// SHA-256 identity of one exact normalized Plugin package.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PluginPackageDigest(String);

impl PluginPackageDigest {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidPluginPackageDigest> {
        let value = value.into();
        let Some(hex) = value.strip_prefix(SHA256_PREFIX) else {
            return Err(InvalidPluginPackageDigest);
        };
        if hex.len() != SHA256_HEX_BYTES
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(InvalidPluginPackageDigest);
        }
        Ok(Self(value))
    }

    pub(crate) fn sha256(bytes: impl AsRef<[u8]>) -> Self {
        let digest = Sha256::digest(bytes.as_ref());
        Self(format!("{SHA256_PREFIX}{digest:x}"))
    }

    pub(crate) fn from_hasher(hasher: Sha256) -> Self {
        Self(format!("{SHA256_PREFIX}{:x}", hasher.finalize()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginPackageDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for PluginPackageDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PluginPackageDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// A package digest that was not a lowercase, self-describing SHA-256 value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidPluginPackageDigest;

impl fmt::Display for InvalidPluginPackageDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "plugin package digest must use 'sha256:' followed by 64 lowercase hex digits",
        )
    }
}

impl std::error::Error for InvalidPluginPackageDigest {}

/// Exact identity of an installed Plugin package.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstalledPluginRef {
    pub id: PluginId,
    pub version: PluginVersion,
    pub digest: PluginPackageDigest,
}

fn is_plugin_id_segment(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
