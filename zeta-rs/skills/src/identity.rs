use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::Sha256;
use sha2::digest::Digest;
use std::fmt;

const MAX_SKILL_NAME_CHARS: usize = 64;
const MAX_SKILL_SOURCE_ID_BYTES: usize = 256;
const SHA256_PREFIX: &str = "sha256:";
const SHA256_HEX_BYTES: usize = 64;

/// Agent Skills-compatible name, unique within one Skill source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SkillName(String);

impl SkillName {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidSkillName> {
        let value = value.into();
        if value.is_empty() {
            return Err(InvalidSkillName::Empty);
        }
        if value.len() > MAX_SKILL_NAME_CHARS {
            return Err(InvalidSkillName::TooLong);
        }
        if value.starts_with('-')
            || value.ends_with('-')
            || value.contains("--")
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(InvalidSkillName::InvalidShape);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SkillName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for SkillName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SkillName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidSkillName {
    Empty,
    TooLong,
    InvalidShape,
}

impl fmt::Display for InvalidSkillName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("skill name must not be empty"),
            Self::TooLong => write!(
                formatter,
                "skill name exceeds the {MAX_SKILL_NAME_CHARS}-character limit"
            ),
            Self::InvalidShape => formatter.write_str(
                "skill name must contain lowercase ASCII letters, digits, and single hyphens",
            ),
        }
    }
}

impl std::error::Error for InvalidSkillName {}

/// Stable identity for one configured or built-in Skill source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SkillSourceId(String);

impl SkillSourceId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidSkillSourceId> {
        let value = value.into();
        if value.len() > MAX_SKILL_SOURCE_ID_BYTES {
            return Err(InvalidSkillSourceId::TooLong);
        }
        let Some((namespace, local_id)) = value.split_once(":skill-source:") else {
            return Err(InvalidSkillSourceId::InvalidShape);
        };
        if namespace.is_empty()
            || local_id.is_empty()
            || namespace.contains(char::is_whitespace)
            || local_id.contains(char::is_whitespace)
            || local_id.contains(':')
            || value.contains('\0')
        {
            return Err(InvalidSkillSourceId::InvalidShape);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SkillSourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for SkillSourceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SkillSourceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidSkillSourceId {
    TooLong,
    InvalidShape,
}

impl fmt::Display for InvalidSkillSourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong => write!(
                formatter,
                "skill source id exceeds the {MAX_SKILL_SOURCE_ID_BYTES}-byte limit"
            ),
            Self::InvalidShape => formatter.write_str(
                "skill source id must use '<namespace>:skill-source:<local-id>' with non-empty \
                 whitespace-free components",
            ),
        }
    }
}

impl std::error::Error for InvalidSkillSourceId {}

/// Source-qualified stable identity for one Skill.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillId {
    pub source: SkillSourceId,
    pub name: SkillName,
}

impl SkillId {
    pub fn new(source: SkillSourceId, name: SkillName) -> Self {
        Self { source, name }
    }
}

/// Exact SHA-256 identity of one `SKILL.md` byte sequence.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContentDigest(String);

impl ContentDigest {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidContentDigest> {
        let value = value.into();
        let Some(hex) = value.strip_prefix(SHA256_PREFIX) else {
            return Err(InvalidContentDigest);
        };
        if hex.len() != SHA256_HEX_BYTES
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(InvalidContentDigest);
        }
        Ok(Self(value))
    }

    pub(crate) fn from_hasher(hasher: Sha256) -> Self {
        Self(format!("{SHA256_PREFIX}{:x}", hasher.finalize()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for ContentDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ContentDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidContentDigest;

impl fmt::Display for InvalidContentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("content digest must use 'sha256:' followed by 64 lowercase hex digits")
    }
}

impl std::error::Error for InvalidContentDigest {}

/// Monotonic identity of one immutable, consumer-visible Skill catalog projection.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SkillCatalogGeneration(u64);

impl SkillCatalogGeneration {
    pub const INITIAL: Self = Self(1);

    pub fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Self {
        Self(
            self.0
                .checked_add(1)
                .expect("skill catalog generation exhausted"),
        )
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
